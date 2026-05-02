mod erased;

use std::{
    cell::UnsafeCell,
    cmp::max,
    fmt::Debug,
    ops::{Deref, DerefMut},
    ptr::null,
    sync::{Arc, Weak},
    time::Duration,
};

use config::{Config, effects::EffectInfo};
pub use erased::*;

use color_eyre::Report;
use futures_intrusive::sync::ManualResetEvent;
use futures_util::future::BoxFuture;
use jellyfin::Result;
use jellyhaj_widgets_core::{
    ContextRef, GetFromContext, JellyhajWidget, TreeVisitor, async_task::StreamExt,
};
use keybinds::KeybindEvents;
use parking_lot::{Mutex, RwLock};
use ratatui::{
    DefaultTerminal, buffer::Buffer, crossterm::event::KeyEvent, layout::Rect, prelude::Backend,
};
use spawn::Spawner;
use tokio::{
    select,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
    time::{Instant, sleep_until},
};
use tracing::instrument;

use crate::state::{Navigation, NextScreen};

#[derive(Debug)]
pub enum KeybindAction<A: Debug + Send + 'static> {
    Inner(A),
    Key(KeyEvent),
}

pub enum WidgetResult<T> {
    Ok(T),
    Err(Report),
    Pop,
    Exit,
}

impl From<WidgetResult<Self>> for Navigation {
    fn from(value: WidgetResult<Self>) -> Self {
        match value {
            WidgetResult::Ok(v) => v,
            WidgetResult::Err(report) => Self::Replace(NextScreen::Error(report)),
            WidgetResult::Pop => Self::PopContext,
            WidgetResult::Exit => Self::Exit,
        }
    }
}

#[instrument(skip_all, level = "trace")]
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
        unsafe { inspect_list(&prev, token) };
    }
}

#[instrument(skip_all, level = "trace")]
unsafe fn replace_element(
    entry: &Arc<StateEntry>,
    new_entry: &Arc<StateEntry>,
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
        unsafe { next.get_list_mut(token) }.prev = Some(Arc::downgrade(new_entry));
        let new = unsafe { new_entry.get_list_mut(token) };
        new.next = Some(next);
        new.prev = Some(Arc::downgrade(&prev));
        unsafe { prev.get_list_mut(token) }.next = Some(new_entry.clone());
    }
    unsafe { inspect_list(new_entry, token) };
}

#[instrument(skip_all, level = "trace")]
unsafe fn append_element(
    entry: &Arc<StateEntry>,
    new_entry: Arc<StateEntry>,
    token: &mut ListAccessToken,
) {
    tracing::trace!("appending element");
    if let Some(next) = { unsafe { entry.get_list_mut(token) }.next.take() } {
        unsafe { next.get_list_mut(token) }.prev = Some(Arc::downgrade(&new_entry));
        let new = unsafe { new_entry.get_list_mut(token) };
        new.next = Some(next);
        new.prev = Some(Arc::downgrade(entry));
        unsafe { entry.get_list_mut(token) }.next = Some(new_entry);
    }
    unsafe { inspect_list(entry, token) };
}

#[instrument(skip_all, level = "trace")]
unsafe fn prepend_element(
    entry: &Arc<StateEntry>,
    new_entry: Arc<StateEntry>,
    token: &mut ListAccessToken,
) {
    unsafe { inspect_list(entry, token) };
    tracing::trace!("prepending element");
    if let Some(prev) = {
        unsafe { entry.get_list_mut(token) }
            .prev
            .take()
            .and_then(|p| p.upgrade())
    } {
        unsafe { entry.get_list_mut(token) }.prev = Some(Arc::downgrade(&new_entry));
        let new = unsafe { new_entry.get_list_mut(token) };
        new.next = Some(entry.clone());
        new.prev = Some(Arc::downgrade(&prev));
        unsafe { prev.get_list_mut(token) }.next = Some(new_entry);
    }
    unsafe { inspect_list(entry, token) };
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
            let prev = entry.prev.as_ref().map_or(null(), Weak::as_ptr);
            let next = entry.next.as_ref().map_or(null(), Arc::as_ptr);
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
            nav = state.next_filtered_event() => {
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
                                    &entry,
                                    &new,
                                    &mut token,
                                );
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
                                    &entry,
                                    new.clone(),
                                    &mut token,
                                );
                            }
                        }
                    }
                    Some(Navigation::PushWithoutTui(next)) => {
                        let mut token = state_token.write();
                        if let Some(entry) = state_entry.upgrade(){
                            unsafe{
                                let new = Arc::new(StateEntry::new(StateValue::WithoutTui(next)));
                                append_element(&entry,new.clone() , &mut token);
                            }
                        }
                    }
                    None => return RunResult::Exit
                }
            }
            () = stop.wait() => {
                return RunResult::Cont(state)
            }
            visitor = visitors.recv() => {
                if let Some(visitor) = visitor{
                    visitor(&|visitor|state.visit(visitor));
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

type Visitor = Box<dyn FnOnce(&dyn Fn(&mut dyn TreeVisitor)) + Send + Sync>;

pub type Erased = Box<ShadedWidget<Navigation>>;

pub type WidgetCreator = Arc<dyn Fn(NextScreen) -> Erased + Send + Sync>;

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
        let (visitor_send, visitor_recv) = tokio::sync::mpsc::unbounded_channel();
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
    /// the `ListAccessToken` must be from this list
    unsafe fn get_list<'a>(&'a self, _token: &'a ListAccessToken) -> &'a ListEntry {
        unsafe { &*self.list.get() }
    }
    /// # Safety
    /// the `ListAccessToken` must be from this list
    unsafe fn get_list_mut<'a>(&'a self, _token: &'a mut ListAccessToken) -> &'a mut ListEntry {
        unsafe { &mut *self.list.get() }
    }

    const fn new(val: StateValue) -> Self {
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
    #[instrument(skip_all, level = "trace", name = "StateStack::drop()")]
    fn drop(&mut self) {
        let mut guard = self.lock.write();
        let mut entry = self.list.clone();
        unsafe { inspect_list(&entry, &guard) };
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
    #[instrument(skip_all, level = "trace", name = "StateStack::new()")]
    pub fn new() -> Self {
        tracing::trace!("new state stack");
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
            entry.prev = Some(Arc::downgrade(&list));
        }

        let token = ListAccessToken { _evil: () };
        unsafe { inspect_list(&list, &token) };
        Self {
            lock: Arc::new(RwLock::new(token)),
            list,
        }
    }
    pub fn push(&self, widget: Erased, widget_creator: WidgetCreator) {
        let mut token = self.lock.write();
        unsafe {
            prepend_element(
                &self.list,
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
        }
    }
    #[must_use]
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

    #[instrument(skip_all, level = "trace")]
    pub fn visit(&self, mut visitor: impl FnMut(&StateValue)) {
        let token = self.lock.read();
        tracing::trace!("visiting states");
        unsafe { inspect_list(&self.list, &token) };
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

#[must_use]
pub struct ShadedWidgetGen<W: ?Sized> {
    last: Instant,
    start: Option<EffectInfo>,
    main: Option<EffectInfo>,
    exit: Option<EffectInfo>,
    widget: W,
}

pub type ShadedWidget<Res> = ShadedWidgetGen<dyn ErasedWidget<Res>>;

pub fn make_new_erased<
    R: ContextRef<Spawner> + ContextRef<Config> + Send + 'static,
    A: Debug + Send + 'static,
    W: JellyhajWidget<R, Action = KeybindAction<A>>,
>(
    cx: R,
    widget: W,
) -> Box<ShadedWidget<W::ActionResult>> {
    ShadedWidget::new(cx, widget)
}

impl<Res: 'static> ShadedWidget<Res> {
    fn new_sized<
        R: ContextRef<Spawner> + ContextRef<Config> + Send + 'static,
        A: Debug + Send + 'static,
        W: JellyhajWidget<R, Action = KeybindAction<A>, ActionResult = Res>,
    >(
        cx: R,
        widget: W,
    ) -> ShadedWidgetGen<impl ErasedWidget<Res>> {
        let effects = &Config::get_ref(&cx).effects;
        let start = effects.start(W::NAME);
        let main = effects.main(W::NAME);
        let exit = effects.exit(W::NAME);
        let widget = erased::make_new_erased(cx, widget);
        ShadedWidgetGen {
            last: Instant::now(),
            start,
            main,
            exit,
            widget,
        }
    }
    pub fn new<
        R: ContextRef<Spawner> + ContextRef<Config> + Send + 'static,
        A: Debug + Send + 'static,
        W: JellyhajWidget<R, Action = KeybindAction<A>, ActionResult = Res>,
    >(
        cx: R,
        widget: W,
    ) -> Box<Self> {
        Box::new(Self::new_sized(cx, widget))
    }

    fn is_stopped_finished(&self) -> bool {
        self.exit.is_none()
    }

    fn render_shaded(&mut self, area: Rect, buf: &mut Buffer) -> Result<u8> {
        self.render(area, buf)?;
        let now = Instant::now();
        let time = now - self.last;
        self.last = now;
        let mut fps = 0u8;
        if let Some(main) = self.main.as_mut() {
            main.effect.process(time, buf, area);
            if main.effect.done() {
                self.main = None;
            } else {
                fps = main.fps;
            }
        }
        if let Some(start) = self.start.as_mut() {
            start.effect.process(time, buf, area);
            if start.effect.done() {
                self.start = None;
            } else {
                fps = max(fps, start.fps);
            }
        }
        Ok(fps)
    }
    fn start_render_stop(&mut self, area: Rect, buf: &mut Buffer) -> Result<u8> {
        self.render(area, buf)?;
        let now = Instant::now();
        let time = now - self.last;
        self.last = now;
        let mut fps = 0u8;
        if let Some(main) = self.main.as_mut() {
            main.effect.process(time, buf, area);
            if main.effect.done() {
                self.main = None;
            } else {
                fps = main.fps;
            }
        }
        if let Some(start) = self.start.as_mut() {
            start.effect.process(time, buf, area);
            if start.effect.done() {
                self.start = None;
            } else {
                fps = max(fps, start.fps);
            }
        }
        if let Some(exit) = self.exit.as_mut() {
            exit.effect.process(Duration::ZERO, buf, area);
            if exit.effect.done() {
                self.exit = None;
            } else {
                fps = max(fps, exit.fps);
            }
        }
        Ok(fps)
    }
    fn render_stop(&mut self, area: Rect, buf: &mut Buffer) -> Result<u8> {
        self.render(area, buf)?;
        let now = Instant::now();
        let time = now - self.last;
        self.last = now;
        let mut fps = 0u8;
        if let Some(main) = self.main.as_mut() {
            main.effect.process(time, buf, area);
            if main.effect.done() {
                self.main = None;
            } else {
                fps = main.fps;
            }
        }
        if let Some(start) = self.start.as_mut() {
            start.effect.process(time, buf, area);
            if start.effect.done() {
                self.start = None;
            } else {
                fps = max(fps, start.fps);
            }
        }
        if let Some(exit) = self.exit.as_mut() {
            exit.effect.process(time, buf, area);
            if exit.effect.done() {
                self.exit = None;
            } else {
                fps = max(fps, exit.fps);
            }
        }
        Ok(fps)
    }
}

impl<Res> DerefMut for ShadedWidget<Res> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<Res> Deref for ShadedWidget<Res> {
    type Target = dyn ErasedWidget<Res>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

fn render_to_term<T>(
    term: &mut DefaultTerminal,
    f: impl FnOnce(Rect, &mut Buffer) -> Result<T>,
) -> Result<Result<T>> {
    term.autoresize()?;
    let mut frame = term.get_frame();
    let res = f(frame.area(), frame.buffer_mut());
    if res.is_err() {
        frame.buffer_mut().reset();
    } else {
        term.flush()?;
        term.hide_cursor()?;
        term.swap_buffers();
        term.backend_mut().flush()?;
    }
    Ok(res)
}

pub async fn render_widget<Res: 'static>(
    widget: &mut ShadedWidget<Res>,
    events: &mut KeybindEvents,
    term: &mut DefaultTerminal,
) -> WidgetResult<Res> {
    let mut render = true;
    let mut duration = Duration::ZERO;
    let mut last_render = Instant::now();
    loop {
        if render {
            last_render = Instant::now();
            match render_to_term(term, |area, buf| widget.render_shaded(area, buf)) {
                Err(e) => {
                    tracing::error!("failed to draw to the terminal:\n{e:?}");
                    return WidgetResult::Exit;
                }
                Ok(Ok(fps)) => {
                    duration = if fps > 0 {
                        Duration::from_secs(1) / u32::from(fps)
                    } else {
                        Duration::ZERO
                    }
                }
                Ok(Err(e)) => return WidgetResult::Err(e),
            }
        }
        select! {
            () = sleep_until(last_render+duration), if !duration.is_zero() => {
                render = true;
            }
            nav = widget.next() => {
                match nav{
                    Some(Some(WidgetResult::Exit))|
                    None => return WidgetResult::Exit,
                    Some(Some(nav)) => return nav,
                    Some(None) => {render = true;}
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderStopRes {
    Ok,
    Exit,
}

pub async fn render_widget_stop<Res: 'static>(
    widget: &mut ShadedWidget<Res>,
    events: &mut KeybindEvents,
    term: &mut DefaultTerminal,
) -> RenderStopRes {
    let mut render;
    let mut last_render = Instant::now();
    let mut duration;
    match render_to_term(term, |area, buf| widget.start_render_stop(area, buf)) {
        Err(e) => {
            tracing::error!("failed to draw to the terminal:\n{e:?}");
            return RenderStopRes::Exit;
        }
        Ok(Ok(fps)) => {
            duration = if fps > 0 {
                Duration::from_secs(1) / u32::from(fps)
            } else {
                Duration::ZERO
            };
        }
        Ok(Err(e)) => {
            tracing::error!("Error rendering stop animation:\n{e:?}");
            return RenderStopRes::Ok;
        }
    }

    loop {
        if widget.is_stopped_finished() {
            break RenderStopRes::Ok;
        }
        assert!(
            !duration.is_zero(),
            "Stop effect with fps 0 may never complete!"
        );
        select! {
            () = sleep_until(last_render + duration), if ! duration.is_zero() => {
                render = true;
            }
            nav = widget.next() => {
                match nav{
                    Some(Some(_) | None) => {render = true;}
                    None => return RenderStopRes::Exit,
                }
            }
            event = events.next() => {
                match event{
                    None => return RenderStopRes::Exit,
                    Some(Err(e)) => {
                        tracing::error!("reading keyboard event failed:\n{e:?}");
                        return RenderStopRes::Exit;
                    }
                    Some(Ok(event)) =>{
                        let (_, r) = widget.submit_event(event, term.get_frame().area().as_size());
                        render=r;
                    }
                }
            }
        }
        if render {
            last_render = Instant::now();
            match render_to_term(term, |area, buf| widget.render_stop(area, buf)) {
                Err(e) => {
                    tracing::error!("failed to draw to the terminal:\n{e:?}");
                    return RenderStopRes::Exit;
                }
                Ok(Ok(fps)) => {
                    duration = if fps > 0 {
                        Duration::from_secs(1) / u32::from(fps)
                    } else {
                        Duration::ZERO
                    }
                }
                Ok(Err(e)) => {
                    tracing::error!("Error rendering stop animation:\n{e:?}");
                    return RenderStopRes::Ok;
                }
            }
        }
    }
}
