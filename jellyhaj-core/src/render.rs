use std::{
    fmt::Debug,
    future::poll_fn,
    io::Write,
    mem,
    pin::{Pin, pin},
    sync::Arc,
    task::{
        Context,
        Poll::{self, Ready},
        ready,
    },
};

use color_eyre::Report;
use config::Config;
use either::Either;
use futures_util::future::BoxFuture;
use jellyfin::Result;
use jellyhaj_context::{
    DB, ImageProtocolCache, JellyfinEventInterests, KeybindEvents, Picker, TuiContext,
};
use jellyhaj_widgets_core::{
    JellyhajWidget, JellyhajWidgetExt, JellyhajWidgetState, Position, TreeVisitor, WidgetContext,
    WidgetTreeVisitor,
    async_task::{EventReceiver, IdWrapper, Stream, StreamExt, TaskSubmitter},
};
use ratatui::{
    DefaultTerminal,
    crossterm::event::{Event, KeyEvent, MouseEvent},
};
use spawn::{CancellationToken, Spawner};
use stats_data::Stats;
use tokio::{select, task::JoinHandle};
use tracing::debug;

use crate::state::{Navigation, Next, NextScreen};

#[derive(Debug)]
pub enum KeybindAction<A: Debug + Send + 'static> {
    Inner(A),
    Key(KeyEvent),
}

pub type Suspended = Box<dyn SuspendedWidget + Send>;

pub trait SuspendedWidget {
    fn name(&self) -> &'static str;
    fn resume<'a>(
        &mut self,
        cx: Pin<&'a mut TuiContext>,
    ) -> Pin<Box<dyn Future<Output = NavigationResult> + Send + 'a>>;
    fn visit_widget_tree(&self, visitor: &mut dyn TreeVisitor);
}

struct SuspendedWidgetImpl<
    A: Debug + Send + 'static,
    W: JellyhajWidget<Action = KeybindAction<A>, ActionResult = Navigation> + 'static,
> {
    task: Option<JoinHandle<Hydrated<W::State>>>,
    stop: Option<tokio_util::sync::DropGuard>,
}

impl<
    A: Debug + Send + 'static,
    W: JellyhajWidget<Action = KeybindAction<A>, ActionResult = Navigation> + 'static,
> SuspendedWidget for SuspendedWidgetImpl<A, W>
{
    fn name(&self) -> &'static str {
        <W::State as JellyhajWidgetState>::NAME
    }

    fn resume<'a>(
        &mut self,
        cx: Pin<&'a mut TuiContext>,
    ) -> Pin<Box<dyn Future<Output = NavigationResult> + Send + 'a>> {
        self.stop = None;
        let renderer: HydrateRenderer<'_, A, W> = HydrateRenderer::Hydrating {
            task: self.task.take().expect("tried to hydrate twice"),
            context: cx,
        };
        Box::pin(renderer)
    }

    fn visit_widget_tree(&self, mut visitor: &mut dyn TreeVisitor) {
        visitor.visit::<W::State>();
    }
}

pub enum NavigationResult {
    Exit,
    Pop,
    Replace(Next),
    Push {
        current: Suspended,
        next: Next,
    },
    PushWithoutTui {
        current: Suspended,
        without_tui: BoxFuture<'static, Result<()>>,
    },
}
enum Hydrated<S: JellyhajWidgetState> {
    Finished(Result<Navigation>),
    Widget {
        state: S,
        submitter: TaskSubmitter<S::Action, IdWrapper>,
        receiver: EventReceiver<S::Action>,
    },
}

enum HydrateRenderer<
    't,
    A: Debug + Send + 'static,
    W: JellyhajWidget<Action = KeybindAction<A>, ActionResult = Navigation> + 'static,
> {
    Hydrating {
        task: JoinHandle<Hydrated<W::State>>,
        context: Pin<&'t mut TuiContext>,
    },
    Rendering(WidgetRenderer<'t, A, Navigation, W>),
    Exit,
}

impl<
    't,
    A: Debug + Send + 'static,
    W: JellyhajWidget<Action = KeybindAction<A>, ActionResult = Navigation> + 'static,
> HydrateRenderer<'t, A, W>
{
    fn project(self: Pin<&mut Self>) -> &mut Self {
        unsafe { self.get_unchecked_mut() }
    }
}

impl<
    't,
    A: Debug + Send + 'static,
    W: JellyhajWidget<Action = KeybindAction<A>, ActionResult = Navigation>,
> Future for HydrateRenderer<'t, A, W>
{
    type Output = NavigationResult;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let res = match this {
            HydrateRenderer::Hydrating { task, context: _ } => {
                match ready!(Pin::new(task).poll(cx)) {
                    Err(e) => {
                        panic!("suspended widget task paniced!\n{e:?}");
                    }
                    Ok(Hydrated::Finished(nav)) => {
                        debug!("suspended widget already finished");
                        nav
                    }
                    Ok(Hydrated::Widget {
                        state,
                        submitter,
                        receiver,
                    }) => {
                        debug!("received suspended widget");
                        if let HydrateRenderer::Hydrating {
                            task: _,
                            mut context,
                        } = mem::replace(this, HydrateRenderer::Exit)
                        {
                            let widget = state.into_widget(context.as_mut());
                            let context = context.project();
                            let mut rendering = WidgetRenderer {
                                term: context.term,
                                events: Events {
                                    receiver,
                                    events: context.events,
                                    first: true,
                                },
                                widget,
                                task: submitter,
                                render: true,
                                config: context.config.clone(),
                                image_picker: context.image_picker.clone(),
                                cache: context.cache.clone(),
                                image_cache: context.image_cache.clone(),
                                jellyfin_events: context.jellyfin_events.clone(),
                                stats: context.stats.clone(),
                            };
                            let res = rendering.poll(cx);
                            *this = HydrateRenderer::Rendering(rendering);
                            ready!(res).transform()
                        } else {
                            unreachable!()
                        }
                    }
                }
            }
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
    W: JellyhajWidget<Action = KeybindAction<A>, ActionResult = Navigation> + 'static,
>(
    res: std::result::Result<Navigation, Report>,
    renderer: WidgetRenderer<'_, A, Navigation, W>,
) -> NavigationResult {
    match res {
        Err(e) => NavigationResult::Replace(Box::new(NextScreen::Error(e))),
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

fn spawn<
    A: Debug + Send + 'static,
    W: JellyhajWidget<Action = KeybindAction<A>, ActionResult = Navigation> + 'static,
>(
    fut: impl Future<Output = Hydrated<W::State>> + Send + 'static,
) -> JoinHandle<Hydrated<W::State>> {
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

fn suspend<
    A: Debug + Send + 'static,
    W: JellyhajWidget<Action = KeybindAction<A>, ActionResult = Navigation> + 'static,
>(
    renderer: WidgetRenderer<'_, A, Navigation, W>,
) -> Box<SuspendedWidgetImpl<A, W>> {
    let WidgetRenderer {
        term: _,
        events,
        widget,
        task,
        render: _,
        config,
        image_picker,
        cache,
        image_cache,
        stats,
        jellyfin_events,
    } = renderer;
    let mut receiver = events.receiver;
    let mut state = widget.into_state();
    let stop = CancellationToken::new();
    let stop_fut = stop.clone();
    let stop = stop.drop_guard();

    Box::new(SuspendedWidgetImpl::<A, W> {
        task: Some(spawn::<A, W>(async move {
            let mut stop_fut = pin!(stop_fut.cancelled_owned());
            loop {
                select! {
                    biased;
                    _ = &mut stop_fut => {
                        return Hydrated::Widget{ state, submitter: task, receiver }
                    }
                    res = receiver.next() => {
                        match res {
                            None => return Hydrated::Finished(Ok(Navigation::Exit)),
                            Some(Err(e)) => return Hydrated::Finished(Err(e)),
                            Some(Ok(a)) => match state.apply_action(
                                WidgetContext{
                                    config: &config,
                                    image_picker: &image_picker,
                                    cache: &cache,
                                    image_cache: &image_cache,
                                    stats: &stats,
                                    jellyfin_events: &jellyfin_events,
                                    submitter: task.as_ref()
                                }, a
                            ) {
                                Err(e) => return Hydrated::Finished(Err(e)),
                                Ok(None) => {}
                                Ok(Some(n)) => return Hydrated::Finished(Ok(n)),
                            },
                        }
                    }
                }
            }
        })),
        stop: Some(stop),
    })
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
    W: JellyhajWidget<Action = KeybindAction<A>, ActionResult = T>,
> {
    term: &'t mut DefaultTerminal,
    events: Events<'t, A>,
    widget: W,
    task: TaskSubmitter<KeybindAction<A>, IdWrapper>,
    render: bool,
    config: Arc<Config>,
    image_picker: Arc<Picker>,
    cache: DB,
    image_cache: ImageProtocolCache,
    stats: Stats,
    jellyfin_events: JellyfinEventInterests,
}

#[allow(clippy::too_many_arguments)]
pub async fn render_widget_bare<
    A: Debug + Send + 'static,
    T: Debug,
    W: Send + Unpin + JellyhajWidget<Action = KeybindAction<A>, ActionResult = T>,
>(
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    spawner: Spawner,
    widget: W,
    config: Arc<Config>,
    image_picker: Arc<Picker>,
    cache: DB,
    image_cache: ImageProtocolCache,
    stats: Stats,
    jellyfin_events: JellyfinEventInterests,
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
        config,
        image_picker,
        cache,
        image_cache,
        stats,
        jellyfin_events,
    };
    match poll_fn(|cx| renderer.poll(cx)).await {
        RenderResult::Ok(v) => RenderResult::Ok((v, renderer.widget)),
        RenderResult::Err(report) => RenderResult::Err(report),
        RenderResult::Exit => RenderResult::Exit,
    }
}

pub async fn render_widget<
    A: Debug + Send + 'static,
    S: JellyhajWidgetState<Action = KeybindAction<A>, ActionResult = Navigation>,
>(
    mut cx: Pin<&mut TuiContext>,
    state: S,
) -> NavigationResult {
    let (task, receiver) = jellyhaj_widgets_core::async_task::new_task_pair(cx.spawn.clone());
    let widget = state.into_widget(cx.as_mut());
    let cx = cx.project();
    let mut renderer = WidgetRenderer {
        term: cx.term,
        events: Events {
            receiver,
            events: cx.events,
            first: true,
        },
        widget,
        task,
        render: true,
        config: cx.config.clone(),
        image_picker: cx.image_picker.clone(),
        cache: cx.cache.clone(),
        image_cache: cx.image_cache.clone(),
        stats: cx.stats.clone(),
        jellyfin_events: cx.jellyfin_events.clone(),
    };
    let res = poll_fn(|cx| renderer.poll(cx)).await.transform();
    with_suspend_current(res, renderer)
}

impl<
    't,
    A: Debug + Send + 'static,
    T: Debug,
    W: JellyhajWidget<Action = KeybindAction<A>, ActionResult = T>,
> WidgetRenderer<'t, A, T, W>
{
    fn render_widget(&mut self) -> Result<()> {
        self.term.autoresize()?;
        let mut frame = self.term.get_frame();
        self.widget.render_fallible(
            frame.area(),
            frame.buffer_mut(),
            WidgetContext {
                config: &self.config,
                image_picker: &self.image_picker,
                cache: &self.cache,
                image_cache: &self.image_cache,
                stats: &self.stats,
                jellyfin_events: &self.jellyfin_events,
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
                            config: &self.config,
                            image_picker: &self.image_picker,
                            cache: &self.cache,
                            image_cache: &self.image_cache,
                            stats: &self.stats,
                            jellyfin_events: &self.jellyfin_events,
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
                            config: &self.config,
                            image_picker: &self.image_picker,
                            cache: &self.cache,
                            image_cache: &self.image_cache,
                            stats: &self.stats,
                            jellyfin_events: &self.jellyfin_events,
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
                            config: &self.config,
                            image_picker: &self.image_picker,
                            cache: &self.cache,
                            image_cache: &self.image_cache,
                            stats: &self.stats,
                            jellyfin_events: &self.jellyfin_events,
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
