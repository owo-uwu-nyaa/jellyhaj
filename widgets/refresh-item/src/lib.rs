use std::{convert::Infallible, fmt::Debug};

use jellyfin::{
    items::{RefreshItemQuery, RefreshMode},
};
use jellyhaj_core::state::{Navigation, NextScreen};
use jellyhaj_form_widget::{
    Selection,
    button::{ActionCreator, Button},
    form::{FormDataTypes, FormResultMapper},
    form_widget,
};
use jellyhaj_widgets_core::{
    Result, WidgetContext, Wrapper,
};
use valuable::Valuable;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Selection, Valuable)]
enum Action {
    #[default]
    #[descr("Scan for new and updated files")]
    NewUpdated,
    #[descr("Search for missing metadata")]
    MissingMetadata,
    #[descr("Replace all metadata")]
    ReplaceMetadata,
}

#[derive(Debug)]
pub enum FormResult {
    Submit,
}

impl From<Infallible> for FormResult {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

#[derive(Default, Debug)]
struct AC;
impl ActionCreator for AC {
    type T = FormResult;

    fn make_action(&self) -> Self::T {
        FormResult::Submit
    }
}

pub struct RefreshItemResultMapper;

impl<R: 'static> FormResultMapper<R, RefreshItem>
    for RefreshItemResultMapper
{
    type Res = Navigation;

    fn map(
        state: &RefreshItem,
        form_result: <RefreshItem as FormDataTypes>::AR,
        _cx: WidgetContext<
            '_,
            <RefreshItem as FormDataTypes>::Action,
            impl Wrapper<<RefreshItem as FormDataTypes>::Action>,
            R,
        >,
    ) -> Result<Option<Self::Res>> {
        let FormResult::Submit = form_result;
        Ok(Some(Navigation::Replace(NextScreen::DoRefreshItem {
            id: state.id.clone(),
            query: state.to_query(),
        })))
    }
}

#[derive(Default, Debug, Valuable)]
#[form_widget("Refresh Metadata", FormResult, RefreshItemResultMapper)]
pub struct RefreshItem {
    #[skip]
    id: String,
    #[descr("Refresh mode")]
    action: Action,
    #[descr("Replace existing images")]
    #[show_if(self.action != Action::NewUpdated)]
    replace_images: bool,
    #[descr("Replace existing trickplay images")]
    #[show_if(self.action != Action::NewUpdated)]
    replace_trickplay: bool,
    #[descr("Refresh Now!")]
    refresh: Button<AC>,
}

impl Default for RefreshItemSelection {
    fn default() -> Self {
        Self::Action(None)
    }
}

impl RefreshItem {
    pub fn new(id: String) -> Self {
        Self {
            id,
            action: Action::NewUpdated,
            replace_images: false,
            replace_trickplay: false,
            refresh: Button::new(AC),
        }
    }
}

impl RefreshItem {
    fn to_query(&self) -> RefreshItemQuery {
        match self.action {
            Action::NewUpdated => RefreshItemQuery {
                recursive: true,
                metadata_refresh_mode: RefreshMode::Default,
                image_refresh_mode: RefreshMode::Default,
                replace_all_metadata: false,
                replace_all_images: false,
                regenerate_trickplay: false,
            },
            Action::MissingMetadata => RefreshItemQuery {
                recursive: true,
                metadata_refresh_mode: RefreshMode::FullRefresh,
                image_refresh_mode: RefreshMode::FullRefresh,
                replace_all_metadata: false,
                replace_all_images: self.replace_images,
                regenerate_trickplay: self.replace_trickplay,
            },
            Action::ReplaceMetadata => RefreshItemQuery {
                recursive: true,
                metadata_refresh_mode: RefreshMode::FullRefresh,
                image_refresh_mode: RefreshMode::FullRefresh,
                replace_all_metadata: true,
                replace_all_images: self.replace_images,
                regenerate_trickplay: self.replace_trickplay,
            },
        }
    }
}
