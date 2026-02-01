use std::{
    cmp::{max, min},
    mem,
};

use crate::{CommandAction, CommandMapper, KeybindWidget, MappedCommand};
use color_eyre::Result;
use itertools::Itertools;
use jellyhaj_widgets_core::JellyhajWidget;
use keybinds::{Command, KeyBinding};
use ratatui::layout::{Position, Size};
use tracing::{debug, warn};

pub fn apply_click<'e, T: Command, W: JellyhajWidget, M: CommandMapper<T, D = W::Action>>(
    this: &mut KeybindWidget<'e, T, W, M>,
    mut position: ratatui::prelude::Position,
    size: ratatui::prelude::Size,
    kind: ratatui::crossterm::event::MouseEventKind,
    modifier: ratatui::crossterm::event::KeyModifiers,
) -> Result<Option<CommandAction<M::U, W::ActionResult>>> {
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
                position,
                Size {
                    width: size.width,
                    height: size.height.saturating_sub(height as u16 + 1),
                },
                kind,
                modifier,
            ) {
                Ok(None) => Ok(None),
                Ok(Some(action)) => Ok(Some(CommandAction::Action(action))),
                Err(e) => Err(e),
            };
        } else {
            position.y = position.y.saturating_sub(size.height - height as u16 + 2);
            position.x = position.x.saturating_sub(2);

            let items_per_screen = width as usize * usable_height;
            if let Some(c) = this
                .next_maps
                .iter()
                .map(|v| v.iter())
                .kmerge_by(|(a, _), (b, _)| a < b)
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
                let current_map = mem::take(&mut this.next_maps);
                match c {
                    KeyBinding::Command(c) => {
                        debug!("found matching command");
                        this.next_maps = Vec::new();
                        return match this.mapper.map(c) {
                            MappedCommand::Up(u) => Ok(Some(CommandAction::Up(u))),
                            MappedCommand::Down(a) => match this.inner.apply_action(a) {
                                Ok(None) => Ok(None),
                                Ok(Some(r)) => Ok(Some(CommandAction::Action(r))),
                                Err(e) => Err(e),
                            },
                        };
                    }
                    KeyBinding::Group { map, name } => {
                        debug!(name, "found matching group");
                        this.next_maps.push(map.clone());
                    }
                    KeyBinding::Invalid(name) => {
                        warn!("'{name}' is an invalid command");
                        if !current_map.is_empty() {
                            this.next_maps = Vec::new();
                        }
                    }
                }
            }
        }
    } else {
        return match this.inner.click(position, size, kind, modifier) {
            Ok(None) => Ok(None),
            Ok(Some(r)) => Ok(Some(CommandAction::Action(r))),
            Err(e) => Err(e),
        };
    }
    Ok(None)
}
