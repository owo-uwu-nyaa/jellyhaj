use std::{mem, ops::ControlFlow};

use color_eyre::Result;
use jellyhaj_core::{Config, state::Navigation};
use jellyhaj_widgets_core::{ContextRef, JellyhajWidget, WidgetContext, Wrapper};
use keybinds::{Command, Key, KeyBinding};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use tracing::{debug, warn};

use crate::{CommandMapper, KeybindAction, KeybindWidget, KeybindWrapper};

pub fn apply_key_event<
    R: ContextRef<Config> + 'static,
    T: Command,
    W: JellyhajWidget<R>,
    M: CommandMapper<T, A = W::Action>,
>(
    this: &mut KeybindWidget<R, T, W, M>,
    cx: WidgetContext<'_, KeybindAction<W::Action>, impl Wrapper<KeybindAction<W::Action>>, R>,
    action: KeybindAction<W::Action>,
) -> Result<Option<ControlFlow<Navigation, W::ActionResult>>> {
    match action {
        KeybindAction::Inner(a) => match this.inner.apply_action(cx.wrap_with(KeybindWrapper), a) {
            Ok(None) => Ok(None),
            Ok(Some(r)) => Ok(Some(ControlFlow::Continue(r))),
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
                    && this.next_maps.is_none()
                    && modifiers
                        .intersection(KeyModifiers::CONTROL | KeyModifiers::ALT)
                        .is_empty()
                {
                    debug!("keyboard press in text field");
                    this.inner.accept_char(c);
                    Ok(None)
                } else {
                    let current_map = mem::take(&mut this.next_maps);
                    debug!(?current_map, "matching on active keymaps");
                    match current_map.as_ref().unwrap_or(&this.top).get(&Key {
                        inner: code,
                        control: modifiers.contains(KeyModifiers::CONTROL),
                        alt: modifiers.contains(KeyModifiers::ALT),
                    }) {
                        Some(KeyBinding::Command(c)) => {
                            debug!("found matching command");
                            this.next_maps = None;
                            debug!("executing command {c:?}");
                            let mapped = this.mapper.map(*c);
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
                        Some(KeyBinding::Group { map, name }) => {
                            debug!(name, "found matching group");
                            this.next_maps = Some(map.clone());
                        }
                        Some(KeyBinding::Invalid(name)) => {
                            warn!("'{name}' is an invalid command");
                            this.next_maps = None;
                        }
                        None => {}
                    }
                    Ok(None)
                }
            }
            _ => Ok(None),
        },
    }
}
