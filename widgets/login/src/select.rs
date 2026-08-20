use std::convert::Infallible;

use jellyfin::{JellyfinClient, NoAuth};
use jellyhaj_core::{
    Config,
    keybinds::FormCommand,
    state::{ClientOut, LoginState, Navigation, NextScreen},
};
use jellyhaj_form_widget::{
    button::Button,
    form::{FormCommandMapper, FormResultMapper, component::FormComponent},
    form_widget,
    label::Label,
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::{
    ContextRef, JellyhajWidgetBase, Result, Wrapper,
    mapper::{ActionMapper, ActionMapperBase, ActionMapperWidget},
    outer::{Named, UnwrapWidget},
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

pub struct SelectResultMapper;

impl FormResultMapper<SelectData> for SelectResultMapper {
    type Res = Navigation;

    fn map(
        state: &SelectData,
        form_result: <SelectData as FormComponent>::AR,
        _cx: jellyhaj_widgets_core::WidgetContext<
            '_,
            <SelectData as FormComponent>::Action,
            impl Wrapper<<SelectData as FormComponent>::Action>,
            (),
        >,
    ) -> Result<Option<Self::Res>> {
        let screen = match form_result {
            Selection::QuickConnect => NextScreen::AuthQuickConnectFetch {
                state: state.state.clone(),
                out: state.client_out.clone(),
                client: state.client.clone(),
                server_id: state.server_id.clone(),
            },
            Selection::Password => NextScreen::AuthPassword {
                state: state.state.clone(),
                out: state.client_out.clone(),
                client: state.client.clone(),
                server_id: state.server_id.clone(),
            },
        };
        Ok(Some(Navigation::Push(screen)))
    }
}

#[form_widget("Select login method", Selection, SelectResultMapper)]
#[derive(Debug, Valuable)]
pub struct SelectData {
    #[form(descr = "Quick Connect")]
    #[form(show_if(self.quick_connect_status))]
    quick_connect: Button<Selection>,
    #[form(descr = "Quick Connect is disabled")]
    #[form(show_if(!self.quick_connect_status))]
    quick_connect_disabled: Label,
    #[form(descr = "Passwort")]
    passwort: Button<Selection>,
    #[form(skip)]
    quick_connect_status: bool,
    #[form(skip)]
    #[valuable(skip)]
    client_out: ClientOut,
    #[form(skip)]
    state: LoginState,
    #[form(skip)]
    server_id: String,
    #[form(skip)]
    #[valuable(skip)]
    client: JellyfinClient<NoAuth>,
}

impl SelectData {
    #[must_use]
    pub const fn new(
        client_out: ClientOut,
        state: LoginState,
        client: JellyfinClient<NoAuth>,
        quick_connect_status: bool,
        server_id: String,
    ) -> Self {
        Self {
            quick_connect: Button::new(Selection::QuickConnect),
            quick_connect_disabled: Label,
            passwort: Button::new(Selection::Password),
            quick_connect_status,
            client_out,
            state,
            server_id,
            client,
        }
    }
}

type InnerWidget = UnwrapWidget<
    KeybindWidget<FormCommand, UnwrapWidget<SelectDataWidget>, FormCommandMapper<SelectDataAction>>,
>;

#[derive(Valuable)]
pub struct SelectActionMapper;

#[derive(Debug)]
pub struct Initial;

impl ActionMapperBase<InnerWidget> for SelectActionMapper {
    type Action = Initial;
}

impl<R: ContextRef<Config> + 'static> ActionMapper<R, InnerWidget> for SelectActionMapper {
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
        if !(this.data.state.username.is_empty()
            || (this.data.state.password.is_empty() && this.data.state.passwort_cmd.is_empty()))
        {
            this.sel = SelectDataSelection::Passwort(());
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
        Ok(Some(Navigation::Push(NextScreen::AuthPassword {
            state: state.state.clone(),
            out: state.client_out.clone(),
            client: state.client.clone(),
            server_id: state.server_id.clone(),
        })))
    }
}

pub struct Name;
impl Named for Name {
    const NAME: &str = "login-select-method";
}

pub type SelectWidget = ActionMapperWidget<Name, InnerWidget, SelectActionMapper>;
