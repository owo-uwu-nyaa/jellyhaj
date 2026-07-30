pub mod password;
pub mod quick_connect;
pub mod select;
pub mod server;

use std::{convert::Infallible, ops::ControlFlow};

use color_eyre::Report;
use jellyhaj_core::Config;
use jellyhaj_core::{keybinds::FormCommand, state::Navigation};
use jellyhaj_form_widget::button::Button;
use jellyhaj_form_widget::form::{FormData, FormResultMapper};
use jellyhaj_form_widget::form_widget;
use jellyhaj_form_widget::label::Label;
use jellyhaj_form_widget::{
    form::FormCommandMapper, label_block::LabelBlock, secret_field::SecretField,
    text_field::TextField,
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::flatten::FlattenWidget;
use jellyhaj_widgets_core::valuable::Valuable;
use jellyhaj_widgets_core::{
    ContextRef, KeyModifiers, MouseEventKind, Result, WidgetContext, Wrapper,
};
use jellyhaj_widgets_core::{JellyhajWidget, JellyhajWidgetBase};
use ratatui::prelude::{Buffer, Position, Rect, Size};

#[derive(Debug, Clone, Copy)]
pub enum ButtonAction {
    Submit,
    QuickConnect,
}

impl From<Infallible> for ButtonAction {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

pub struct LoginResultMapper;

impl FormResultMapper<LoginData> for LoginResultMapper {
    type Res = LoginType;

    fn map(
        state: &LoginData,
        form_result: <LoginData as jellyhaj_form_widget::form::FormDataTypes>::AR,
        _cx: WidgetContext<
            '_,
            <LoginData as jellyhaj_form_widget::form::FormDataTypes>::Action,
            impl Wrapper<<LoginData as jellyhaj_form_widget::form::FormDataTypes>::Action>,
            (),
        >,
    ) -> Result<Option<Self::Res>> {
        Ok(Some(match form_result {
            ButtonAction::Submit => LoginType::Password {
                server_url: state.server_url.text.clone(),
                username: state.username.text.clone(),
                password: state.password.secret.clone(),
            },
            ButtonAction::QuickConnect => LoginType::QuickConnect {
                server_url: state.server_url.text.clone(),
            },
        }))
    }
}

#[form_widget(
    "Enter Jellyfin Server / Login Information",
    ButtonAction,
    LoginResultMapper
)]
#[derive(Debug, Valuable)]
pub struct LoginData {
    #[descr("Jellyfin URL")]
    server_url: TextField,
    #[descr("Username")]
    username: TextField,
    #[descr("Password")]
    #[show_if(!self.password_cmd)]
    password: SecretField,
    #[descr("Password already set through command in login config")]
    #[show_if(self.password_cmd)]
    password_set: Label,
    #[skip]
    password_cmd: bool,
    #[descr("Login")]
    submit: Button<ButtonAction>,
    #[descr("Login with Quick Connect")]
    quick_connect: Button<ButtonAction>,
    #[descr("Error")]
    error: LabelBlock,
}

type InnerWidget =
    FlattenWidget<KeybindWidget<FormCommand, LoginDataWidget, FormCommandMapper<LoginDataAction>>>;

#[derive(Valuable)]
pub struct LoginWidget {
    #[valuable(skip)]
    inner: InnerWidget,
}

#[derive(Debug)]
pub enum LoginType {
    Password {
        server_url: String,
        username: String,
        password: String,
    },
    QuickConnect {
        server_url: String,
    },
}

impl LoginType {
    #[must_use]
    pub fn get_server_url(&self) -> &str {
        match self {
            Self::Password {
                server_url,
                username: _,
                password: _,
            }
            | Self::QuickConnect { server_url } => server_url,
        }
    }
}

impl JellyhajWidgetBase for LoginWidget {
    type Action = <InnerWidget as JellyhajWidgetBase>::Action;

    type ActionResult = ControlFlow<Navigation, LoginType>;

    const NAME: &str = "login";

    fn visit_children(&self, visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        visitor.visit(&self.inner);
    }

    fn min_width(&self) -> Option<u16> {
        self.inner.min_width()
    }

    fn min_height(&self) -> Option<u16> {
        self.inner.min_height()
    }

    fn accepts_text_input(&self) -> bool {
        self.inner.accepts_text_input()
    }

    fn accept_char(&mut self, text: char) {
        self.inner.accept_char(text);
    }

    fn accept_text(&mut self, text: String) {
        self.inner.accept_text(text);
    }
}

impl<R: ContextRef<Config> + 'static> JellyhajWidget<R> for LoginWidget {
    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {
        self.inner.init(cx);
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        self.inner.apply_action(cx, action)
    }

    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        position: Position,
        size: Size,
        kind: MouseEventKind,
        modifier: KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        self.inner.click(cx, position, size, kind, modifier)
    }

    fn render_fallible_inner(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()> {
        self.inner.render_fallible_inner(area, buf, cx)
    }
}

impl LoginWidget {
    pub fn new(
        server_url: String,
        username: String,
        password: String,
        password_cmd_set: bool,
        error: &Report,
        c: &Config,
    ) -> Self {
        let selection = if server_url.is_empty() {
            LoginDataSelection::ServerUrl(())
        } else if username.is_empty() {
            LoginDataSelection::Username(())
        } else if !password_cmd_set && password.is_empty() {
            LoginDataSelection::Password(())
        } else {
            LoginDataSelection::Submit(())
        };
        Self {
            inner: FlattenWidget::new(KeybindWidget::new(
                LoginData {
                    server_url: TextField::new(server_url),
                    username: TextField::new(username),
                    password: SecretField::new(password),
                    password_set: Label,
                    password_cmd: password_cmd_set,
                    submit: Button::new(ButtonAction::Submit),
                    quick_connect: Button::new(ButtonAction::QuickConnect),
                    error: LabelBlock::new(format!("{error:?}")),
                }
                .make_with(selection),
                c.keybinds.form.clone(),
                FormCommandMapper::default(),
            )),
        }
    }
}
