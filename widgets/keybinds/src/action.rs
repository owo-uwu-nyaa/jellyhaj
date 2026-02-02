use std::{iter, mem};

use color_eyre::Result;
use itertools::Either;
use jellyhaj_widgets_core::{JellyhajWidget, Wrapper, async_task::TaskSubmitter};
use keybinds::{Command, Key, KeyBinding};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use tracing::{debug, warn};

use crate::{
    CommandAction, CommandMapper, KeybindAction, KeybindWidget, KeybindWrapper, MappedCommand,
};

fn if_non_empty<T>(v: &Vec<T>) -> Option<&Vec<T>> {
    if v.is_empty() { None } else { Some(v) }
}

pub fn apply_key_event<'e, T: Command, W: JellyhajWidget, M: CommandMapper<T, D = W::Action>>(
    this: &mut KeybindWidget<'e, T, W, M>,
    task: TaskSubmitter<KeybindAction<W::Action>, impl Wrapper<KeybindAction<W::Action>>>,
    action: KeybindAction<W::Action>,
) -> Result<Option<CommandAction<M::U, W::ActionResult>>> {
    match action {
        KeybindAction::Inner(a) => match this
            .inner
            .apply_action(task.wrap_with(KeybindWrapper), a)
        {
            Ok(None) => Ok(None),
            Ok(Some(r)) => Ok(Some(CommandAction::Action(r))),
            Err(e) => Err(e),
        },
        KeybindAction::Key(key_event) => match key_event {
            KeyEvent {
                code: KeyCode::Right,
                modifiers: KeyModifiers::CONTROL,
                kind: KeyEventKind::Press,
                state: _,
            } => {
                debug!("moving keybind help page");
                this.current_view = this.current_view.saturating_add(1);
                Ok(None)
            }
            KeyEvent {
                code: KeyCode::Left,
                modifiers: KeyModifiers::CONTROL,
                kind: KeyEventKind::Press,
                state: _,
            } => {
                debug!("moving keybind help page");
                this.current_view = this.current_view.saturating_add(1);
                Ok(None)
            }
            KeyEvent {
                code,
                modifiers,
                kind: KeyEventKind::Press | KeyEventKind::Repeat,
                state: _,
            } => {
                if this.inner.accepts_text_input()
                    && let KeyCode::Char(c) = code
                    && this.next_maps.is_empty()
                    && modifiers
                        .intersection(KeyModifiers::CONTROL | KeyModifiers::ALT)
                        .is_empty()
                {
                    debug!("keyboard press in text field");
                    this.inner.accept_char(c);
                    Ok(None)
                } else {
                    let current_map = mem::take(&mut this.next_maps);
                    let (top, minor) = (&this.top, &this.minor);
                    debug!(?current_map, "matching on active keymaps");

                    for c in if_non_empty(current_map.as_ref())
                        .map(|v| Either::Right(v.iter()))
                        .unwrap_or_else(|| Either::Left(iter::once(top).chain(minor)))
                    {
                        match c.get(&Key {
                            inner: code,
                            control: modifiers.contains(KeyModifiers::CONTROL),
                            alt: modifiers.contains(KeyModifiers::ALT),
                        }) {
                            Some(KeyBinding::Command(c)) => {
                                debug!("found matching command");
                                this.next_maps = Vec::new();
                                return match this.mapper.map(*c) {
                                    MappedCommand::Up(u) => Ok(Some(CommandAction::Up(u))),
                                    MappedCommand::Down(a) => match this
                                        .inner
                                        .apply_action(task.wrap_with(KeybindWrapper), a)
                                    {
                                        Ok(None) => Ok(None),
                                        Ok(Some(r)) => Ok(Some(CommandAction::Action(r))),
                                        Err(e) => Err(e),
                                    },
                                };
                            }
                            Some(KeyBinding::Group { map, name }) => {
                                debug!(name, "found matching group");
                                this.next_maps.push(map.clone());
                            }
                            Some(KeyBinding::Invalid(name)) => {
                                warn!("'{name}' is an invalid command");
                                if !current_map.is_empty() {
                                    this.next_maps = Vec::new();
                                }
                                break;
                            }
                            None => {}
                        }
                    }
                    Ok(None)
                }
            }
            _ => Ok(None),
        },
    }
}
