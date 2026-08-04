use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use futures_util::StreamExt;
use jellyhaj_widgets_core::{Buffer, Rect, Result};
use keybinds::KeybindEvents;
use ratatui::{DefaultTerminal, prelude::Backend};
use tokio::{
    task::coop::poll_proceed,
    time::{Instant, Sleep, sleep_until},
};
use tracing::trace;

use crate::widgets::{WidgetResult, shaded::widget::ShadedWidget};

pub fn render_to_term<T>(
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

pub struct RenderWidget {
    render: bool,
    sleep_fut: Option<Sleep>,
}

impl RenderWidget {
    #[inline]
    const fn project(self: Pin<&mut Self>) -> RenderWidgetProj<'_> {
        let this = unsafe { self.get_unchecked_mut() };
        RenderWidgetProj {
            render: &mut this.render,
            sleep_fut: unsafe { Pin::new_unchecked(&mut this.sleep_fut) },
        }
    }

    pub fn poll_render<Res: 'static>(
        mut self: Pin<&mut Self>,
        widget: &mut ShadedWidget<Res>,
        events: &mut KeybindEvents,
        term: &mut DefaultTerminal,
        cx: &mut Context<'_>,
    ) -> Poll<WidgetResult<Res>> {
        let poll_res = self
            .as_mut()
            .project()
            .poll_widget(widget, events, term, cx);
        let mut this = self.project();

        loop {
            if let Some(sleep_fut) = this.sleep_fut.as_mut().as_pin_mut()
                && sleep_fut.poll(cx).is_ready()
            {
                trace!("time based render wakeup");
                *this.render = true;
                this.sleep_fut.as_mut().set(None);
            }
            if *this.render {
                trace!("rendering widget");
                *this.render = false;
                match render_to_term(term, |area, buf| widget.render_shaded(area, buf)) {
                    Err(e) => {
                        tracing::error!("failed to draw to the terminal:\n{e:?}");
                        return Poll::Ready(WidgetResult::Exit);
                    }
                    Ok(Ok(fps)) => {
                        if fps > 0 {
                            let duration = Duration::from_secs(1) / u32::from(fps);
                            this.sleep_fut
                                .set(Some(sleep_until(Instant::now() + duration)));
                            let Poll::Ready(consume_budget) = poll_proceed(cx) else {
                                break;
                            };
                            consume_budget.made_progress();
                            continue;
                        }
                    }
                    Ok(Err(e)) => return Poll::Ready(WidgetResult::Err(e)),
                }
            }
            break;
        }
        poll_res
    }
}

pub struct RenderWidgetProj<'e> {
    render: &'e mut bool,
    sleep_fut: Pin<&'e mut Option<Sleep>>,
}

impl RenderWidgetProj<'_> {
    fn poll_widget<Res: 'static>(
        self,
        widget: &mut ShadedWidget<Res>,
        events: &mut KeybindEvents,
        term: &mut DefaultTerminal,
        cx: &mut Context<'_>,
    ) -> Poll<WidgetResult<Res>> {
        loop {
            let Poll::Ready(consume_budget) = poll_proceed(cx) else {
                return Poll::Pending;
            };
            if let Poll::Ready(nav) = events.poll_next_unpin(cx) {
                trace!("keyboard based render wakeup");
                consume_budget.made_progress();
                match nav {
                    None => return Poll::Ready(WidgetResult::Exit),
                    Some(Err(e)) => {
                        tracing::error!("reading keyboard event failed:\n{e:?}");
                        return Poll::Ready(WidgetResult::Exit);
                    }
                    Some(Ok(event)) => {
                        let (res, r) =
                            widget.submit_event(event, term.get_frame().area().as_size());
                        *self.render |= r;
                        if let Some(nav) = res {
                            return Poll::Ready(nav);
                        }
                    }
                }
            } else if let Poll::Ready(nav) = widget.poll_next_unpin(cx) {
                trace!("task based render wakeup");
                consume_budget.made_progress();
                match nav {
                    None => return Poll::Ready(WidgetResult::Exit),
                    Some(Some(nav)) => return Poll::Ready(nav),
                    Some(None) => {
                        *self.render = true;
                    }
                }
            } else {
                return Poll::Pending;
            }
        }
    }
}

#[must_use]
pub const fn render_widget() -> RenderWidget {
    RenderWidget {
        render: true,
        sleep_fut: None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderStopRes {
    Ok,
    Exit,
}

pub struct RenderStopWidget {
    render: bool,
    sleep_fut: Option<Sleep>,
}

impl RenderStopWidget {
    const fn project(self: Pin<&mut Self>) -> RenderStopWidgetProj<'_> {
        let this = unsafe { self.get_unchecked_mut() };
        RenderStopWidgetProj {
            render: &mut this.render,
            sleep_fut: unsafe { Pin::new_unchecked(&mut this.sleep_fut) },
        }
    }
    pub fn poll_render_stop<Res: 'static>(
        mut self: Pin<&mut Self>,
        widget: &mut ShadedWidget<Res>,
        events: &mut KeybindEvents,
        term: &mut DefaultTerminal,
        cx: &mut Context<'_>,
    ) -> Poll<RenderStopRes> {
        let poll_res = self
            .as_mut()
            .project()
            .poll_widget(widget, events, term, cx);
        let mut this = self.project();

        loop {
            if let Some(sleep_fut) = this.sleep_fut.as_mut().as_pin_mut()
                && sleep_fut.poll(cx).is_ready()
            {
                trace!("time based render wakeup");
                *this.render = true;
                this.sleep_fut.as_mut().set(None);
            }
            if *this.render {
                trace!("rendering widget");
                *this.render = false;
                match render_to_term(term, |area, buf| widget.render_stop(area, buf)) {
                    Err(e) => {
                        tracing::error!("failed to draw to the terminal:\n{e:?}");
                        return Poll::Ready(RenderStopRes::Exit);
                    }
                    Ok(Ok(fps)) => {
                        if widget.is_stopped_finished() {
                            return Poll::Ready(RenderStopRes::Ok);
                        }
                        assert!(fps > 0, "Stop effect with fps 0 may never complete!");
                        let duration = Duration::from_secs(1) / u32::from(fps);
                        this.sleep_fut
                            .set(Some(sleep_until(Instant::now() + duration)));
                        let Poll::Ready(consume_budget) = poll_proceed(cx) else {
                            break;
                        };
                        consume_budget.made_progress();
                        continue;
                    }
                    Ok(Err(e)) => {
                        tracing::error!("Error rendering stop animation:\n{e:?}");
                        return Poll::Ready(RenderStopRes::Ok);
                    }
                }
            }
            break;
        }
        poll_res
    }
}

pub struct RenderStopWidgetProj<'e> {
    render: &'e mut bool,
    sleep_fut: Pin<&'e mut Option<Sleep>>,
}

impl RenderStopWidgetProj<'_> {
    fn poll_widget<Res: 'static>(
        self,
        widget: &mut ShadedWidget<Res>,
        events: &mut KeybindEvents,
        term: &mut DefaultTerminal,
        cx: &mut Context<'_>,
    ) -> Poll<RenderStopRes> {
        loop {
            let Poll::Ready(consume_budget) = poll_proceed(cx) else {
                return Poll::Pending;
            };
            if let Poll::Ready(nav) = events.poll_next_unpin(cx) {
                trace!("keyboard based render wakeup");
                consume_budget.made_progress();
                match nav {
                    None => return Poll::Ready(RenderStopRes::Exit),
                    Some(Err(e)) => {
                        tracing::error!("reading keyboard event failed:\n{e:?}");
                        return Poll::Ready(RenderStopRes::Exit);
                    }
                    Some(Ok(event)) => {
                        let (_, r) = widget.submit_event(event, term.get_frame().area().as_size());
                        *self.render |= r;
                    }
                }
            } else if let Poll::Ready(nav) = widget.poll_next_unpin(cx) {
                trace!("task based render wakeup");
                consume_budget.made_progress();
                match nav {
                    None => return Poll::Ready(RenderStopRes::Exit),
                    Some(Some(_) | None) => {
                        *self.render = true;
                    }
                }
            } else {
                return Poll::Pending;
            }
        }
    }
}

#[must_use]
pub const fn render_widget_stop() -> RenderStopWidget {
    RenderStopWidget {
        render: true,
        sleep_fut: None,
    }
}
