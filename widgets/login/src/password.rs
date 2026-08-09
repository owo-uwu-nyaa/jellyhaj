use std::{convert::Infallible, ops::ControlFlow};

use color_eyre::eyre::Report;
use jellyfin::{JellyfinClient, NoAuth};
use jellyhaj_core::{
    Config,
    keybinds::FormCommand,
    state::{ClientOut, LoginState, Navigation, NextScreen},
    widgets::KeybindAction,
};
use jellyhaj_form_widget::{
    FormAction,
    button::Button,
    form::{FormCommandMapper, FormResultMapper, component::FormComponent},
    form_widget,
    secret_field::SecretField,
    text_field::TextField,
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::{
    ContextRef, JellyhajWidget, JellyhajWidgetBase, JellyhajWidgetExt, Result, Wrapper,
};
use valuable::Valuable;

#[derive(Debug, Clone, Copy)]
pub struct Login;

impl From<Infallible> for Login {
    fn from(_: Infallible) -> Self {
        unimplemented!()
    }
}

#[derive(Debug)]
pub enum PasswordUsage {
    Password(String),
    PasswordCmd(String),
}

#[derive(Debug)]
pub struct LoginResult {
    password: PasswordUsage,
    username: String,
}

pub struct PasswordResultMapper;
impl FormResultMapper<PasswordData> for PasswordResultMapper {
    type Res = LoginResult;

    fn map(
        state: &PasswordData,
        _: <PasswordData as FormComponent>::AR,
        _cx: jellyhaj_widgets_core::WidgetContext<
            '_,
            <PasswordData as FormComponent>::Action,
            impl Wrapper<<PasswordData as FormComponent>::Action>,
            (),
        >,
    ) -> Result<Option<Self::Res>> {
        Ok(Some(LoginResult {
            password: if state.use_password_cmd {
                PasswordUsage::PasswordCmd(state.password_cmd.text.clone())
            } else {
                PasswordUsage::Password(state.password.secret.clone())
            },
            username: state.username.text.clone(),
        }))
    }
}

#[derive(Debug, Valuable)]
#[form_widget("Login", Login, PasswordResultMapper)]
pub struct PasswordData {
    #[form(descr = "Username")]
    username: TextField,
    #[form(descr = "Use password cmd")]
    use_password_cmd: bool,
    #[form(descr = "Password", show_if(!self.use_password_cmd))]
    password: SecretField,
    #[form(descr = "Password cmd (JSON array)", show_if(self.use_password_cmd))]
    password_cmd: TextField,
    #[form(descr = "Login")]
    login: Button<Login>,
}

impl PasswordData {
    #[must_use]
    pub fn new(username: String, password: String, password_cmd_vec: &Vec<String>) -> Self {
        let (use_password_cmd, password_cmd) = if password_cmd_vec.is_empty() {
            (false, "[\"\"]".to_string())
        } else {
            (
                true,
                serde_json::to_string(&password_cmd_vec)
                    .expect("deserializing string vec should never fail!"),
            )
        };
        Self {
            username: TextField::new(username),
            use_password_cmd,
            password: SecretField::new(password),
            password_cmd: TextField::new(password_cmd),
            login: Button::new(Login),
        }
    }
}

#[derive(Valuable)]
pub struct PasswordWidget {
    #[valuable(skip)]
    inner: KeybindWidget<FormCommand, PasswordDataWidget, FormCommandMapper<PasswordDataAction>>,
    #[valuable(skip)]
    client_out: ClientOut,
    state: LoginState,
    server_id: String,
    #[valuable(skip)]
    client: JellyfinClient<NoAuth>,
}

impl PasswordWidget {
    pub const fn new(
        inner: KeybindWidget<
            FormCommand,
            PasswordDataWidget,
            FormCommandMapper<PasswordDataAction>,
        >,
        client_out: ClientOut,
        state: LoginState,
        client: JellyfinClient<NoAuth>,
        server_id: String,
    ) -> Self {
        Self {
            inner,
            client_out,
            state,
            server_id,
            client,
        }
    }
}

#[derive(Debug)]
pub enum PasswordAction {
    Inner(FormAction<PasswordDataAction>),
    Initial,
}

impl JellyhajWidgetBase for PasswordWidget {
    type Action = KeybindAction<PasswordAction>;

    type ActionResult = Navigation;

    const NAME: &str = "login-password";

    fn visit_children(&self, visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        visitor.visit(&self.inner);
    }

    fn min_width(&self) -> Option<u16> {
        None
    }

    fn min_height(&self) -> Option<u16> {
        None
    }
}

#[derive(Clone, Copy)]
struct Wrap;
impl Wrapper<KeybindAction<FormAction<PasswordDataAction>>> for Wrap {
    type F = KeybindAction<PasswordAction>;

    fn wrap(&self, val: KeybindAction<FormAction<PasswordDataAction>>) -> Self::F {
        match val {
            KeybindAction::Inner(v) => KeybindAction::Inner(PasswordAction::Inner(v)),
            KeybindAction::Key(key_event) => KeybindAction::Key(key_event),
        }
    }
}

fn map(
    v: Result<Option<ControlFlow<Navigation, ControlFlow<Navigation, LoginResult>>>>,
    client_out: &ClientOut,
    state: &LoginState,
    client: &JellyfinClient<NoAuth>,
    server_id: &str,
) -> Result<Option<Navigation>> {
    v.map(|v| {
        v.map(|v| match v {
            ControlFlow::Break(n) | ControlFlow::Continue(ControlFlow::Break(n)) => n,
            ControlFlow::Continue(ControlFlow::Continue(LoginResult { password, username })) => {
                let mut state = LoginState {
                    server_url: state.server_url.clone(),
                    username,
                    password: String::new(),
                    passwort_cmd: Vec::new(),
                };
                match password {
                    PasswordUsage::Password(p) => state.password = p,
                    PasswordUsage::PasswordCmd(cmd) => match serde_json::from_str(&cmd) {
                        Ok(v) => state.passwort_cmd = v,
                        Err(e) => {
                            return Navigation::Push(NextScreen::Error(
                                Report::new(e).wrap_err("While parsing password cmd array"),
                            ));
                        }
                    },
                }
                Navigation::Push(NextScreen::AuthPasswordFetch {
                    state,
                    out: client_out.clone(),
                    client: client.clone(),
                    server_id: server_id.to_owned(),
                })
            }
        })
    })
}
impl<R: 'static + ContextRef<Config>> JellyhajWidget<R> for PasswordWidget {
    fn init(
        &mut self,
        cx: jellyhaj_widgets_core::WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) {
        if !(self.state.username.is_empty()
            || (self.state.password.is_empty() && self.state.passwort_cmd.is_empty()))
        {
            self.inner.inner.sel = PasswordDataSelection::Login(());
            cx.submitter
                .spawn_value_infallible(KeybindAction::Inner(PasswordAction::Initial));
        }
        self.inner.init(cx.wrap_with(Wrap));
    }

    fn apply_action(
        &mut self,
        cx: jellyhaj_widgets_core::WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        let inner = match action {
            KeybindAction::Inner(PasswordAction::Initial) => {
                return Ok(Some(Navigation::Push(NextScreen::AuthPasswordFetch {
                    state: self.state.clone(),
                    out: self.client_out.clone(),
                    client: self.client.clone(),
                    server_id: self.server_id.clone(),
                })));
            }
            KeybindAction::Inner(PasswordAction::Inner(v)) => KeybindAction::Inner(v),
            KeybindAction::Key(key_event) => KeybindAction::Key(key_event),
        };
        map(
            self.inner.apply_action(cx.wrap_with(Wrap), inner),
            &self.client_out,
            &self.state,
            &self.client,
            &self.server_id,
        )
    }

    fn click(
        &mut self,
        cx: jellyhaj_widgets_core::WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        position: ratatui::prelude::Position,
        size: ratatui::prelude::Size,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        map(
            self.inner
                .click(cx.wrap_with(Wrap), position, size, kind, modifier),
            &self.client_out,
            &self.state,
            &self.client,
            &self.server_id,
        )
    }

    fn render_fallible_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        cx: jellyhaj_widgets_core::WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()> {
        self.inner.render_fallible(area, buf, cx.wrap_with(Wrap))
    }
}
