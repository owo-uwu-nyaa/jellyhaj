use jellyfin::{JellyfinClient, NoAuth};
use jellyhaj_core::{
    state::{ClientOut, LoginState},
    widgets::shaded::widget::{Erased, make_new_erased},
};
use jellyhaj_form_widget::form::{FormCommandMapper, FormData};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_login_widget::select::{
    SelectData, SelectDataAction, SelectDataSelection, SelectWidget,
};

use crate::LoginContext;

pub fn render_select_auth_method(
    cx: LoginContext,
    state: LoginState,
    out: ClientOut,
    client: JellyfinClient<NoAuth>,
    quick_connect_available: bool,
    server_id: String,
) -> Erased {
    let widget =
        SelectData::new(quick_connect_available).make_with(SelectDataSelection::Passwort(()));
    let widget = KeybindWidget::new(
        widget,
        cx.config.keybinds.form.clone(),
        FormCommandMapper::<SelectDataAction>::default(),
    );
    let widget = SelectWidget::new(widget, out, state, client, server_id);
    make_new_erased(cx, widget)
}
