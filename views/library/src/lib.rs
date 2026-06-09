use std::pin::pin;

use color_eyre::{Result, eyre::Context};
use futures_util::StreamExt;
use jellyfin::{
    JellyfinClient, JellyfinVec,
    items::{GetItemsQuery, MediaItem},
    user_views::UserView,
};
use jellyhaj_core::{
    context::TuiContext,
    render::{Erased, make_new_erased},
    state::NextScreen,
};
use jellyhaj_fetch_view::make_fetch;
use jellyhaj_library_widget::LibraryWidget;

async fn fetch_user_view(jellyfin: JellyfinClient, view: Box<UserView>) -> Result<NextScreen> {
    let (items, seen) = {
        let mut stream = pin!(JellyfinVec::stream(async |start| {
            jellyfin
                .get_items(&GetItemsQuery {
                    user_id: jellyfin.get_auth().user.id.as_str().into(),
                    start_index: start.into(),
                    limit: 100.into(),
                    recursive: None,
                    parent_id: view.id.as_str().into(),
                    exclude_item_types: None,
                    include_item_types: None,
                    enable_images: true.into(),
                    enable_image_types: "Thumb, Backdrop, Primary".into(),
                    image_type_limit: 1.into(),
                    enable_user_data: true.into(),
                    fields: None,
                    sort_by: "DateLastContentAdded".into(),
                    sort_order: "Descending".into(),
                })
                .await
                .context("requesting items")?
                .deserialize()
                .context("deserializing items")
        }));
        if let Some(v) = stream.next().await {
            let v = v?;
            (v.items, Some(stream.seen()))
        } else {
            (vec![], None)
        }
    };
    Ok(NextScreen::UserView { view, items, seen })
}

#[must_use]
pub fn render_fetch_user_view(cx: TuiContext, view: Box<UserView>) -> Erased {
    let title = format!("Loading user view {}", view.name);
    let inner = fetch_user_view(cx.jellyfin.clone(), view);
    make_fetch(cx, title, inner)
}

#[must_use]
pub fn render_user_view(
    cx: TuiContext,
    view: Box<UserView>,
    items: Vec<MediaItem>,
    seen: Option<u32>,
) -> Erased {
    let widget = LibraryWidget::new(view, items, &cx, seen);
    make_new_erased(cx, widget)
}
