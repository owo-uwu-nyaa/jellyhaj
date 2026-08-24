use std::convert::Infallible;

use jellyhaj_core::state::{Navigation, NextScreen};
use jellyhaj_form_widget::{
    FormAction,
    button::Button,
    form::{Form, FormResultMapper, component::FormComponent},
    form_widget,
    label::DynamicLabel,
    text_field::TextField,
};
use jellyhaj_widgets_core::{RenderFlag, Result, WidgetContext, Wrapper, valuable::Valuable};

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
pub struct HttpClientData {
    #[form(descr = "Url")]
    url: TextField,
    #[form(descr = "GET")]
    get: Button<Get>,
    #[form(descr = "Device ID:")]
    device_id: DynamicLabel,
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
