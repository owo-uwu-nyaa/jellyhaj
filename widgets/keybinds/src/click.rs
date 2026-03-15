use std::{
    cmp::{max, min},
    ops::ControlFlow,
};

use crate::{CommandMapper, KeybindAction, KeybindWidget, KeybindWrapper};
use color_eyre::Result;
use jellyhaj_core::{Config, state::Navigation};
use jellyhaj_widgets_core::{ContextRef, JellyhajWidget, WidgetContext, Wrapper};
use keybinds::{Command, KeyBinding};
use ratatui::layout::{Position, Size};
use tracing::{debug, warn};

pub fn apply_click<
    R: ContextRef<Config> + 'static,
    T: Command,
    W: JellyhajWidget<R>,
    M: CommandMapper<T, A = W::Action>,
>(
    this: &mut KeybindWidget<R, T, W, M>,
    cx: WidgetContext<'_, KeybindAction<W::Action>, impl Wrapper<KeybindAction<W::Action>>, R>,
    mut position: ratatui::prelude::Position,
    size: ratatui::prelude::Size,
    kind: ratatui::crossterm::event::MouseEventKind,
    modifier: ratatui::crossterm::event::KeyModifiers,
) -> Result<Option<ControlFlow<Navigation, W::ActionResult>>> {
    let len: usize = this.next_maps.iter().map(|v| v.len()).sum();
    if len > 0 {
        let width = (size.width - 4) / 20;
        let full_usable_height = len.div_ceil(width as usize);
        let full_height = full_usable_height + 4;
        let height = min(full_height, max(5, size.height as usize / 4));
        let usable_height = height - 4;
        let num_views = full_usable_height.div_ceil(usable_height);
        this.current_view = min(this.current_view, num_views - 1);
        if position.y < size.height - height as u16 {
            return match this.inner.click(
                cx.wrap_with(KeybindWrapper),
                position,
                Size {
                    width: size.width,
                    height: size.height.saturating_sub(height as u16 + 1),
                },
                kind,
                modifier,
            ) {
                Ok(None) => Ok(None),
                Ok(Some(action)) => Ok(Some(ControlFlow::Continue(action))),
                Err(e) => Err(e),
            };
        } else {
            position.y = position.y.saturating_sub(size.height - height as u16 + 2);
            position.x = position.x.saturating_sub(2);

            let items_per_screen = width as usize * usable_height;
            if let Some(c) = this
                .next_maps
                .iter()
                .flat_map(|m| m.iter())
                .skip(items_per_screen * this.current_view)
                .take(items_per_screen)
                .zip(
                    (0u16..usable_height as u16)
                        .flat_map(|y| (0u16..width).map(move |x| Position { x: x * 20, y })),
                )
                .filter(|(_, pos)| {
                    position.y == pos.y && (position.x..position.x + 16).contains(&pos.x)
                })
                .map(|((_, v), _)| v.clone())
                .next()
            {
                match c {
                    KeyBinding::Command(c) => {
                        debug!("found matching command");
                        this.next_maps = None;
                        debug!("executing command {c:?}");
                        let mapped = this.mapper.map(c);
                        debug!("triggering action {mapped:?}");
                        return match mapped {
                            ControlFlow::Break(u) => Ok(Some(ControlFlow::Break(u))),
                            ControlFlow::Continue(a) => {
                                match this.inner.apply_action(cx.wrap_with(KeybindWrapper), a) {
                                    Ok(None) => Ok(None),
                                    Ok(Some(r)) => Ok(Some(ControlFlow::Continue(r))),
                                    Err(e) => Err(e),
                                }
                            }
                        };
                    }
                    KeyBinding::Group { map, name } => {
                        debug!(name, "found matching group");
                        this.next_maps = Some(map.clone());
                    }
                    KeyBinding::Invalid(name) => {
                        warn!("'{name}' is an invalid command");
                        this.next_maps = None;
                    }
                }
            }
        }
    } else {
        return match this
            .inner
            .click(cx.wrap_with(KeybindWrapper), position, size, kind, modifier)
        {
            Ok(None) => Ok(None),
            Ok(Some(r)) => Ok(Some(ControlFlow::Continue(r))),
            Err(e) => Err(e),
        };
    }
    Ok(None)
}
