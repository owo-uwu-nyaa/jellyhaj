use std::{convert::Infallible, ops::ControlFlow};

use color_eyre::Report;
use jellyhaj_core::Config;
use jellyhaj_core::{keybinds::FormCommand, state::Navigation};
use jellyhaj_form_widget::button::{ActionCreator, Button};
use jellyhaj_form_widget::form::FormData;
use jellyhaj_form_widget::form_widget;
use jellyhaj_form_widget::label::Label;
use jellyhaj_form_widget::{
    form::FormCommandMapper, label_block::LabelBlock, secret_field::SecretField,
    text_field::TextField,
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::JellyhajWidget;
use jellyhaj_widgets_core::flatten::FlattenWidget;
use jellyhaj_widgets_core::valuable::Valuable;
use jellyhaj_widgets_core::{
    ContextRef, GetFromContext, KeyModifiers, MouseEventKind, Result, WidgetContext, Wrapper,
};
use ratatui::prelude::{Buffer, Position, Rect, Size};

#[derive(Debug, Default)]
pub struct Submit;
impl From<Infallible> for Submit {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}
impl ActionCreator for Submit {
    type T = Self;

    fn make_action(&self) -> Self::T {
        Self
    }
}

#[form_widget("Enter Jellyfin Server / Login Information", Submit)]
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
    submit: Button<Submit>,
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
pub enum LoginResult {
    Quit,
    Data {
        server_url: String,
        username: String,
        password: String,
    },
}

impl<R: ContextRef<Config> + 'static> JellyhajWidget<R> for LoginWidget {
    type Action = <InnerWidget as JellyhajWidget<R>>::Action;

    type ActionResult = LoginResult;

    const NAME: &str = "login";

    fn visit_children(&self, visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        visitor.visit::<R, InnerWidget>(&self.inner);
    }

    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {
        self.inner.init(cx);
    }

    fn min_width(&self) -> Option<u16> {
        JellyhajWidget::<R>::min_width(&self.inner)
    }

    fn min_height(&self) -> Option<u16> {
        JellyhajWidget::<R>::min_height(&self.inner)
    }

    fn accepts_text_input(&self) -> bool {
        JellyhajWidget::<R>::accepts_text_input(&self.inner)
    }

    fn accept_char(&mut self, text: char) {
        JellyhajWidget::<R>::accept_char(&mut self.inner, text);
    }

    fn accept_text(&mut self, text: String) {
        JellyhajWidget::<R>::accept_text(&mut self.inner, text);
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        let res = self.inner.apply_action(cx, action);
        self.map_res(res)
    }

    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        position: Position,
        size: Size,
        kind: MouseEventKind,
        modifier: KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        let res = self.inner.click(cx, position, size, kind, modifier);
        self.map_res(res)
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

pub fn new_login_widget<R: ContextRef<Config> + 'static>(
    server_url: String,
    username: String,
    password: String,
    password_cmd_set: bool,
    error: String,
    cx: &R,
) -> LoginWidget {
    let server_unset = server_url.is_empty();
    let data = LoginData {
        server_url: TextField { text: server_url },
        username: TextField { text: username },
        password: SecretField { secret: password },
        password_cmd: password_cmd_set,
        password_set: Label,
        submit: Button::new(Submit),
        error: LabelBlock { text: error },
    };
    let data = data.make_with(if server_unset {
        LoginDataSelection::ServerUrl(())
    } else {
        LoginDataSelection::Submit(())
    });
    let keybinds = KeybindWidget::new(
        data,
        Config::get_ref(cx).keybinds.form.clone(),
        FormCommandMapper::<LoginDataAction>::default(),
    );
    LoginWidget {
        inner: InnerWidget::new(keybinds),
    }
}

impl LoginWidget {
    pub fn new(
        server_url: String,
        username: String,
        password: String,
        password_cmd_set: bool,
        error: Report,
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
        LoginWidget {
            inner: FlattenWidget::new(KeybindWidget::new(
                LoginData {
                    server_url: TextField::new(server_url),
                    username: TextField::new(username),
                    password: SecretField::new(password),
                    password_set: Default::default(),
                    password_cmd: password_cmd_set,
                    submit: Default::default(),
                    error: LabelBlock::new(format!("{error:?}")),
                }
                .make_with(selection),
                c.keybinds.form.clone(),
                Default::default(),
            )),
        }
    }

    fn map_res(
        &mut self,
        res: Result<Option<ControlFlow<Navigation, Submit>>>,
    ) -> Result<Option<LoginResult>, color_eyre::eyre::Error> {
        res.map(|v| {
            v.map(|v| match v {
                ControlFlow::Continue(_) => LoginResult::Data {
                    server_url: self.inner.inner.inner.data.server_url.text.clone(),
                    username: self.inner.inner.inner.data.username.text.clone(),
                    password: self.inner.inner.inner.data.password.secret.clone(),
                },
                ControlFlow::Break(_) => LoginResult::Quit,
            })
        })
    }
}
