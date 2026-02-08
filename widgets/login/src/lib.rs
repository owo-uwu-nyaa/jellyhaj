mod render;

use jellyhaj_widgets_core::{JellyhajWidget, Wrapper, async_task::TaskSubmitter};
use ratatui::crossterm::event::{MouseButton, MouseEventKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginInfo {
    pub server_url: String,
    pub username: String,
    pub password: String,
    pub password_cmd: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginSelection {
    Server,
    Username,
    Password,
    Retry,
}

pub struct LoginWidget<'s> {
    info: &'s mut LoginInfo,
    selection: LoginSelection,
    error: String,
    changed: bool,
}

impl<'s> LoginWidget<'s> {
    pub fn new<'v>(
        info: &'v mut LoginInfo,
        selection: LoginSelection,
        error: String,
    ) -> LoginWidget<'v> {
        LoginWidget {
            info,
            selection,
            error,
            changed: false,
        }
    }
}

#[derive(Debug)]
pub struct LoginEdit {
    pub changed: bool,
}

#[derive(Debug)]
pub enum LoginAction {
    Submit,
    Prev,
    Next,
    Delete,
}

fn get_field(info: &mut LoginInfo, selection: LoginSelection) -> Option<&mut String> {
    match selection {
        LoginSelection::Server => Some(&mut info.server_url),
        LoginSelection::Username => Some(&mut info.username),
        LoginSelection::Password => {
            if info.password_cmd.is_none() {
                Some(&mut info.password)
            } else {
                None
            }
        }
        LoginSelection::Retry => None,
    }
}

impl JellyhajWidget for LoginWidget<'_> {
    type State = LoginEdit;

    type Action = LoginAction;

    type ActionResult = LoginEdit;

    fn min_width(&self) -> Option<u16> {
        Some(18)
    }

    fn min_height(&self) -> Option<u16> {
        Some(19)
    }

    fn into_state(self) -> Self::State {
        LoginEdit {
            changed: self.changed,
        }
    }

    fn accepts_text_input(&self) -> bool {
        !(self.selection == LoginSelection::Retry
            || (self.selection == LoginSelection::Password && self.info.password_cmd.is_some()))
    }

    fn accept_char(&mut self, text: char) {
        if let Some(field) = get_field(self.info, self.selection) {
            field.push(text);
            self.changed = true;
        }
    }

    fn accept_text(&mut self, text: String) {
        if let Some(field) = get_field(self.info, self.selection) {
            field.push_str(&text);
            self.changed = true;
        }
    }

    fn apply_action(
        &mut self,
        _: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        match action {
            LoginAction::Submit => Ok(Some(LoginEdit {
                changed: self.changed,
            })),
            LoginAction::Prev => {
                self.selection = match self.selection {
                    LoginSelection::Server => LoginSelection::Retry,
                    LoginSelection::Username => LoginSelection::Server,
                    LoginSelection::Password => LoginSelection::Username,
                    LoginSelection::Retry => LoginSelection::Password,
                };
                Ok(None)
            }
            LoginAction::Next => {
                self.selection = match self.selection {
                    LoginSelection::Server => LoginSelection::Username,
                    LoginSelection::Username => LoginSelection::Password,
                    LoginSelection::Password => LoginSelection::Retry,
                    LoginSelection::Retry => LoginSelection::Server,
                };
                Ok(None)
            }
            LoginAction::Delete => {
                if let Some(field) = get_field(self.info, self.selection) {
                    field.pop();
                    self.changed = true;
                }
                Ok(None)
            }
        }
    }

    fn click(
        &mut self,
        _: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        position: ratatui::prelude::Position,
        size: ratatui::prelude::Size,
        kind: MouseEventKind,
        _: ratatui::crossterm::event::KeyModifiers,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        if kind == MouseEventKind::Down(MouseButton::Left)
            && position.x >= 1
            && position.x < size.width - 2
        {
            #[allow(non_contiguous_range_endpoints)]
            match position.y {
                2..5 => self.selection = LoginSelection::Server,
                6..9 => self.selection = LoginSelection::Username,
                10..13 => self.selection = LoginSelection::Password,
                14..17 => {
                    return Ok(Some(LoginEdit {
                        changed: self.changed,
                    }));
                }
                _ => {}
            }
        }
        Ok(None)
    }

    fn render_fallible_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        _: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
    ) -> jellyhaj_widgets_core::Result<()> {
        render::render_login(self.info, self.selection, &self.error, area, buf);
        Ok(())
    }
}
