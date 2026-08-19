use std::convert::Infallible;

use jellyhaj_core::{
    Config,
    keybinds::FormCommand,
    state::{ClientOut, LoginState, Navigation, NextScreen},
};
use jellyhaj_form_widget::{
    button::Button,
    form::{FormCommandMapper, FormResultMapper, component::FormComponent},
    form_widget,
    text_field::TextField,
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::{
    ContextRef, JellyhajWidgetBase, Result, Wrapper,
    mapper::{ActionMapper, ActionMapperBase, ActionMapperWidget},
    outer::{Named, UnwrapWidget},
};
use valuable::Valuable;

#[derive(Debug, Clone, Copy)]
pub struct Connect;

impl From<Infallible> for Connect {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

pub struct ServerResultMapper;
impl FormResultMapper<ServerData> for ServerResultMapper {
    type Res = Navigation;

    fn map(
        state: &ServerData,
        _form_result: <ServerData as FormComponent>::AR,
        _cx: jellyhaj_widgets_core::WidgetContext<
            '_,
            <ServerData as FormComponent>::Action,
            impl Wrapper<<ServerData as FormComponent>::Action>,
            (),
        >,
    ) -> Result<Option<Self::Res>> {
        Ok(Some(Navigation::Push(NextScreen::ConnectToServer {
            state: LoginState {
                server_url: state.server_url.text.clone(),
                username: state.state.username.clone(),
                password: state.state.password.clone(),
                passwort_cmd: state.state.passwort_cmd.clone(),
            },
            out: state.client_out.clone(),
        })))
    }
}

#[form_widget("Connect to Jellyfin Server", Connect, ServerResultMapper)]
#[derive(Debug, Valuable)]
pub struct ServerData {
    #[form(descr = "Jellyfin URL")]
    server_url: TextField,
    #[form(descr = "Connect")]
    connect: Button<Connect>,
    #[form(skip)]
    state: LoginState,
    #[valuable(skip)]
    #[form(skip)]
    client_out: ClientOut,
}

impl ServerData {
    #[must_use]
    pub fn new(state: LoginState, client_out: ClientOut) -> Self {
        let server_url = state.server_url.clone();
        Self {
            server_url: TextField::new(server_url),
            connect: Button::new(Connect),
            state,
            client_out,
        }
    }
}

type InnerWidget = UnwrapWidget<
    KeybindWidget<FormCommand, UnwrapWidget<ServerDataWidget>, FormCommandMapper<ServerDataAction>>,
>;

#[derive(Valuable)]
pub struct ServerMapper;
#[derive(Debug)]
pub struct Initial;

impl ActionMapperBase<InnerWidget> for ServerMapper {
    type Action = Initial;
}

impl<R: ContextRef<Config> + 'static> ActionMapper<R, InnerWidget> for ServerMapper {
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
        if !this.inner.inner.inner.data.server_url.text.is_empty() {
            this.inner.inner.inner.sel = ServerDataSelection::Connect(());
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
        Ok(Some(Navigation::Push(NextScreen::ConnectToServer {
            state: this.inner.inner.inner.data.state.clone(),
            out: this.inner.inner.inner.data.client_out.clone(),
        })))
    }
}

pub struct Name;
impl Named for Name {
    const NAME: &str = "server-url";
}

pub type ServerWidget = ActionMapperWidget<Name, InnerWidget, ServerMapper>;
