use std::{
    cell::UnsafeCell,
    convert::Infallible,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, ready},
};

use futures_util::{FutureExt, future::BoxFuture};
use jellyhaj_widgets_core::{
    Result,
    async_task::{UnboundedReceiver, UnboundedSender},
};
use keybinds::KeybindEvents;
use parking_lot::RwLock;
use pin_project_lite::pin_project;
use ratatui::DefaultTerminal;
use tokio::task::{JoinHandle, coop::poll_proceed};
use tracing::{debug, info, instrument, warn};

use crate::{
    state::{Navigation, NextScreen},
    term::{RunWithout, run_without},
    widgets::{
        RunResult, WidgetCreator,
        list::{
            ListAccessToken, ListEntry, StateEntry, inspect_list, prepend_element, remove_element,
        },
        shaded::{
            render::{
                RenderStopRes, RenderStopWidget, RenderWidget, render_widget, render_widget_stop,
            },
            widget::{Erased, ShadedWidget},
        },
        suspended::SuspendedInner,
    },
};

pub enum StateValue {
    Suspended(SuspendedInner),
    Empty,
    WithoutTui(BoxFuture<'static, Result<()>>),
}

pub struct StateStack {
    lock: Arc<RwLock<ListAccessToken>>,
    list: Arc<StateEntry>,
}

pub struct StateStackHandle {
    inner: Arc<StateStack>,
}

impl Deref for StateStackHandle {
    type Target = Arc<StateStack>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Drop for StateStackHandle {
    // break reference cycles, isolate all entries
    #[instrument(skip_all, level = "trace", name = "StateStack::drop()")]
    fn drop(&mut self) {
        debug!("dropping state stack");
        self.inner.destroy();
    }
}

impl StateStackHandle {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(StateStack::new()),
        }
    }
}

impl Default for StateStackHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl StateStack {
    #[instrument(skip_all, level = "trace", name = "StateStack::new()")]
    fn new() -> Self {
        debug!("new state stack");
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

    #[instrument(skip_all, level = "trace")]
    fn destroy(&self) {
        debug!("destroying state stack");
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

pin_project! {
    pub struct RenderLoop<'l> {
        widget_creator: WidgetCreator,
        state: &'l StateStack,
        term: &'l mut DefaultTerminal,
        events: &'l mut KeybindEvents,
        external: &'l mut UnboundedReceiver<NextScreen>,
        external_closed_detected: bool,
        #[pin]
        loop_state: RenderLoopState,
    }
}

pin_project! {
    #[project = RenderLoopStateProj]
    pub enum RenderLoopState {
        WithoutTui{fut: RunWithout},
        Render{ #[pin] state: RenderWidget, widget: Option<Box<ShadedWidget<Navigation>>>},
        RenderStop{#[pin] state: RenderStopWidget, widget: Box<ShadedWidget<Navigation>>, next: Option<NextScreen>},
        Suspended{handle: JoinHandle<RunResult>}

    }
}

impl Future for RenderLoop<'_> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        loop {
            let new_state = 'new_state: {
                'pop_state: {
                    let _: Infallible = match this.loop_state.as_mut().project() {
                        RenderLoopStateProj::WithoutTui { fut } => {
                            if let Err(e) = ready!(fut.poll_unpin(cx)) {
                                break 'new_state make_render((this.widget_creator)(
                                    NextScreen::Error(e),
                                ));
                            }
                            break 'pop_state;
                        }
                        RenderLoopStateProj::Render { state, widget } => {
                            if let Poll::Ready(res) = state.poll_render(
                                widget.as_deref_mut().expect("polled after return?"),
                                this.events,
                                this.term,
                                cx,
                            ) {
                                let widget = widget.take().expect("polled after return?");
                                match Navigation::from(res) {
                                    Navigation::PopContext => {
                                        break 'new_state make_render_stop(widget, None);
                                    }
                                    Navigation::Push(next_screen) => {
                                        this.state.push(widget, this.widget_creator.clone());
                                        break 'new_state make_render((this.widget_creator)(
                                            next_screen,
                                        ));
                                    }
                                    Navigation::Replace(next_screen) => {
                                        break 'new_state make_render_stop(
                                            widget,
                                            Some(next_screen),
                                        );
                                    }
                                    Navigation::Exit => return Poll::Ready(()),
                                    Navigation::PushWithoutTui(pin) => {
                                        break 'new_state make_without_tui(pin);
                                    }
                                }
                            } else if let Some(next) = ready!(this.external.poll_recv(cx)) {
                                let widget = widget.take().expect("polled after return?");
                                this.state.push(widget, this.widget_creator.clone());
                                break 'new_state make_render((this.widget_creator)(next));
                            }
                            if !*this.external_closed_detected {
                                *this.external_closed_detected = true;
                                warn!("external widget queue is closed");
                            }
                            return Poll::Pending;
                        }
                        RenderLoopStateProj::RenderStop {
                            state,
                            widget,
                            next,
                        } => {
                            match ready!(state.poll_render_stop(widget, this.events, this.term, cx))
                            {
                                RenderStopRes::Ok => {
                                    if let Some(next) = next.take() {
                                        break 'new_state make_render((this.widget_creator)(next));
                                    }
                                    break 'pop_state;
                                }
                                RenderStopRes::Exit => return Poll::Ready(()),
                            }
                        }
                        RenderLoopStateProj::Suspended { handle } => {
                            match ready!(handle.poll_unpin(cx)).expect("suspended widget paniced") {
                                RunResult::Cont(widget) => break 'new_state make_render(widget),
                                RunResult::Empty => {
                                    break 'pop_state;
                                }
                                RunResult::Exit => return Poll::Ready(()),
                            }
                        }
                    };
                }
                break 'new_state match this.state.pop() {
                    StateValue::Suspended(suspended) => {
                        debug!("resuming suspended widget: {}", suspended.name);
                        RenderLoopState::Suspended {
                            handle: suspended.get_widget(),
                        }
                    }
                    StateValue::Empty => {
                        info!("stack is now empty");
                        return Poll::Ready(());
                    }
                    StateValue::WithoutTui(without_tui) => RenderLoopState::WithoutTui {
                        fut: run_without(without_tui),
                    },
                };
            };
            this.loop_state.set(new_state);
            //ensure this future participates in tokio cooperative scheduling
            ready!(poll_proceed(cx)).made_progress();
        }
    }
}

fn make_without_tui(f: BoxFuture<'static, Result<()>>) -> RenderLoopState {
    RenderLoopState::WithoutTui {
        fut: run_without(f),
    }
}

fn make_render(widget: Box<ShadedWidget<Navigation>>) -> RenderLoopState {
    RenderLoopState::Render {
        state: render_widget(),
        widget: Some(widget),
    }
}

fn make_render_stop(
    widget: Box<ShadedWidget<Navigation>>,
    next: Option<NextScreen>,
) -> RenderLoopState {
    RenderLoopState::RenderStop {
        state: render_widget_stop(),
        widget,
        next,
    }
}

pub fn render_loop<'e>(
    initial: NextScreen,
    widget_creator: WidgetCreator,
    state: &'e StateStack,
    term: &'e mut DefaultTerminal,
    events: &'e mut KeybindEvents,
    external: &'e mut UnboundedReceiver<NextScreen>,
) -> RenderLoop<'e> {
    let loop_state = make_render(widget_creator(initial));
    RenderLoop {
        widget_creator,
        state,
        term,
        events,
        external,
        external_closed_detected: false,
        loop_state,
    }
}

pub struct WidgetPusher {
    inner: UnboundedSender<NextScreen>,
}
impl DerefMut for WidgetPusher {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Deref for WidgetPusher {
    type Target = UnboundedSender<NextScreen>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
