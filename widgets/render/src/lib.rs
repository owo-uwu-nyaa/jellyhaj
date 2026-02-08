use std::{
    fmt::Debug, pin::{Pin, pin}, task::Poll
};

use color_eyre::{Report, Result};
use either::Either;
use futures_util::Stream;
use futures_util::StreamExt;
use jellyhaj_keybinds_widget::{CommandAction, KeybindAction};
pub use jellyhaj_widgets_core::JellyhajWidget;
use jellyhaj_widgets_core::{
    JellyhajWidgetExt,
    async_task::{EventReceiver, IdWrapper, TaskSubmitter, new_task_pair},
};
use keybinds::KeybindEvents;
use pin_project_lite::pin_project;
use ratatui::{
    Terminal,
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent},
    prelude::Backend,
};
use spawn::Spawner;
use std::ops::DerefMut;

pub trait TermExt {
    fn render<
        A: Debug + Send + 'static,
        R: Debug + Send + 'static,
        M,
        W: Send + JellyhajWidget<Action = KeybindAction<A>, ActionResult = CommandAction<R, M>>,
    >(
        &mut self,
        widget: &mut W,
        events: &mut KeybindEvents,
        spawner: Spawner,
    ) -> impl Future<Output = Result<CommandAction<R, M>>> + Send;
}

pin_project! {
    struct SelectStream<'e, A: Send> {
        #[pin]
        events: &'e mut KeybindEvents,
        receiver: EventReceiver<A>,
        first:  bool
    }
}

impl<'e, A: Send> Stream for SelectStream<'e, A> {
    type Item = Either<std::result::Result<Event, std::io::Error>, Result<A>>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let res = if *this.first {
            match this.events.poll_next(cx) {
                Poll::Ready(Some(v)) => Poll::Ready(Some(Either::Left(v))),
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Pending => match Pin::new(this.receiver.deref_mut()).poll_next(cx) {
                    Poll::Ready(Some(v)) => Poll::Ready(Some(Either::Right(v))),
                    Poll::Ready(None) => Poll::Ready(None),
                    Poll::Pending => Poll::Pending,
                },
            }
        } else {
            match Pin::new(this.receiver.deref_mut()).poll_next(cx) {
                Poll::Ready(Some(v)) => Poll::Ready(Some(Either::Right(v))),
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Pending => match this.events.poll_next(cx) {
                    Poll::Ready(Some(v)) => Poll::Ready(Some(Either::Left(v))),
                    Poll::Ready(None) => Poll::Ready(None),
                    Poll::Pending => Poll::Pending,
                },
            }
        };
        *this.first ^= true;
        res
    }
}

impl<B: Backend<Error = std::io::Error> + Send> TermExt for Terminal<B> {
    async fn render<
        A: Debug+ Send + 'static,
        R: Debug+ Send + 'static,
        M,
        W: Send + JellyhajWidget<Action = KeybindAction<A>, ActionResult = CommandAction<R, M>>,
    >(
        &mut self,
        widget: &mut W,
        events: &mut KeybindEvents,
        spawner: Spawner,
    ) -> Result<CommandAction<R, M>> {
        let (task, receiver) = new_task_pair::<KeybindAction<A>>(spawner);
        draw(self, widget, task.clone())?;
        let mut stream = pin!(SelectStream {
            events,
            receiver,
            first: true,
        });
        while let Some(v) = stream.next().await {
            match v {
                Either::Left(Err(e)) => {
                    return Err(e.into());
                }
                Either::Left(Ok(Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: KeyEventKind::Press | KeyEventKind::Repeat,
                    state: _,
                }))) => return Ok(CommandAction::Exit),
                Either::Left(Ok(Event::Key(key))) => {
                    match widget.apply_action(task.clone(), KeybindAction::Key(key)) {
                        Ok(None) => {}
                        Ok(Some(v)) => return Ok(v),
                        Err(e) => return Err(e),
                    }
                }
                Either::Left(Ok(Event::Mouse(MouseEvent {
                    kind,
                    column,
                    row,
                    modifiers,
                }))) => {
                    let size = self.current_buffer_mut().area.as_size();
                    match widget.click(task.clone(), (column, row).into(), size, kind, modifiers) {
                        Ok(None) => {}
                        Ok(Some(v)) => return Ok(v),
                        Err(e) => return Err(e),
                    }
                }
                Either::Left(Ok(Event::Paste(v))) => {
                    if widget.accepts_text_input() {
                        widget.accept_text(v);
                    }
                }
                Either::Left(_) => {}
                Either::Right(Err(e)) => return Err(e),
                Either::Right(Ok(action)) => match widget.apply_action(task.clone(), action) {
                    Ok(None) => {}
                    Ok(Some(v)) => return Ok(v),
                    Err(e) => return Err(e),
                },
            }
            draw(self, widget, task.clone())?
        }
        todo!()
    }
}

fn draw<B: Backend<Error = std::io::Error> + Send, W: JellyhajWidget>(
    term: &mut Terminal<B>,
    widget: &mut W,
    task: TaskSubmitter<W::Action, IdWrapper>,
) -> Result<()> {
    let mut err: Option<Report> = None;
    let err_ref = &mut err;
    let render_res = term.try_draw(move |frame| {
        *err_ref = widget
            .render_fallible(frame.area(), frame.buffer_mut(), task)
            .err();
        if err_ref.is_some() {
            Err(std::io::Error::other("dummy"))
        } else {
            Ok(())
        }
    });
    if let Some(e) = err {
        Err(e)
    } else {
        render_res?;
        Ok(())
    }
}
