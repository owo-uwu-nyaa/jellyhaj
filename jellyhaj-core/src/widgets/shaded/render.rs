use std::time::Duration;

use futures_util::StreamExt;
use jellyhaj_widgets_core::{Buffer, Rect, Result};
use keybinds::KeybindEvents;
use ratatui::{DefaultTerminal, prelude::Backend};
use tokio::{
    select,
    time::{Instant, sleep_until},
};

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
        let next = widget.next();
        select! {
        () = sleep_until(last_render+duration), if !duration.is_zero() => {
            render = true;
        }
        nav = next => {
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
