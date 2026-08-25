use std::convert::Infallible;

use jellyfin::{
    Authed,
    connect::JsonResponseHelper,
    request::{NoQuery, RequestBuilderExt},
};
use jellyhaj_context::TuiContext;
use jellyhaj_core::{
    state::{Navigation, NextScreen},
    widgets::shaded::widget::{Erased, make_new_erased},
};
use jellyhaj_form_widget::{
    FormAction,
    button::Button,
    form::{Form, FormCommandMapper, FormDataExt, FormResultMapper, component::FormComponent},
    form_widget,
    label::DynamicLabel,
    text_field::TextField,
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::{
    RenderFlag, Result, WidgetContext, Wrapper,
    outer::{Named, OuterWidget, UnwrapWidget},
};
use valuable::Valuable;

#[derive(Debug, Clone, Copy)]
struct Get;
impl From<Infallible> for Get {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}
struct RequestMapper;
impl FormResultMapper<HttpClientData> for RequestMapper {
    type Res = Navigation;

    fn map(
        state: &mut Form<HttpClientData>,
        _form_result: <HttpClientData as FormComponent>::AR,
        _cx: WidgetContext<
            '_,
            FormAction<<HttpClientData as FormComponent>::Action>,
            impl Wrapper<FormAction<<HttpClientData as FormComponent>::Action>>,
            (),
        >,
        _render_flag: &mut RenderFlag,
    ) -> Result<Option<Self::Res>> {
        Ok(Some(Navigation::Push(NextScreen::HttpClientFetch {
            url: state.data.url.text.clone(),
        })))
    }
}

#[derive(Valuable)]
#[form_widget("http client", Get, RequestMapper)]
struct HttpClientData {
    #[form(descr = "Url")]
    url: TextField,
    #[form(descr = "GET")]
    get: Button<Get>,
    #[form(descr = "Device ID:")]
    device_id: DynamicLabel,
}

impl HttpClientData {
    #[must_use]
    fn new(device_id: String) -> Self {
        Self {
            url: TextField::new("/".to_string()),
            get: Button::new(Get),
            device_id: DynamicLabel { val: device_id },
        }
    }
}

struct Name;
impl Named for Name {
    const NAME: &str = "http-client";
}

pub fn render_http_client(cx: TuiContext) -> Erased {
    let widget = OuterWidget::<Name, _>::new(KeybindWidget::new(
        UnwrapWidget::new(
            HttpClientData::new(cx.jellyfin.get_auth().device_id().to_owned()).make_with_default(),
        ),
        cx.config.keybinds.form.clone(),
        FormCommandMapper::default(),
    ));
    make_new_erased(cx, widget)
}

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
