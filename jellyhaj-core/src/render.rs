use std::{
    cell::UnsafeCell,
    fmt::Debug,
    io::Write,
    pin::Pin,
    ptr::null,
    sync::{Arc, Weak},
    task::{
        Context,
        Poll::{self},
        ready,
    },
};

use color_eyre::Report;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use futures_intrusive::sync::ManualResetEvent;
use futures_util::future::BoxFuture;
use jellyfin::Result;
use jellyhaj_widgets_core::{
    ContextRef, JellyhajWidget, JellyhajWidgetExt, Position, Size, TreeVisitor, WidgetContext,
    WidgetTreeVisitor,
    async_task::{EventReceiver, IdWrapper, Stream, StreamExt, TaskSubmitter, new_task_pair},
};
use keybinds::KeybindEvents;
use parking_lot::{Mutex, RwLock};
use pin_project_lite::pin_project;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{Event, KeyEvent, MouseEvent},
};
use spawn::Spawner;
use tokio::{select, task::JoinHandle};
use tracing::instrument;

use crate::state::{Navigation, NextScreen};

#[derive(Debug)]
pub enum KeybindAction<A: Debug + Send + 'static> {
    Inner(A),
    Key(KeyEvent),
}

pub trait ErasedWidget<Res>:
    Stream<Item = Option<WidgetResult<Res>>> + Send + 'static + Unpin
{
    fn name(&self) -> &'static str;
    fn submit_event(&mut self, event: Event, size: Size) -> (Option<WidgetResult<Res>>, bool);
    fn render(&mut self, term: &mut DefaultTerminal) -> Result<Option<Report>>;
    fn visit(&self, visitor: &mut dyn TreeVisitor);
}

pin_project! {
    struct ErasedWidgetImpl<R: 'static, W: JellyhajWidget<R>> {
        widget: W,
        submitter: TaskSubmitter<W::Action, IdWrapper>,
        receiver: EventReceiver<W::Action>,
        context: R,
    }
}

pub type Erased = Box<dyn ErasedWidget<Navigation>>;

pub type WidgetCreator = Arc<dyn Fn(NextScreen) -> Erased + Send + Sync>;

impl<R: 'static, W: JellyhajWidget<R, ActionResult = Navigation>> ErasedWidgetImpl<R, W> {}

pub fn make_new_erased<
    R: ContextRef<Spawner> + Send + 'static,
    A: Debug + Send + 'static,
    W: JellyhajWidget<R, Action = KeybindAction<A>>,
>(
    cx: R,
    mut widget: W,
) -> Box<dyn ErasedWidget<W::ActionResult>> {
    let (submitter, receiver) = new_task_pair(cx.as_ref().clone());
    widget.init(WidgetContext {
        refs: &cx,
        submitter: submitter.as_ref(),
    });
    let state = ErasedWidgetImpl {
        widget,
        context: cx,
        submitter,
        receiver,
    };
    Box::new(state)
}

pub enum WidgetResult<T> {
    Ok(T),
    Err(Report),
    Pop,
    Exit,
}

impl From<WidgetResult<Navigation>> for Navigation {
    fn from(value: WidgetResult<Navigation>) -> Self {
        match value {
            WidgetResult::Ok(v) => v,
            WidgetResult::Err(report) => Navigation::Replace(NextScreen::Error(report)),
            WidgetResult::Pop => Navigation::PopContext,
            WidgetResult::Exit => Navigation::Exit,
        }
    }
}

impl<R: 'static, W: JellyhajWidget<R>> Stream for ErasedWidgetImpl<R, W> {
    type Item = Option<WidgetResult<W::ActionResult>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        Poll::Ready(Some(match ready!(this.receiver.poll_next_unpin(cx)) {
            Some(Ok(action)) => match this.widget.apply_action(
                WidgetContext {
                    refs: this.context,
                    submitter: this.submitter.as_ref(),
                },
                action,
            ) {
                Ok(Some(n)) => Some(WidgetResult::Ok(n)),
                Ok(None) => None,
                Err(e) => Some(WidgetResult::Err(e)),
            },
            Some(Err(e)) => Some(WidgetResult::Err(e)),
            None => Some(WidgetResult::Pop),
        }))
    }
}

impl<R: Send + 'static, A: Debug + Send + 'static, W: JellyhajWidget<R, Action = KeybindAction<A>>>
    ErasedWidget<W::ActionResult> for ErasedWidgetImpl<R, W>
{
    fn name(&self) -> &'static str {
        W::NAME
    }

    fn visit(&self, mut visitor: &mut dyn TreeVisitor) {
        visitor.visit(&self.widget);
    }

    fn submit_event(
        &mut self,
        event: Event,
        frame_size: Size,
    ) -> (Option<WidgetResult<W::ActionResult>>, bool) {
        let res = match event {
            Event::Key(key) => self.widget.apply_action(
                WidgetContext {
                    refs: &self.context,
                    submitter: self.submitter.as_ref(),
                },
                KeybindAction::Key(key),
            ),
            Event::Mouse(MouseEvent {
                kind,
                column,
                row,
                modifiers,
            }) => self.widget.click(
                WidgetContext {
                    refs: &self.context,
                    submitter: self.submitter.as_ref(),
                },
                Position::new(column, row),
                frame_size,
                kind,
                modifiers,
            ),
            Event::Paste(v) => {
                if self.widget.accepts_text_input() {
                    self.widget.accept_text(v);
                    return (None, true);
                } else {
                    return (None, false);
                }
            }
            Event::Resize(_, _) => return (None, true),
            _ => return (None, true),
        };
        let res = match res {
            Ok(None) => None,
            Ok(Some(v)) => Some(WidgetResult::Ok(v)),
            Err(e) => Some(WidgetResult::Err(e)),
        };
        (res, true)
    }

    fn render(&mut self, term: &mut DefaultTerminal) -> Result<Option<Report>> {
        term.autoresize()?;
        let mut frame = term.get_frame();
        let res = self.widget.render_fallible(
            frame.area(),
            frame.buffer_mut(),
            WidgetContext {
                refs: &self.context,
                submitter: self.submitter.as_ref(),
            },
        );
        if res.is_err() {
            frame.buffer_mut().reset();
        } else {
            term.flush()?;
            term.hide_cursor()?;
            term.swap_buffers();
            term.backend_mut().flush()?;
        }
        Ok(res.err())
    }
}

unsafe fn remove_element(entry: &StateEntry, token: &mut ListAccessToken) {
    tracing::trace!("removing elelment");
    if let (Some(next), Some(prev)) = {
        let entry = unsafe { entry.get_list_mut(token) };
        (
            entry.next.take(),
            entry.prev.take().as_ref().and_then(Weak::upgrade),
        )
    } {
        unsafe { next.get_list_mut(token) }.prev = Some(Arc::downgrade(&prev));
        unsafe { prev.get_list_mut(token) }.next = Some(next);
    }
}

unsafe fn replace_element(
    entry: Arc<StateEntry>,
    new_entry: Arc<StateEntry>,
    token: &mut ListAccessToken,
) {
    tracing::trace!("replacing element");
    if let (Some(next), Some(prev)) = {
        let entry = unsafe { entry.get_list_mut(token) };
        (
            entry.next.take(),
            entry.prev.take().as_ref().and_then(Weak::upgrade),
        )
    } {
        unsafe { next.get_list_mut(token) }.prev = Some(Arc::downgrade(&new_entry));
        let new = unsafe { new_entry.get_list_mut(token) };
        new.next = Some(next);
        new.prev = Some(Arc::downgrade(&prev));
        unsafe { prev.get_list_mut(token) }.next = Some(new_entry);
    }
}

unsafe fn append_element(
    entry: Arc<StateEntry>,
    new_entry: Arc<StateEntry>,
    token: &mut ListAccessToken,
) {
    tracing::trace!("appending element");
    if let Some(next) = { unsafe { entry.get_list_mut(token) }.next.take() } {
        unsafe { next.get_list_mut(token) }.prev = Some(Arc::downgrade(&new_entry));
        let new = unsafe { new_entry.get_list_mut(token) };
        new.next = Some(next);
        new.prev = Some(Arc::downgrade(&entry));
        unsafe { entry.get_list_mut(token) }.next = Some(new_entry);
    }
}

unsafe fn prepend_element(
    entry: Arc<StateEntry>,
    new_entry: Arc<StateEntry>,
    token: &mut ListAccessToken,
) {
    tracing::trace!("prepending element");
    if let Some(prev) = {
        unsafe { entry.get_list_mut(token) }
            .prev
            .take()
            .and_then(|p| p.upgrade())
    } {
        unsafe { entry.get_list_mut(token) }.prev = Some(Arc::downgrade(&new_entry));
        let new = unsafe { new_entry.get_list_mut(token) };
        new.next = Some(entry);
        new.prev = Some(Arc::downgrade(&prev));
        unsafe { prev.get_list_mut(token) }.next = Some(new_entry);
    }
}

unsafe fn inspect_list(start: &Arc<StateEntry>, token: &ListAccessToken) {
    let span = tracing::trace_span!("inspect_list");
    let _entered = span.enter();
    let mut next_entry = Some(start);
    if !span.is_disabled() {
        while let Some(cur) = next_entry {
            let kind = match &cur.value {
                StateValue::Suspended(_) => "Suspended",
                StateValue::Empty => "Empty",
                StateValue::WithoutTui(_) => "WithoutTui",
            };
            let entry = unsafe { cur.get_list(token) };
            let prev = entry.prev.as_ref().map(Weak::as_ptr).unwrap_or(null());
            let next = entry.next.as_ref().map(Arc::as_ptr).unwrap_or(null());
            tracing::trace!(name: "list-entry", kind = kind, prev = ?prev, next = ?next);
            next_entry = entry.next.as_ref();
            if let Some(e) = next_entry
                && Arc::ptr_eq(e, start)
            {
                break;
            }
        }
    }
}

pub enum RunResult {
    Cont(Erased),
    Empty,
    Exit,
}

#[instrument(skip_all)]
async unsafe fn run_suspended(
    mut state: Erased,
    stop: Arc<ManualResetEvent>,
    mut visitors: UnboundedReceiver<Visitor>,
    widget_creator: WidgetCreator,
    state_entry: Weak<StateEntry>,
    state_token: Arc<RwLock<ListAccessToken>>,
) -> RunResult {
    loop {
        select! {
            nav = state.next() => {
                let nav = match nav{
                    Some(Some(v)) => Some(v),
                    Some(None) => continue,
                    None => None
                };
                match nav.map(Navigation::from) {
                    Some(Navigation::PopContext) => {
                        let mut token = state_token.write();
                        if let Some(entry) = state_entry.upgrade() {
                            unsafe {remove_element(&entry, &mut token)};
                        }
                        return RunResult::Empty;
                    }
                    Some(Navigation::Exit) => {
                        return RunResult::Exit;
                    }
                    Some(Navigation::Replace(next)) => {
                        let mut token = state_token.write();
                        if let Some(entry) = state_entry.upgrade() {
                            unsafe{
                                let new = Arc::new_cyclic(|this|StateEntry::new(StateValue::Suspended(SuspendedInner::new(
                                        widget_creator(next),
                                        this.clone(),
                                        widget_creator.clone(),
                                        state_token.clone()
                                    ))));
                                replace_element(
                                    entry,
                                    new.clone(),
                                    &mut token,
                                );
                                inspect_list(&new, &token);
                            }
                        }
                        return RunResult::Empty;
                    }
                    Some(Navigation::Push(next)) => {
                        let mut token = state_token.write();
                        if let Some(entry) = state_entry.upgrade() {
                            unsafe{
                                let new = Arc::new_cyclic(|this|StateEntry::new(StateValue::Suspended(SuspendedInner::new(
                                        widget_creator(next),
                                        this.clone(),
                                        widget_creator.clone(),
                                        state_token.clone()
                                    ))));
                                append_element(
                                    entry,
                                    new.clone(),
                                    &mut token,
                                );
                                inspect_list(&new, &token);
                            }
                        }
                    }
                    Some(Navigation::PushWithoutTui(next)) => {
                        let mut token = state_token.write();
                        if let Some(entry) = state_entry.upgrade(){
                            unsafe{
                                let new = Arc::new(StateEntry::new(StateValue::WithoutTui(next)));
                                append_element(entry,new.clone() , &mut token);
                                inspect_list(&new, &token);
                            }
                        }
                    }
                    None => return RunResult::Exit
                }
            }
            _ = stop.wait() => {
                return RunResult::Cont(state)
            }
            visitor = visitors.next() => {
                if let Some(visitor) = visitor{
                    visitor(&|visitor|state.visit(visitor))
                }else {
                    return RunResult::Cont(state)
                }
            }
        };
    }
}

struct DropGuard {
    inner: Arc<ManualResetEvent>,
}

impl Drop for DropGuard {
    fn drop(&mut self) {
        self.inner.set();
    }
}

type Visitor = Box<dyn Fn(&dyn Fn(&mut dyn TreeVisitor)) + Send + Sync>;

pub struct SuspendedInner {
    task: Mutex<Option<JoinHandle<RunResult>>>,
    drop_guard: DropGuard,
    pub name: &'static str,
    pub send_visitor: UnboundedSender<Visitor>,
}

impl SuspendedInner {
    pub async fn get_widget(&self) -> RunResult {
        self.drop_guard.inner.set();
        let handle = self.task.lock().take().expect("tried to get task twice");
        handle.await.expect("polling state paniced")
    }

    unsafe fn new(
        widget: Erased,
        this: Weak<StateEntry>,
        widget_creator: WidgetCreator,
        state_token: Arc<RwLock<ListAccessToken>>,
    ) -> Self {
        let stop = Arc::new(ManualResetEvent::new(false));
        let (visitor_send, visitor_recv) = unbounded();
        let name = widget.name();
        let fut = unsafe {
            run_suspended(
                widget,
                stop.clone(),
                visitor_recv,
                widget_creator,
                this,
                state_token,
            )
        };
        let task = {
            #[cfg(tokio_unstable)]
            {
                tokio::task::Builder::new()
                    .name(W::State::NAME)
                    .spawn(fut)
                    .expect("spawning future should not fail")
            }
            #[cfg(not(tokio_unstable))]
            {
                tokio::task::spawn(fut)
            }
        };
        Self {
            task: Mutex::new(Some(task)),
            drop_guard: DropGuard { inner: stop },
            name,
            send_visitor: visitor_send,
        }
    }
}

struct ListAccessToken {
    _evil: (),
}

struct ListEntry {
    next: Option<Arc<StateEntry>>,
    prev: Option<Weak<StateEntry>>,
}

pub enum StateValue {
    Suspended(SuspendedInner),
    Empty,
    WithoutTui(BoxFuture<'static, Result<()>>),
}

struct StateEntry {
    list: UnsafeCell<ListEntry>,
    pub value: StateValue,
}

unsafe impl Sync for StateEntry {}
unsafe impl Send for StateEntry {}

impl StateEntry {
    /// # Safety
    /// the ListAccessToken must be from this list
    unsafe fn get_list<'a>(&'a self, _token: &'a ListAccessToken) -> &'a ListEntry {
        unsafe { &*self.list.get() }
    }
    /// # Safety
    /// the ListAccessToken must be from this list
    unsafe fn get_list_mut<'a>(&'a self, _token: &'a mut ListAccessToken) -> &'a mut ListEntry {
        unsafe { &mut *self.list.get() }
    }

    fn new(val: StateValue) -> Self {
        Self {
            list: UnsafeCell::new(ListEntry {
                next: None,
                prev: None,
            }),
            value: val,
        }
    }
}

pub struct StateStack {
    lock: Arc<RwLock<ListAccessToken>>,
    list: Arc<StateEntry>,
}

impl Drop for StateStack {
    // break reference cycles, isolate all entries
    fn drop(&mut self) {
        let mut guard = self.lock.write();
        let mut entry = self.list.clone();
        while let Some(new_entry) = {
            let entry = unsafe { entry.get_list_mut(&mut guard) };
            entry.prev = None;
            entry.next.take()
        } {
            entry = new_entry;
        }
    }
}

impl StateStack {
    pub fn new() -> Self {
        let list = Arc::new(StateEntry {
            list: UnsafeCell::new(ListEntry {
                next: None,
                prev: None,
            }),
            value: StateValue::Empty,
        });
        //initialize, this is the ownly copy so this is save
        unsafe {
            let entry = &mut *list.list.get();
            entry.next = Some(list.clone());
            entry.prev = Some(Arc::downgrade(&list))
        }

        StateStack {
            lock: Arc::new(RwLock::new(ListAccessToken { _evil: () })),
            list,
        }
    }
    pub fn push(&self, widget: Erased, widget_creator: WidgetCreator) {
        let mut token = self.lock.write();
        unsafe {
            prepend_element(
                self.list.clone(),
                Arc::new_cyclic(|this| {
                    StateEntry::new(StateValue::Suspended(SuspendedInner::new(
                        widget,
                        this.clone(),
                        widget_creator,
                        self.lock.clone(),
                    )))
                }),
                &mut token,
            );
            inspect_list(&self.list, &token);
        }
    }
    pub fn pop(&self) -> StateValue {
        let mut token = self.lock.write();
        let entry = unsafe { self.list.get_list(&token) }
            .prev
            .as_ref()
            .expect("previous should be set while the list is live")
            .upgrade()
            .expect("previous should not be dropped");
        if Arc::ptr_eq(&self.list, &entry) {
            StateValue::Empty
        } else {
            unsafe {
                remove_element(&entry, &mut token);
            }
            unsafe { inspect_list(&self.list, &token) };
            Arc::into_inner(entry)
                .expect("should not currently be owned")
                .value
        }
    }

    pub fn visit(&self, mut visitor: impl FnMut(&StateValue)) {
        let token = self.lock.read();
        let head = &self.list;
        let mut current = head;
        loop {
            current = unsafe { current.get_list(&token) }
                .next
                .as_ref()
                .expect("next should be set while the list is live");
            if Arc::ptr_eq(current, head) {
                break;
            }
            visitor(&current.value);
        }
    }
}

impl Default for StateStack {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn render_widget<R: 'static>(
    widget: &mut dyn ErasedWidget<R>,
    events: &mut KeybindEvents,
    term: &mut DefaultTerminal,
) -> WidgetResult<R> {
    let mut render = true;
    loop {
        if render {
            match widget.render(term) {
                Err(e) => {
                    tracing::error!("failed to draw to the terminal:\n{e:?}");
                    return WidgetResult::Exit;
                }
                Ok(None) => {}
                Ok(Some(e)) => return WidgetResult::Err(e),
            }
        }

        select! {
            nav = widget.next() => {
                match nav{
                    Some(Some(nav)) => return nav,
                    Some(None) => {render = true;}
                    None => return WidgetResult::Exit,
                }
            }
            event = events.next() => {
                match event{
                    None => return WidgetResult::Exit,
                    Some(Err(e)) => {
                        tracing::error!("reading keyboard event failed:\n{e:?}");
                        return WidgetResult::Exit;
                    }
                    Some(Ok(event)) =>{
                        let (res, r) = widget.submit_event(event, term.get_frame().area().as_size());
                        render=r;
                        if let Some(nav) = res{
                            return nav
                        }
                    }
                }
            }
        }
    }
}
