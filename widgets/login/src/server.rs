use std::{convert::Infallible, ops::ControlFlow};

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
    text_field::TextField,
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::{
    ContextRef, JellyhajWidget, JellyhajWidgetBase, JellyhajWidgetExt, Result, Wrapper,
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
    type Res = String;

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
        Ok(Some(state.server_url.text.clone()))
    }
}

#[form_widget("Connect to Jellyfin Server", Connect, ServerResultMapper)]
#[derive(Debug, Valuable)]
pub struct ServerData {
    #[form(descr = "Jellyfin URL")]
    server_url: TextField,
    #[form(descr = "Connect")]
    connect: Button<Connect>,
}

impl ServerData {
    #[must_use]
    pub const fn new(url: String) -> Self {
        Self {
            server_url: TextField::new(url),
            connect: Button::new(Connect),
        }
    }
}

#[derive(Valuable)]
pub struct ServerWidget {
    #[valuable(skip)]
    inner: KeybindWidget<FormCommand, ServerDataWidget, FormCommandMapper<ServerDataAction>>,
    #[valuable(skip)]
    client_out: ClientOut,
    state: LoginState,
}

impl ServerWidget {
    pub const fn new(
        inner: KeybindWidget<FormCommand, ServerDataWidget, FormCommandMapper<ServerDataAction>>,
        client_out: ClientOut,
        state: LoginState,
    ) -> Self {
        Self {
            inner,
            client_out,
            state,
        }
    }
}

#[derive(Debug)]
pub enum ServerAction {
    Inner(FormAction<ServerDataAction>),
    Initial,
}

impl JellyhajWidgetBase for ServerWidget {
    type Action = KeybindAction<ServerAction>;

    type ActionResult = Navigation;

    const NAME: &str = "server-url";

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
impl Wrapper<KeybindAction<FormAction<ServerDataAction>>> for Wrap {
    type F = KeybindAction<ServerAction>;

    fn wrap(&self, val: KeybindAction<FormAction<ServerDataAction>>) -> Self::F {
        match val {
            KeybindAction::Inner(v) => KeybindAction::Inner(ServerAction::Inner(v)),
            KeybindAction::Key(key_event) => KeybindAction::Key(key_event),
        }
    }
}

fn map(
    v: Result<Option<ControlFlow<Navigation, ControlFlow<Navigation, String>>>>,
    client_out: &ClientOut,
    state: &LoginState,
) -> Result<Option<Navigation>> {
    v.map(|v| {
        v.map(|v| match v {
            ControlFlow::Break(n) | ControlFlow::Continue(ControlFlow::Break(n)) => n,
            ControlFlow::Continue(ControlFlow::Continue(server)) => {
                Navigation::Push(NextScreen::ConnectToServer {
                    state: LoginState {
                        server_url: server,
                        username: state.username.clone(),
                        password: state.password.clone(),
                        passwort_cmd: state.passwort_cmd.clone(),
                    },
                    out: client_out.clone(),
                })
            }
        })
    })
}
impl<R: 'static + ContextRef<Config>> JellyhajWidget<R> for ServerWidget {
    fn init(
        &mut self,
        cx: jellyhaj_widgets_core::WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) {
        if !self.state.server_url.is_empty() {
            self.inner.inner.sel = ServerDataSelection::Connect(());
            cx.submitter
                .spawn_value_infallible(KeybindAction::Inner(ServerAction::Initial));
        }
        self.inner.init(cx.wrap_with(Wrap));
    }

    fn apply_action(
        &mut self,
        cx: jellyhaj_widgets_core::WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        let inner = match action {
            KeybindAction::Inner(ServerAction::Initial) => {
                return Ok(Some(Navigation::Push(NextScreen::ConnectToServer {
                    state: self.state.clone(),
                    out: self.client_out.clone(),
                })));
            }
            KeybindAction::Inner(ServerAction::Inner(v)) => KeybindAction::Inner(v),
            KeybindAction::Key(key_event) => KeybindAction::Key(key_event),
        };
        map(
            self.inner.apply_action(cx.wrap_with(Wrap), inner),
            &self.client_out,
            &self.state,
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
