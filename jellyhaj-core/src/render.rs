use std::{
    fmt::Debug,
    future::poll_fn,
    io::Write,
    mem,
    pin::{Pin, pin},
    task::{
        Context,
        Poll::{self, Ready},
        ready,
    },
};

use color_eyre::Report;
use either::Either;
use futures_util::future::BoxFuture;
use jellyfin::Result;
use jellyhaj_widgets_core::{
    ContextRef, GetFromContext, JellyhajWidget, JellyhajWidgetExt, JellyhajWidgetState, Position,
    TreeVisitor, WidgetContext, WidgetTreeVisitor,
    async_task::{EventReceiver, IdWrapper, Stream, StreamExt, TaskSubmitter},
};
use keybinds::KeybindEvents;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{Event, KeyEvent, MouseEvent},
};
use spawn::{CancellationToken, Spawner};
use tokio::{select, task::JoinHandle};
use tracing::debug;

use crate::state::{Navigation, NextScreen};

#[derive(Debug)]
pub enum KeybindAction<A: Debug + Send + 'static> {
    Inner(A),
    Key(KeyEvent),
}

pub type Suspended = Box<dyn SuspendedWidget + Send>;

pub trait SuspendedWidget {
    fn name(&self) -> &'static str;
    fn resume<'p>(
        &mut self,
        term: &'p mut DefaultTerminal,
        events: &'p mut KeybindEvents,
    ) -> Pin<Box<dyn Future<Output = NavigationResult> + Send + 'p>>;
    fn visit_widget_tree(&self, visitor: &mut dyn TreeVisitor);
}

struct SuspendedWidgetImpl<
    A: Debug + Send + 'static,
    R: Send + 'static,
    W: JellyhajWidget<R, Action = KeybindAction<A>, ActionResult = Navigation> + 'static,
> {
    task: Option<JoinHandle<Hydrated<R, W::State>>>,
    stop: Option<tokio_util::sync::DropGuard>,
}

impl<
    A: Debug + Send + 'static,
    R: Send + 'static,
    W: JellyhajWidget<R, Action = KeybindAction<A>, ActionResult = Navigation> + 'static,
> SuspendedWidget for SuspendedWidgetImpl<A, R, W>
{
    fn name(&self) -> &'static str {
        <W::State as JellyhajWidgetState<R>>::NAME
    }

    fn resume<'p>(
        &mut self,
        term: &'p mut DefaultTerminal,
        events: &'p mut KeybindEvents,
    ) -> Pin<Box<dyn Future<Output = NavigationResult> + Send + 'p>> {
        self.stop = None;
        let renderer: HydrateRenderer<'_, A, R, W> = HydrateRenderer::Hydrating {
            task: self.task.take().expect("tried to hydrate twice"),
            term,
            events,
        };
        Box::pin(renderer)
    }

    fn visit_widget_tree(&self, mut visitor: &mut dyn TreeVisitor) {
        visitor.visit::<_, W::State>();
    }
}

pub enum NavigationResult {
    Exit,
    Pop,
    Replace(NextScreen),
    Push {
        current: Suspended,
        next: NextScreen,
    },
    PushWithoutTui {
        current: Suspended,
        without_tui: BoxFuture<'static, Result<()>>,
    },
}
enum Hydrated<R: 'static, S: JellyhajWidgetState<R>> {
    Finished(NavigationResult),
    Widget {
        state: S,
        submitter: TaskSubmitter<S::Action, IdWrapper>,
        receiver: EventReceiver<S::Action>,
        context: R,
    },
}

enum HydrateRenderer<
    't,
    A: Debug + Send + 'static,
    R: 'static,
    W: JellyhajWidget<R, Action = KeybindAction<A>, ActionResult = Navigation> + 'static,
> {
    Hydrating {
        task: JoinHandle<Hydrated<R, W::State>>,
        term: &'t mut DefaultTerminal,
        events: &'t mut KeybindEvents,
    },
    Rendering(WidgetRenderer<'t, A, Navigation, R, W>),
    Exit,
}

impl<
    't,
    A: Debug + Send + 'static,
    R: 'static,
    W: JellyhajWidget<R, Action = KeybindAction<A>, ActionResult = Navigation> + 'static,
> HydrateRenderer<'t, A, R, W>
{
    fn project(self: Pin<&mut Self>) -> &mut Self {
        unsafe { self.get_unchecked_mut() }
    }
}

impl<
    't,
    A: Debug + Send + 'static,
    R: Send + 'static,
    W: JellyhajWidget<R, Action = KeybindAction<A>, ActionResult = Navigation>,
> Future for HydrateRenderer<'t, A, R, W>
{
    type Output = NavigationResult;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let res = match this {
            HydrateRenderer::Hydrating {
                task,
                term: _,
                events: _,
            } => match ready!(Pin::new(task).poll(cx)) {
                Err(e) => {
                    panic!("suspended widget task paniced!\n{e:?}");
                }
                Ok(Hydrated::Finished(nav)) => {
                    debug!("suspended widget already finished");
                    return Ready(nav);
                }
                Ok(Hydrated::Widget {
                    state,
                    submitter,
                    receiver,
                    context,
                }) => {
                    debug!("received suspended widget");
                    if let HydrateRenderer::Hydrating {
                        task: _,
                        term,
                        events,
                    } = mem::replace(this, HydrateRenderer::Exit)
                    {
                        let widget = state.into_widget(&context);
                        let mut rendering = WidgetRenderer {
                            term,
                            events: Events {
                                receiver,
                                events,
                                first: true,
                            },
                            widget,
                            task: submitter,
                            render: true,
                            cx: context,
                        };
                        let res = rendering.poll(cx);
                        *this = HydrateRenderer::Rendering(rendering);
                        ready!(res).transform()
                    } else {
                        unreachable!()
                    }
                }
            },
            HydrateRenderer::Rendering(widget_renderer) => {
                ready!(widget_renderer.poll(cx)).transform()
            }
            HydrateRenderer::Exit => {
                unreachable!("the render future either already paniced or was called after ")
            }
        };
        if let HydrateRenderer::Rendering(renderer) = mem::replace(this, HydrateRenderer::Exit) {
            Ready(with_suspend_current(res, renderer))
        } else {
            unreachable!()
        }
    }
}

fn with_suspend_current<
    A: Debug + Send + 'static,
    R: Send + 'static,
    W: JellyhajWidget<R, Action = KeybindAction<A>, ActionResult = Navigation> + 'static,
>(
    res: std::result::Result<Navigation, Report>,
    renderer: WidgetRenderer<'_, A, Navigation, R, W>,
) -> NavigationResult {
    match res {
        Err(e) => NavigationResult::Replace(NextScreen::Error(e) ),
        Ok(Navigation::Exit) => NavigationResult::Exit,
        Ok(Navigation::PopContext) => NavigationResult::Pop,
        Ok(Navigation::Replace(n)) => NavigationResult::Replace(n),
        Ok(Navigation::Push(next)) => NavigationResult::Push {
            current: suspend(renderer),
            next,
        },
        Ok(Navigation::PushWithoutTui(without_tui)) => NavigationResult::PushWithoutTui {
            current: suspend(renderer),
            without_tui,
        },
    }
}

#[track_caller]
fn spawn<
    A: Debug + Send + 'static,
    R: Send + 'static,
    W: JellyhajWidget<R, Action = KeybindAction<A>, ActionResult = Navigation> + 'static,
>(
    fut: impl Future<Output = Hydrated<R, W::State>> + Send + 'static,
) -> JoinHandle<Hydrated<R, W::State>> {
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
}

async fn run_suspended<
    A: Debug + Send + 'static,
    R: Send + 'static,
    S: JellyhajWidgetState<R, Action = KeybindAction<A>, ActionResult = Navigation> + 'static,
>(
    mut state: S,
    task: TaskSubmitter<KeybindAction<A>, IdWrapper>,
    mut receiver: EventReceiver<KeybindAction<A>>,
    cx: R,
    stop: CancellationToken,
) -> Hydrated<R, S> {
    let mut stop_fut = pin!(stop.cancelled_owned());
    loop {
        let res = select! {
            biased;
            _ = &mut stop_fut => {
                return Hydrated::Widget{ state, submitter: task, receiver, context: cx }
            }
            res = receiver.next() => {
                res
            }
        };
        let action = match res {
            None => unreachable!(),
            Some(Err(e)) => {
                return Hydrated::Finished(NavigationResult::Replace(NextScreen::Error(
                    e,
                ) ));
            }
            Some(Ok(a)) => a,
        };
        match state.apply_action(
            WidgetContext {
                refs: &cx,
                submitter: task.as_ref(),
            },
            action,
        ) {
            Err(e) => {
                return Hydrated::Finished(NavigationResult::Replace(NextScreen::Error(
                    e,
                ) ));
            }
            Ok(None) => {}
            Ok(Some(Navigation::PopContext)) => {
                return Hydrated::Finished(NavigationResult::Pop);
            }
            Ok(Some(Navigation::Exit)) => {
                return Hydrated::Finished(NavigationResult::Exit);
            }
            Ok(Some(Navigation::Replace(next))) => {
                return Hydrated::Finished(NavigationResult::Replace(next));
            }
            Ok(Some(Navigation::Push(next))) => {
                return Hydrated::Finished(NavigationResult::Push {
                    current: suspend_state(state, task, receiver, cx),
                    next,
                });
            }
            Ok(Some(Navigation::PushWithoutTui(without_tui))) => {
                return Hydrated::Finished(NavigationResult::PushWithoutTui {
                    current: suspend_state(state, task, receiver, cx),
                    without_tui,
                });
            }
        }
    }
}

fn suspend_state<
    A: Debug + Send + 'static,
    R: Send + 'static,
    S: JellyhajWidgetState<R, Action = KeybindAction<A>, ActionResult = Navigation> + 'static,
>(
    state: S,
    task: TaskSubmitter<KeybindAction<A>, IdWrapper>,
    receiver: EventReceiver<KeybindAction<A>>,
    cx: R,
) -> Box<SuspendedWidgetImpl<A, R, S::Widget>> {
    let stop = CancellationToken::new();
    Box::new(SuspendedWidgetImpl {
        task: Some(spawn::<A, R, S::Widget>(run_suspended(
            state,
            task,
            receiver,
            cx,
            stop.clone(),
        ))),
        stop: Some(stop.drop_guard()),
    })
}

fn suspend<
    A: Debug + Send + 'static,
    R: Send + 'static,
    W: JellyhajWidget<R, Action = KeybindAction<A>, ActionResult = Navigation> + 'static,
>(
    renderer: WidgetRenderer<'_, A, Navigation, R, W>,
) -> Box<SuspendedWidgetImpl<A, R, W>> {
    suspend_state(
        renderer.widget.into_state(),
        renderer.task,
        renderer.events.receiver,
        renderer.cx,
    )
}

struct Events<'t, A: Debug + Send + 'static> {
    receiver: EventReceiver<KeybindAction<A>>,
    events: &'t mut KeybindEvents,
    first: bool,
}

impl<'t, A: Debug + Send + 'static> Stream for Events<'t, A> {
    type Item = Result<Either<Event, KeybindAction<A>>>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let events: Pin<&mut KeybindEvents> = Pin::new(&mut this.events);
        let receiver = Pin::new(&mut this.receiver);
        let first = this.first;
        this.first = first ^ true;
        if first {
            match receiver.poll_next(cx) {
                Ready(None) => Ready(None),
                Ready(Some(Err(e))) => Ready(Some(Err(e))),
                Ready(Some(Ok(v))) => Ready(Some(Ok(Either::Right(v)))),
                Poll::Pending => match events.poll_next(cx) {
                    Ready(None) => Ready(None),
                    Ready(Some(Err(e))) => Ready(Some(Err(e.into()))),
                    Ready(Some(Ok(v))) => Ready(Some(Ok(Either::Left(v)))),
                    Poll::Pending => Poll::Pending,
                },
            }
        } else {
            match events.poll_next(cx) {
                Ready(None) => Ready(None),
                Ready(Some(Err(e))) => Ready(Some(Err(e.into()))),
                Ready(Some(Ok(v))) => Ready(Some(Ok(Either::Left(v)))),
                Poll::Pending => match receiver.poll_next(cx) {
                    Ready(None) => Ready(None),
                    Ready(Some(Err(e))) => Ready(Some(Err(e))),
                    Ready(Some(Ok(v))) => Ready(Some(Ok(Either::Right(v)))),
                    Poll::Pending => Poll::Pending,
                },
            }
        }
    }
}

struct WidgetRenderer<
    't,
    A: Debug + Send + 'static,
    T: Debug,
    R: 'static,
    W: JellyhajWidget<R, Action = KeybindAction<A>, ActionResult = T>,
> {
    term: &'t mut DefaultTerminal,
    events: Events<'t, A>,
    widget: W,
    task: TaskSubmitter<KeybindAction<A>, IdWrapper>,
    render: bool,
    cx: R,
}

pub async fn render_widget_bare<
    A: Debug + Send + 'static,
    T: Debug,
    R: 'static,
    W: Send + Unpin + JellyhajWidget<R, Action = KeybindAction<A>, ActionResult = T>,
>(
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    spawner: Spawner,
    widget: W,
    cx: R,
) -> RenderResult<(T, W)> {
    let (task, receiver) = jellyhaj_widgets_core::async_task::new_task_pair(spawner);
    let mut renderer = WidgetRenderer {
        term,
        events: Events {
            receiver,
            events,
            first: true,
        },
        widget,
        task,
        render: true,
        cx,
    };
    match poll_fn(|cx| renderer.poll(cx)).await {
        RenderResult::Ok(v) => RenderResult::Ok((v, renderer.widget)),
        RenderResult::Err(report) => RenderResult::Err(report),
        RenderResult::Exit => RenderResult::Exit,
    }
}

pub async fn render_widget<
    A: Debug + Send + 'static,
    R: ContextRef<Spawner> + Send + 'static,
    S: JellyhajWidgetState<R, Action = KeybindAction<A>, ActionResult = Navigation>,
>(
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    cx: R,
    state: S,
) -> NavigationResult {
    let (task, receiver) =
        jellyhaj_widgets_core::async_task::new_task_pair(Spawner::get_ref(&cx).clone());
    let widget = state.into_widget(&cx);
    let mut renderer = WidgetRenderer {
        term,
        events: Events {
            receiver,
            events,
            first: true,
        },
        widget,
        task,
        render: true,
        cx,
    };
    let res = poll_fn(|cx| renderer.poll(cx)).await.transform();
    with_suspend_current(res, renderer)
}

impl<
    't,
    A: Debug + Send + 'static,
    T: Debug,
    R: 'static,
    W: JellyhajWidget<R, Action = KeybindAction<A>, ActionResult = T>,
> WidgetRenderer<'t, A, T, R, W>
{
    fn render_widget(&mut self) -> Result<()> {
        self.term.autoresize()?;
        let mut frame = self.term.get_frame();
        self.widget.render_fallible(
            frame.area(),
            frame.buffer_mut(),
            WidgetContext {
                refs: &self.cx,
                submitter: self.task.as_ref(),
            },
        )?;
        self.term.flush()?;
        self.term.hide_cursor()?;
        self.term.swap_buffers();
        self.term.backend_mut().flush()?;
        Ok(())
    }

    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<RenderResult<T>> {
        Ready(loop {
            if self.render {
                self.render = false;
                if let Err(e) = self.render_widget() {
                    break RenderResult::Err(e);
                }
            }
            let action_result = match ready!(self.events.poll_next_unpin(cx)) {
                None => break RenderResult::Exit,
                Some(Err(e)) => break RenderResult::Err(e),
                Some(Ok(Either::Left(Event::Key(key)))) => {
                    self.render = true;
                    self.widget.apply_action(
                        WidgetContext {
                            refs: &self.cx,
                            submitter: self.task.as_ref(),
                        },
                        KeybindAction::Key(key),
                    )
                }
                Some(Ok(Either::Left(Event::Mouse(MouseEvent {
                    kind,
                    column,
                    row,
                    modifiers,
                })))) => {
                    self.render = true;
                    self.widget.click(
                        WidgetContext {
                            refs: &self.cx,
                            submitter: self.task.as_ref(),
                        },
                        Position::new(column, row),
                        self.term.get_frame().area().as_size(),
                        kind,
                        modifiers,
                    )
                }
                Some(Ok(Either::Left(Event::Paste(v)))) if self.widget.accepts_text_input() => {
                    self.render = true;
                    self.widget.accept_text(v);
                    continue;
                }
                Some(Ok(Either::Left(Event::Resize(_, _)))) => {
                    self.render = true;
                    continue;
                }
                Some(Ok(Either::Left(_))) => continue,
                Some(Ok(Either::Right(v))) => {
                    self.render = true;
                    self.widget.apply_action(
                        WidgetContext {
                            refs: &self.cx,
                            submitter: self.task.as_ref(),
                        },
                        v,
                    )
                }
            };
            match action_result {
                Err(e) => break RenderResult::Err(e),
                Ok(None) => continue,
                Ok(Some(n)) => break RenderResult::Ok(n),
            }
        })
    }
}

pub enum RenderResult<T> {
    Ok(T),
    Err(Report),
    Exit,
}

impl RenderResult<Navigation> {
    pub fn transform(self) -> Result<Navigation> {
        match self {
            RenderResult::Ok(v) => Ok(v),
            RenderResult::Err(e) => Err(e),
            RenderResult::Exit => Ok(Navigation::Exit),
        }
    }
}
