use jellyfin::{
    connect::JsonResponseHelper,
    request::{NoQuery, RequestBuilderExt},
};
use jellyhaj_core::{
    context::TuiContext,
    state::NextScreen,
    widgets::shaded::widget::{Erased, make_new_erased},
};
use jellyhaj_form_widget::form::{FormCommandMapper, FormDataDefaultExt};
use jellyhaj_http_client_widget::HttpClientData;
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::outer::{Named, OuterWidget, UnwrapWidget};
pub struct Name;
impl Named for Name {
    const NAME: &str = "http-client";
}

#[must_use]
pub fn render_http_client(cx: TuiContext) -> Erased {
    let widget = OuterWidget::<Name, _>::new(KeybindWidget::new(
        UnwrapWidget::new(HttpClientData::new().make_with_default()),
        cx.config.keybinds.form.clone(),
        FormCommandMapper::default(),
    ));
    make_new_erased(cx, widget)
}

#[must_use]
pub fn render_http_client_fetch(cx: TuiContext, url: String) -> Erased {
    let jellyfin = cx.jellyfin.clone();
    let fut = async move {
        let val = jellyfin
            .send_request_json(jellyfin.get(&*url, NoQuery)?.empty_body()?)
            .deserialize()
            .await?;
        Ok(NextScreen::InspectValue(val))
    };
    jellyhaj_fetch_view::make_fetch(cx, "Sending request to jellyfin server", fut)
}
