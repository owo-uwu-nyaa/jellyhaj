use std::{convert::Infallible, pin::Pin};

use color_eyre::{Result, eyre::Context};
use jellyfin::{
    JellyfinClient,
    items::{RefreshItemQuery, RefreshMode},
};
use jellyhaj_core::{
    context::TuiContext,
    keybinds::RefreshItemCommand,
    state::{Navigation, NextScreen},
};
use jellyhaj_fetch_view::render_fetch_future;
use jellyhaj_form_widget::{
    FormAction, QuitForm, Selection,
    button::{ActionCreator, Button},
    form,
};
use jellyhaj_keybinds_widget::{CommandAction, KeybindWidget, MappedCommand};
use jellyhaj_render_widgets::TermExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Selection)]
enum Action {
    #[default]
    #[descr("Scan for new and updated files")]
    NewUpdated,
    #[descr("Search for missing metadata")]
    MissingMetadata,
    #[descr("Replace all metadata")]
    ReplaceMetadata,
}

pub enum FormResult {
    Quit,
    Submit,
}

impl From<QuitForm> for FormResult {
    fn from(_: QuitForm) -> Self {
        FormResult::Quit
    }
}

#[derive(Default)]
struct AC;
impl ActionCreator for AC {
    type T = FormResult;

    fn make_action(&self) -> Self::T {
        FormResult::Submit
    }
}

#[derive(Default)]
#[form("Refresh Metadata", FormResult)]
struct RefreshItem {
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

pub async fn render_refresh_item_form(
    cx: Pin<&mut TuiContext>,
    item: String,
) -> Result<Navigation> {
    let cx = cx.project();

    let mut form = RefreshItem::default();
    let mut widget = KeybindWidget::new(
        RefreshItemWidget::new(&mut form),
        &cx.config.help_prefixes,
        cx.config.keybinds.refresh_item.clone(),
        |command| {
            MappedCommand::<Infallible, _>::Down(match command {
                RefreshItemCommand::Quit => FormAction::Quit,
                RefreshItemCommand::Up => FormAction::Up,
                RefreshItemCommand::Down => FormAction::Down,
                RefreshItemCommand::Select => FormAction::Enter,
            })
        },
    );

    Ok(
        match cx
            .term
            .render(&mut widget, cx.events, cx.spawn.clone())
            .await?
        {
            CommandAction::Action(FormResult::Quit) => Navigation::PopContext,
            CommandAction::Action(FormResult::Submit) => {
                Navigation::Replace(NextScreen::SendRefreshItem(item, form.to_query()))
            }
            CommandAction::Exit => Navigation::Exit,
        },
    )
}

async fn refresh_item(
    jellyfin: &JellyfinClient,
    item_id: String,
    query: RefreshItemQuery,
) -> Result<Navigation> {
    jellyfin
        .refresh_item(&item_id, &query)
        .await
        .context("refreshing jellyfin item")?;

    Ok(Navigation::PopContext)
}

pub async fn render_send_refresh_item(
    cx: Pin<&mut TuiContext>,
    item_id: String,
    query: RefreshItemQuery,
) -> Result<Navigation> {
    let cx = cx.project();
    render_fetch_future(
        "refreshing Item",
        refresh_item(cx.jellyfin, item_id, query),
        cx.events,
        cx.config.keybinds.fetch.clone(),
        cx.term,
        &cx.config.help_prefixes,
        cx.spawn.clone(),
    )
    .await
}
