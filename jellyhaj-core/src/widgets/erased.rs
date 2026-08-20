use std::{
    fmt::Debug,
    pin::Pin,
    task::{Context, Poll, ready},
};

use crate::widgets::{KeybindAction, WidgetResult};
use color_eyre::Result;
use futures_util::Stream;
use jellyhaj_widgets_core::{
    ContextRef, JellyhajWidget, JellyhajWidgetExt, Position, RenderFlag, Size, TreeVisitor,
    WidgetContext, WidgetTreeVisitor,
    async_task::{EventReceiver, IdWrapper, TaskSubmitter, new_task_pair},
};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{Event, MouseEvent},
    layout::Rect,
};
use spawn::Spawner;

pub trait ErasedWidget<Res: 'static>: Send + 'static {
    fn name(&self) -> &'static str;
    fn submit_event(&mut self, event: Event, size: Size) -> Option<WidgetResult<Res>>;
    fn render(&mut self, area: Rect, buffer: &mut Buffer) -> Result<()>;
    fn visit(&self, visitor: &mut dyn TreeVisitor);
    fn reset_render_flag(&mut self) -> bool;
    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Option<WidgetResult<Res>>>>;
}

struct ErasedWidgetImpl<R: 'static, W: JellyhajWidget<R>> {
    widget: W,
    submitter: TaskSubmitter<W::Action, IdWrapper>,
    receiver: EventReceiver<W::Action>,
    context: R,
    render_flag: RenderFlag,
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
    ) -> Option<WidgetResult<W::ActionResult>> {
        let res = match event {
            Event::Key(key) => self.widget.apply_action(
                WidgetContext {
                    refs: &self.context,
                    submitter: self.submitter.as_ref(),
                },
                KeybindAction::Key(key),
                &mut self.render_flag,
            ),
            Event::Mouse(MouseEvent {
                kind,
                column,
                row,
                modifiers,
            }) => {
                if kind.is_moved() {
                    return None;
                }

                self.widget.click(
                    WidgetContext {
                        refs: &self.context,
                        submitter: self.submitter.as_ref(),
                    },
                    Position::new(column, row),
                    frame_size,
                    kind,
                    modifiers,
                    &mut self.render_flag,
                )
            }
            Event::Paste(v) => {
                if self.widget.accepts_text_input() {
                    self.widget.accept_text(v, &mut self.render_flag);
                }
                return None;
            }
            Event::Resize(_, _) => {
                self.render_flag.set();
                return None;
            }
            _ => return None,
        };
        match res {
            Ok(None) => None,
            Ok(Some(v)) => Some(WidgetResult::Ok(v)),
            Err(e) => Some(WidgetResult::Err(e)),
        }
    }

    fn render(&mut self, area: Rect, buffer: &mut Buffer) -> Result<()> {
        self.widget.render_fallible(
            area,
            buffer,
            WidgetContext {
                refs: &self.context,
                submitter: self.submitter.as_ref(),
            },
        )
    }

    fn reset_render_flag(&mut self) -> bool {
        self.render_flag.reset()
    }

    fn poll_next(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Option<WidgetResult<W::ActionResult>>>> {
        let this = self;

        Poll::Ready(Some(match ready!(this.receiver.poll_recv(cx)) {
            Some(Ok(action)) => match this.widget.apply_action(
                WidgetContext {
                    refs: &mut this.context,
                    submitter: this.submitter.as_ref(),
                },
                action,
                &mut this.render_flag,
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

pub(super) fn make_new_erased<
    R: ContextRef<Spawner> + Send + 'static,
    A: Debug + Send + 'static,
    W: JellyhajWidget<R, Action = KeybindAction<A>>,
>(
    cx: R,
    mut widget: W,
) -> impl ErasedWidget<W::ActionResult> {
    let (submitter, receiver) = new_task_pair(cx.as_ref().clone());
    widget.init(WidgetContext {
        refs: &cx,
        submitter: submitter.as_ref(),
    });
    ErasedWidgetImpl {
        widget,
        context: cx,
        submitter,
        receiver,
        render_flag: RenderFlag::default(),
    }
}

pub trait ErasedWidgetExt<'w, Res> {
    fn filtered_events(self) -> WidgetEventStream<'w, Res>;
    fn next_filtered_event(self) -> impl Future<Output = Option<WidgetResult<Res>>> + Send;
}

fn filtered_poll<Res: 'static>(
    erased: &mut dyn ErasedWidget<Res>,
    cx: &mut Context<'_>,
) -> Poll<Option<WidgetResult<Res>>> {
    loop {
        break match erased.poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(None)) => continue,
            Poll::Ready(Some(Some(v))) => Poll::Ready(Some(v)),
        };
    }
}

impl<'w, Res: 'static> ErasedWidgetExt<'w, Res> for &'w mut dyn ErasedWidget<Res> {
    fn filtered_events(self) -> WidgetEventStream<'w, Res> {
        WidgetEventStream { inner: self }
    }

    fn next_filtered_event(self) -> impl Future<Output = Option<WidgetResult<Res>>> + Send {
        std::future::poll_fn(move |cx| filtered_poll(self, cx))
    }
}

pub struct WidgetEventStream<'w, Res> {
    inner: &'w mut dyn ErasedWidget<Res>,
}

impl<Res: 'static> Stream for WidgetEventStream<'_, Res> {
    type Item = WidgetResult<Res>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        filtered_poll(self.as_mut().get_mut().inner, cx)
    }
}
