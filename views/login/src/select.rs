use jellyfin::{JellyfinClient, NoAuth};
use jellyhaj_core::{
    state::{ClientOut, LoginState},
    widgets::shaded::widget::{Erased, make_new_erased},
};
use jellyhaj_form_widget::form::{FormCommandMapper, FormDataExt};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_login_widget::select::{
    SelectActionMapper, SelectData, SelectDataAction, SelectWidget,
};
use jellyhaj_widgets_core::outer::UnwrapWidget;

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
        SelectData::new(out, state, client, quick_connect_available, server_id).make_with_default();
    let widget = UnwrapWidget::new(widget);
    let widget = KeybindWidget::new(
        widget,
        cx.config.keybinds.form.clone(),
        FormCommandMapper::<SelectDataAction>::default(),
    );
    let widget = UnwrapWidget::new(widget);
    let widget = SelectWidget::new(widget, SelectActionMapper);
    make_new_erased(cx, widget)
}
