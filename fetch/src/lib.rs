use color_eyre::{
    Result,
    eyre::{Context, OptionExt},
};
use jellyfin::{
    JellyfinClient, JellyfinVec,
    items::{GetItemsQuery, MediaItem},
};
use jellyhaj_core::{keybinds::LoadingCommand, state::Navigation};
use jellyhaj_keybinds_widget::{CommandAction, KeybindWidget, MappedCommand};
use jellyhaj_loading_widget::Loading;
use jellyhaj_render_widgets::TermExt;
use keybinds::{BindingMap, KeybindEvents};
use ratatui::DefaultTerminal;
use spawn::Spawner;
use tokio::select;
use tracing::instrument;

struct QuitAction;

pub async fn fetch_screen(
    title: &str,
    fetch: impl Future<Output = Result<Navigation>>,
    events: &mut KeybindEvents,
    keybinds: BindingMap<LoadingCommand>,
    term: &mut DefaultTerminal,
    help_prefixes: &[String],
    spawner: Spawner,
) -> Result<Navigation> {
    async fn render(
        title: &str,
        events: &mut KeybindEvents,
        keybinds: BindingMap<LoadingCommand>,
        term: &mut DefaultTerminal,
        help_prefixes: &[String],
        spawner: Spawner,
    ) -> Result<Navigation> {
        let mut widget = KeybindWidget::new(
            Loading::new(title),
            help_prefixes,
            keybinds,
            |LoadingCommand::Quit| MappedCommand::Up(QuitAction),
        );
        Ok(match term.render(&mut widget, events, spawner).await? {
            CommandAction::Up(QuitAction) => Navigation::PopContext,
            CommandAction::Exit => Navigation::Exit,
        })
    }
    select! {
        v = fetch => v,
        v = render(title, events, keybinds, term, help_prefixes, spawner) => v
    }
}

async fn single_item(jellyfin: &JellyfinClient, query: &GetItemsQuery<'_>) -> Result<MediaItem> {
    jellyfin
        .get_items(query)
        .await
        .context("fetching episode")?
        .deserialize()
        .await
        .context("deserializing episode")?
        .items
        .pop()
        .ok_or_eyre("No such item")
}

#[instrument(skip(jellyfin))]
pub async fn fetch_all_children(jellyfin: &JellyfinClient, id: &str) -> Result<Vec<MediaItem>> {
    let user_id = jellyfin.get_auth().user.id.as_str();
    let items = JellyfinVec::collect(async |start| {
        jellyfin
            .get_items(&GetItemsQuery {
                user_id: user_id.into(),
                start_index: start.into(),
                limit: 100.into(),
                parent_id: id.into(),
                enable_images: true.into(),
                enable_image_types: "Thumb, Backdrop, Primary".into(),
                image_type_limit: 1.into(),
                enable_user_data: true.into(),
                fields: "Overview".into(),
                ..Default::default()
            })
            .await
            .context("requesting items")?
            .deserialize()
            .await
            .context("deserializing items")
    })
    .await?;
    Ok(items)
}

#[instrument(skip(jellyfin))]
pub async fn fetch_item(jellyfin: &JellyfinClient, id: &str) -> Result<MediaItem> {
    let user_id = jellyfin.get_auth().user.id.as_str();
    single_item(
        jellyfin,
        &GetItemsQuery {
            user_id: user_id.into(),
            start_index: 0.into(),
            limit: 1.into(),
            parent_id: id.into(),
            enable_images: true.into(),
            enable_image_types: "Thumb, Backdrop, Primary".into(),
            image_type_limit: 1.into(),
            enable_user_data: true.into(),
            fields: "Overview".into(),
            ..Default::default()
        },
    )
    .await
}

pub async fn fetch_child_of_type(
    jellyfin: &JellyfinClient,
    t: &str,
    id: &str,
) -> Result<MediaItem> {
    let user_id = jellyfin.get_auth().user.id.as_str();
    single_item(
        jellyfin,
        &GetItemsQuery {
            user_id: user_id.into(),
            start_index: Some(0),
            limit: Some(1),
            parent_id: Some(id),
            include_item_types: Some(t),
            enable_images: true.into(),
            enable_image_types: "Primary, Backdrop, Thumb".into(),
            image_type_limit: 1.into(),
            enable_user_data: true.into(),
            recursive: true.into(),
            fields: "Overview".into(),
            ..Default::default()
        },
    )
    .await
}
