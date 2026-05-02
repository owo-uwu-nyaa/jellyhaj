use color_eyre::{Result, eyre::Context};
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
    let user_id = jellyfin.get_auth().user.id.as_str();
    let items = JellyfinVec::collect(async |start| {
        jellyfin
            .get_items(&GetItemsQuery {
                user_id: user_id.into(),
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
    })
    .await?;
    Ok(NextScreen::UserView { view, items })
}

#[must_use]
pub fn render_fetch_user_view(cx: TuiContext, view: Box<UserView>) -> Erased {
    let title = format!("Loading user view {}", view.name);
    let inner = fetch_user_view(cx.jellyfin.clone(), view);
    make_fetch(cx, title, inner)
}

#[must_use]
pub fn render_user_view(cx: TuiContext, view: Box<UserView>, items: Vec<MediaItem>) -> Erased {
    let widget = LibraryWidget::new(view, items, &cx);
    make_new_erased(cx, widget)
}
