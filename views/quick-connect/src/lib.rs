use std::convert::Infallible;

use color_eyre::eyre::Context;
use jellyhaj_context::TuiContext;
use jellyhaj_core::{
    state::{Navigation, NextScreen::QuickConnectAuth},
    widgets::shaded::widget::{Erased, make_new_erased},
};
use jellyhaj_fetch_view::make_nav_fetch;
use jellyhaj_form_widget::{
    FormAction,
    button::Button,
    form::{Form, FormCommandMapper, FormDataExt, FormResultMapper, component::FormComponent},
    form_widget,
    text_field::TextField,
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::{
    RenderFlag, WidgetContext, Wrapper,
    outer::{Named, OuterWidget, UnwrapWidget},
    valuable::Valuable,
};

#[derive(Debug, Clone, Copy, Default)]
enum Action {
    #[default]
    Login,
}
impl From<Infallible> for Action {
    fn from(_value: Infallible) -> Self {
        unreachable!()
    }
}

struct Mapper;

impl FormResultMapper<QuickConnect> for Mapper {
    type Res = Navigation;

    fn map(
        state: &mut Form<QuickConnect>,
        form_result: <QuickConnect as FormComponent>::AR,
        _cx: WidgetContext<
            '_,
            FormAction<<QuickConnect as FormComponent>::Action>,
            impl Wrapper<FormAction<<QuickConnect as FormComponent>::Action>>,
            (),
        >,
        _render_flag: &mut RenderFlag,
    ) -> jellyhaj_widgets_core::Result<Option<Self::Res>> {
        let Action::Login = form_result;
        Ok(Some(Navigation::Replace(QuickConnectAuth(
            state.data.code.text.clone(),
        ))))
    }
}

#[derive(Debug, Default, Valuable)]
#[form_widget("Authorize with Quick Connect", Action, Mapper)]
struct QuickConnect {
    #[form(descr = "Code")]
    code: TextField,
    #[form(descr = "Authenticate")]
    auth: Button<Action>,
}

struct Name;
impl Named for Name {
    const NAME: &str = "quick-connect";
}

pub fn make_quick_connect(cx: TuiContext) -> Erased {
    let widget = OuterWidget::<Name, _>::new(KeybindWidget::new(
        UnwrapWidget::new(QuickConnect::default().make_with_default()),
        cx.config.keybinds.form.clone(),
        FormCommandMapper::default(),
    ));
    make_new_erased(cx, widget)
}

pub fn make_quick_connect_auth(cx: TuiContext, code: String) -> Erased {
    let jellyfin = cx.jellyfin.clone();
    let fut = async move {
        jellyfin
            .authorize_quick_connect(&code)
            .await
            .context("authorizing via quick connect")?;
        Ok(Navigation::PopContext)
    };
    make_nav_fetch(cx, "Authorize through Quick Connect", fut)
}
