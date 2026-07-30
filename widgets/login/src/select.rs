use std::{convert::Infallible, ops::ControlFlow};

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
    form::{FormCommandMapper, IdFormResultMapper},
    form_widget,
    label::Label,
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::{
    ContextRef, JellyhajWidget, JellyhajWidgetBase, JellyhajWidgetExt, Result, Wrapper,
};
use valuable::Valuable;

#[derive(Debug, Clone, Copy)]
pub enum Selection {
    QuickConnect,
    Password,
}

impl From<Infallible> for Selection {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

#[form_widget("Select login method", Selection, IdFormResultMapper)]
#[derive(Debug, Valuable)]
pub struct SelectData {
    #[descr("Quick Connect")]
    #[show_if(self.quick_connect_status)]
    quick_connect: Button<Selection>,
    #[descr("Quick Connect is disabled")]
    #[show_if(!self.quick_connect_status)]
    quick_connect_disabled: Label,
    #[descr("Passwort")]
    passwort: Button<Selection>,
    #[skip]
    quick_connect_status: bool,
}

impl SelectData {
    #[must_use]
    pub const fn new(quick_connect_status: bool) -> Self {
        Self {
            quick_connect: Button::new(Selection::QuickConnect),
            quick_connect_disabled: Label,
            passwort: Button::new(Selection::Password),
            quick_connect_status,
        }
    }
}

#[derive(Valuable)]
pub struct SelectWidget {
    #[valuable(skip)]
    inner: KeybindWidget<FormCommand, SelectDataWidget, FormCommandMapper<SelectDataAction>>,
    #[valuable(skip)]
    client_out: ClientOut,
    state: LoginState,
    server_id: String,
    #[valuable(skip)]
    client: JellyfinClient<NoAuth>,
}

impl SelectWidget {
    pub const fn new(
        inner: KeybindWidget<FormCommand, SelectDataWidget, FormCommandMapper<SelectDataAction>>,
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
pub enum SelectAction {
    Inner(FormAction<SelectDataAction>),
    Initial,
}

impl JellyhajWidgetBase for SelectWidget {
    type Action = KeybindAction<SelectAction>;

    type ActionResult = Navigation;

    const NAME: &str = "login-select-method";

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
impl Wrapper<KeybindAction<FormAction<SelectDataAction>>> for Wrap {
    type F = KeybindAction<SelectAction>;

    fn wrap(&self, val: KeybindAction<FormAction<SelectDataAction>>) -> Self::F {
        match val {
            KeybindAction::Inner(v) => KeybindAction::Inner(SelectAction::Inner(v)),
            KeybindAction::Key(key_event) => KeybindAction::Key(key_event),
        }
    }
}

fn map(
    v: Result<Option<ControlFlow<Navigation, ControlFlow<Navigation, Selection>>>>,
    client_out: &ClientOut,
    state: &LoginState,
    client: &JellyfinClient<NoAuth>,
    server_id: &str,
) -> Result<Option<Navigation>> {
    v.map(|v| {
        v.map(|v| match v {
            ControlFlow::Break(n) | ControlFlow::Continue(ControlFlow::Break(n)) => n,
            ControlFlow::Continue(ControlFlow::Continue(Selection::QuickConnect)) => {
                Navigation::Push(NextScreen::AuthQuickConnectFetch {
                    state: state.clone(),
                    out: client_out.clone(),
                    client: client.clone(),
                    server_id: server_id.to_owned(),
                })
            }
            ControlFlow::Continue(ControlFlow::Continue(Selection::Password)) => {
                Navigation::Push(NextScreen::AuthPassword {
                    state: state.clone(),
                    out: client_out.clone(),
                    client: client.clone(),
                    server_id: server_id.to_owned(),
                })
            }
        })
    })
}
impl<R: 'static + ContextRef<Config>> JellyhajWidget<R> for SelectWidget {
    fn init(
        &mut self,
        cx: jellyhaj_widgets_core::WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) {
        if !(self.state.username.is_empty()
            || (self.state.password.is_empty() && self.state.passwort_cmd.is_empty()))
        {
            self.inner.inner.sel = SelectDataSelection::Passwort(());
            cx.submitter
                .spawn_value_infallible(KeybindAction::Inner(SelectAction::Initial));
        }
        self.inner.init(cx.wrap_with(Wrap));
    }

    fn apply_action(
        &mut self,
        cx: jellyhaj_widgets_core::WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        let inner = match action {
            KeybindAction::Inner(SelectAction::Initial) => {
                return Ok(Some(Navigation::Push(NextScreen::AuthPassword {
                    state: self.state.clone(),
                    out: self.client_out.clone(),
                    client: self.client.clone(),
                    server_id: self.server_id.clone(),
                })));
            }
            KeybindAction::Inner(SelectAction::Inner(v)) => KeybindAction::Inner(v),
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
