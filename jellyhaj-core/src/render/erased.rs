use std::{
    fmt::Debug,
    pin::Pin,
    task::{Context, Poll, ready},
};

use crate::render::{KeybindAction, WidgetResult};
use color_eyre::Result;
use futures_util::{Stream, StreamExt};
use jellyhaj_widgets_core::{
    ContextRef, JellyhajWidget, JellyhajWidgetExt, Position, Size, TreeVisitor, WidgetContext,
    WidgetTreeVisitor,
    async_task::{EventReceiver, IdWrapper, TaskSubmitter, new_task_pair},
};
use pin_project_lite::pin_project;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{Event, MouseEvent},
    layout::Rect,
};
use spawn::Spawner;

pub trait ErasedWidget<Res>:
    Stream<Item = Option<WidgetResult<Res>>> + Send + 'static + Unpin
{
    fn name(&self) -> &'static str;
    fn submit_event(&mut self, event: Event, size: Size) -> (Option<WidgetResult<Res>>, bool);
    fn render(&mut self, area: Rect, buffer: &mut Buffer) -> Result<()>;
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
    }
}

pub trait ErasedWidgetExt<'w, Res> {
    fn filtered_events(self) -> WidgetEventStream<'w, Res>;
    fn next_filtered_event(self) -> impl Future<Output = Option<WidgetResult<Res>>> + Send;
}

impl<'w, Res> ErasedWidgetExt<'w, Res> for &'w mut dyn ErasedWidget<Res> {
    fn filtered_events(self) -> WidgetEventStream<'w, Res> {
        WidgetEventStream { inner: self }
    }

    fn next_filtered_event(self) -> impl Future<Output = Option<WidgetResult<Res>>> + Send {
        let mut stream = self.filtered_events();
        std::future::poll_fn(move |cx| stream.poll_next_unpin(cx))
    }
}

pub struct WidgetEventStream<'w, Res> {
    inner: &'w mut dyn ErasedWidget<Res>,
}

impl<Res> Stream for WidgetEventStream<'_, Res> {
    type Item = WidgetResult<Res>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            break match self.as_mut().get_mut().inner.poll_next_unpin(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Ready(Some(None)) => continue,
                Poll::Ready(Some(Some(v))) => Poll::Ready(Some(v)),
            };
        }
    }
}
