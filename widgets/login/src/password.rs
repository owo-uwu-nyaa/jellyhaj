use std::convert::Infallible;

use color_eyre::eyre::Report;
use jellyfin::{JellyfinClient, NoAuth};
use jellyhaj_core::{
    Config,
    keybinds::FormCommand,
    state::{ClientOut, LoginState, Navigation, NextScreen},
};
use jellyhaj_form_widget::{
    FormAction,
    button::Button,
    form::{Form, FormCommandMapper, FormResultMapper, component::FormComponent},
    form_widget,
    secret_field::SecretField,
    text_field::TextField,
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::{
    ContextRef, JellyhajWidgetBase, RenderFlag, Result, Wrapper,
    mapper::{ActionMapper, ActionMapperBase, ActionMapperWidget},
    outer::{Named, UnwrapWidget},
};
use valuable::Valuable;

#[derive(Debug, Clone, Copy)]
pub struct Login;

impl From<Infallible> for Login {
    fn from(_: Infallible) -> Self {
        unimplemented!()
    }
}

pub struct PasswordResultMapper;
impl FormResultMapper<PasswordData> for PasswordResultMapper {
    type Res = Navigation;

    fn map(
        state: &mut Form<PasswordData>,
        _: <PasswordData as FormComponent>::AR,
        _cx: jellyhaj_widgets_core::WidgetContext<
            '_,
            FormAction<<PasswordData as FormComponent>::Action>,
            impl Wrapper<FormAction<<PasswordData as FormComponent>::Action>>,
            (),
        >,
        _render_flag: &mut RenderFlag,
    ) -> Result<Option<Self::Res>> {
        let mut login_state = LoginState {
            server_url: state.data.server_url.clone(),
            username: state.data.username.text.clone(),
            password: String::new(),
            passwort_cmd: Vec::new(),
        };
        if state.data.use_password_cmd {
            match serde_json::from_str(&state.data.password_cmd.text) {
                Ok(v) => login_state.passwort_cmd = v,
                Err(e) => {
                    return Ok(Some(Navigation::Push(NextScreen::Error(
                        Report::new(e).wrap_err("While parsing password cmd array"),
                    ))));
                }
            }
        } else {
            login_state.password.clone_from(&state.data.password.secret);
        }
        Ok(Some(Navigation::Push(NextScreen::AuthPasswordFetch {
            state: login_state,
            out: state.data.client_out.clone(),
            client: state.data.client.clone(),
            server_id: state.data.server_id.clone(),
        })))
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
    #[form(skip)]
    #[valuable(skip)]
    client_out: ClientOut,
    #[form(skip)]
    server_url: String,
    #[form(skip)]
    server_id: String,
    #[form(skip)]
    password_cmd_vec: Vec<String>,
    #[form(skip)]
    #[valuable(skip)]
    client: JellyfinClient<NoAuth>,
}

impl PasswordData {
    #[must_use]
    pub fn new(
        state: LoginState,
        server_id: String,
        client_out: ClientOut,
        client: JellyfinClient<NoAuth>,
    ) -> Self {
        let (use_password_cmd, password_cmd) = if state.passwort_cmd.is_empty() {
            (false, "[\"\"]".to_string())
        } else {
            (
                true,
                serde_json::to_string(&state.passwort_cmd)
                    .expect("deserializing string vec should never fail!"),
            )
        };
        Self {
            username: TextField::new(state.username),
            use_password_cmd,
            password: SecretField::new(state.password),
            password_cmd: TextField::new(password_cmd),
            login: Button::new(Login),
            server_url: state.server_url,
            client_out,
            server_id,
            password_cmd_vec: state.passwort_cmd,
            client,
        }
    }
}

type InnerWidget = UnwrapWidget<
    KeybindWidget<
        FormCommand,
        UnwrapWidget<PasswordDataWidget>,
        FormCommandMapper<PasswordDataAction>,
    >,
>;

pub struct Name;
impl Named for Name {
    const NAME: &str = "login-password";
}

#[derive(Debug)]
pub struct Initial;
#[derive(Valuable)]
pub struct PasswordActionMapper;

impl ActionMapperBase<InnerWidget> for PasswordActionMapper {
    type Action = Initial;
}

impl<R: ContextRef<Config> + 'static> ActionMapper<R, InnerWidget> for PasswordActionMapper {
    fn init(
        &mut self,
        this: &mut InnerWidget,
        cx: jellyhaj_widgets_core::WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        _this_cx: jellyhaj_widgets_core::WidgetContext<
            '_,
            <InnerWidget as JellyhajWidgetBase>::Action,
            impl Wrapper<<InnerWidget as JellyhajWidgetBase>::Action>,
            R,
        >,
    ) {
        let this = &mut this.inner.inner.inner;
        if !(this.data.username.text.is_empty()
            || (this.data.password.secret.is_empty() && this.data.password_cmd_vec.is_empty()))
        {
            this.sel = PasswordDataSelection::Login(());
            cx.submitter.spawn_value_infallible(Initial);
        }
    }

    fn map_action(
        &mut self,
        this: &mut InnerWidget,
        _cx: jellyhaj_widgets_core::WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        _this_cx: jellyhaj_widgets_core::WidgetContext<
            '_,
            <InnerWidget as JellyhajWidgetBase>::Action,
            impl Wrapper<<InnerWidget as JellyhajWidgetBase>::Action>,
            R,
        >,
        _action: Self::Action,
        _render_flag: &mut jellyhaj_widgets_core::RenderFlag,
    ) -> Result<Option<<InnerWidget as JellyhajWidgetBase>::ActionResult>> {
        let state = &this.inner.inner.inner.data;
        let mut login_state = LoginState {
            server_url: state.server_url.clone(),
            username: state.username.text.clone(),
            password: String::new(),
            passwort_cmd: Vec::new(),
        };
        if state.use_password_cmd {
            match serde_json::from_str(&state.password_cmd.text) {
                Ok(v) => login_state.passwort_cmd = v,
                Err(e) => {
                    return Ok(Some(Navigation::Push(NextScreen::Error(
                        Report::new(e).wrap_err("While parsing password cmd array"),
                    ))));
                }
            }
        } else {
            login_state.password.clone_from(&state.password.secret);
        }
        Ok(Some(Navigation::Push(NextScreen::AuthPasswordFetch {
            state: login_state,
            out: state.client_out.clone(),
            client: state.client.clone(),
            server_id: state.server_id.clone(),
        })))
    }
}

pub type PasswordWidget = ActionMapperWidget<Name, InnerWidget, PasswordActionMapper>;
