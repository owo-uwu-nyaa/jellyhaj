use std::convert::Infallible;

use color_eyre::eyre::Context;
use jellyhaj_core::{
    context::TuiContext,
    render::{Erased, make_new_erased},
    state::{Navigation, NextScreen::QuickConnectAuth},
};
use jellyhaj_fetch_view::make_nav_fetch;
use jellyhaj_form_widget::{
    button::Button,
    form::{FormCommandMapper, FormData, FormResultMapper},
    form_widget,
    text_field::TextField,
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::{
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

impl<R: 'static> FormResultMapper<R, QuickConnect> for Mapper {
    type Res = Navigation;

    fn map(
        state: &QuickConnect,
        form_result: <QuickConnect as jellyhaj_form_widget::form::FormDataTypes>::AR,
        _cx: jellyhaj_widgets_core::WidgetContext<
            '_,
            <QuickConnect as jellyhaj_form_widget::form::FormDataTypes>::Action,
            impl jellyhaj_widgets_core::Wrapper<
                <QuickConnect as jellyhaj_form_widget::form::FormDataTypes>::Action,
            >,
            R,
        >,
    ) -> jellyhaj_widgets_core::Result<Option<Self::Res>> {
        let Action::Login = form_result;
        Ok(Some(Navigation::Replace(QuickConnectAuth(
            state.code.text.clone(),
        ))))
    }
}

#[derive(Debug, Default, Valuable)]
#[form_widget("Authorize with Quick Connect", Action, Mapper)]
struct QuickConnect {
    #[descr("Code")]
    code: TextField,
    #[descr("Authenticate")]
    auth: Button<Action>,
}

struct Name;
impl Named for Name {
    const NAME: &str = "quick-connect";
}

#[must_use]
pub fn make_quick_connect(cx: TuiContext) -> Erased {
    let widget = OuterWidget::<Name, _>::new(KeybindWidget::new(
        UnwrapWidget::new(QuickConnect::default().make_with(QuickConnectSelection::Code(()))),
        cx.config.keybinds.form.clone(),
        FormCommandMapper::default(),
    ));
    make_new_erased(cx, widget)
}

#[must_use]
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
