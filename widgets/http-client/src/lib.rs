use std::convert::Infallible;

use jellyhaj_core::state::{Navigation, NextScreen};
use jellyhaj_form_widget::{
    button::Button,
    form::{FormDataTypes, FormResultMapper},
    form_widget,
    label::DynamicLabel,
    text_field::TextField,
};
use jellyhaj_widgets_core::{Result, WidgetContext, Wrapper, valuable::Valuable};

#[derive(Debug, Clone, Copy)]
pub struct Get;
impl From<Infallible> for Get {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

pub struct RequestMapper;
impl FormResultMapper<HttpClientData> for RequestMapper {
    type Res = Navigation;

    fn map(
        state: &HttpClientData,
        _form_result: <HttpClientData as FormDataTypes>::AR,
        _cx: WidgetContext<
            '_,
            <HttpClientData as FormDataTypes>::Action,
            impl Wrapper<<HttpClientData as FormDataTypes>::Action>,
            (),
        >,
    ) -> Result<Option<Self::Res>> {
        Ok(Some(Navigation::Push(NextScreen::HttpClientFetch {
            url: state.url.text.clone(),
        })))
    }
}

#[derive(Valuable)]
#[form_widget("Http Client", Get, RequestMapper)]
pub struct HttpClientData {
    #[descr("Url")]
    url: TextField,
    #[descr("GET")]
    get: Button<Get>,
    #[descr("Device ID:")]
    device_id: DynamicLabel,
}

impl Default for HttpClientDataSelection {
    fn default() -> Self {
        Self::Url(())
    }
}

impl HttpClientData {
    #[must_use]
    pub fn new(device_id: String) -> Self {
        Self {
            url: TextField::new("/".to_string()),
            get: Button::new(Get),
            device_id: DynamicLabel { val: device_id },
        }
    }
}
