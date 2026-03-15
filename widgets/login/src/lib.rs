use std::{convert::Infallible, ops::ControlFlow};

use jellyhaj_core::Config;
use jellyhaj_core::{keybinds::FormCommand, state::Navigation};
use jellyhaj_form_widget::button::{ActionCreator, Button};
use jellyhaj_form_widget::form_widget;
use jellyhaj_form_widget::label::Label;
use jellyhaj_form_widget::{
    form::{FormCommandMapper, FormData},
    label_block::LabelBlock,
    secret_field::SecretField,
    text_field::TextField,
};
use jellyhaj_keybinds_widget::{KeybindState, KeybindWidget};
use jellyhaj_widgets_core::{
    ContextRef, GetFromContext, KeyModifiers, MouseEventKind, Result, WidgetContext, Wrapper,
};
use jellyhaj_widgets_core::{JellyhajWidget, JellyhajWidgetState, flatten::FlattenState};
use ratatui::prelude::{Buffer, Position, Rect, Size};

#[derive(Debug)]
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
#[derive(Debug)]
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

type InnerState<R> = FlattenState<
    R,
    Navigation,
    Submit,
    KeybindState<R, FormCommand, LoginDataState, FormCommandMapper<LoginDataAction>>,
>;

type InnerWidget<R> = <InnerState<R> as JellyhajWidgetState<R>>::Widget;

pub struct LoginState<R: ContextRef<Config> + 'static> {
    inner: InnerState<R>,
}

impl<R: ContextRef<Config> + 'static> std::fmt::Debug for LoginState<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoginState")
            .field("inner", &self.inner)
            .finish()
    }
}

pub struct LoginWidget<R: ContextRef<Config> + 'static> {
    inner: InnerWidget<R>,
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

impl<R: ContextRef<Config> + 'static> JellyhajWidgetState<R> for LoginState<R> {
    type Action = <InnerState<R> as JellyhajWidgetState<R>>::Action;

    type ActionResult = LoginResult;

    type Widget = LoginWidget<R>;

    const NAME: &str = "login";

    fn visit_children(visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        visitor.visit::<R, InnerState<R>>();
    }

    fn into_widget(self, cx: &R) -> Self::Widget {
        LoginWidget {
            inner: self.inner.into_widget(cx),
        }
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        self.inner.apply_action(cx, action).map(|v| {
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

impl<R: ContextRef<Config> + 'static> JellyhajWidget<R> for LoginWidget<R> {
    type Action = <InnerWidget<R> as JellyhajWidget<R>>::Action;

    type ActionResult = LoginResult;

    type State = LoginState<R>;

    fn min_width(&self) -> Option<u16> {
        self.inner.min_width()
    }

    fn min_height(&self) -> Option<u16> {
        self.inner.min_height()
    }

    fn into_state(self) -> Self::State {
        LoginState {
            inner: self.inner.into_state(),
        }
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

impl<R: ContextRef<Config> + 'static> LoginWidget<R> {
    fn map_res(
        &mut self,
        res: Result<Option<<InnerWidget<R> as JellyhajWidget<R>>::ActionResult>>,
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

    pub fn new(
        server_url: String,
        username: String,
        password: String,
        password_cmd_set: bool,
        error: String,
        cx: &R,
    ) -> Self {
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
        let data = data.make_state_with(if server_unset {
            LoginDataSelection::ServerUrl(())
        } else {
            LoginDataSelection::Submit(())
        });
        let keybinds = KeybindWidget::new(
            data.into_widget(),
            Config::get_ref(cx).keybinds.form.clone(),
            FormCommandMapper::<LoginDataAction>::default(),
        );
        Self {
            inner: InnerWidget::new(keybinds),
        }
    }
}
