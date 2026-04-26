use std::{borrow::Cow, ops::ControlFlow};

use color_eyre::{
    Result,
    eyre::{Context, OptionExt},
};
use futures_util::FutureExt;
use jellyfin::{
    JellyfinClient, JellyfinVec,
    items::{GetItemsQuery, MediaItem},
};
use jellyhaj_core::{
    CommandMapper,
    context::TuiContext,
    keybinds::LoadingCommand,
    render::{Erased, make_new_erased},
    state::{Navigation, NextScreen},
};
use jellyhaj_fetch_widget::{FetchAction, FetchWidget};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::outer::{Named, OuterWidget};
use tracing::instrument;

struct FetchMapper;

impl CommandMapper<LoadingCommand> for FetchMapper {
    type A = FetchAction;

    fn map(&self, command: LoadingCommand) -> std::ops::ControlFlow<Navigation, Self::A> {
        match command {
            LoadingCommand::Quit => ControlFlow::Break(Navigation::PopContext),
            LoadingCommand::Global(g) => ControlFlow::Break(g.into()),
        }
    }
}

struct Name;
impl Named for Name {
    const NAME: &str = "fetch";
}

pub fn make_fetch(
    cx: TuiContext,
    title: impl Into<Cow<'static, str>>,
    fut: impl Future<Output = Result<NextScreen>> + Send + 'static,
) -> Erased {
    make_nav_fetch(cx, title, fut.map(|r| r.map(Navigation::Replace)))
}

pub fn make_nav_fetch(
    cx: TuiContext,
    title: impl Into<Cow<'static, str>>,
    fut: impl Future<Output = Result<Navigation>> + Send + 'static,
) -> Erased {
    let widget = OuterWidget::<Name, _>::new(KeybindWidget::new(
        FetchWidget::new(fut, title),
        cx.config.keybinds.fetch.clone(),
        FetchMapper,
    ));
    make_new_erased(cx, widget)
}

#[instrument(skip(jellyfin))]
pub async fn single_item(
    jellyfin: &JellyfinClient,
    query: &GetItemsQuery<'_>,
) -> Result<MediaItem> {
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
