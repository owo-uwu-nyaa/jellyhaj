use color_eyre::{Result, eyre::Context};
use jellyfin::{
    JellyfinClient, JellyfinVec, connect::JsonResponseHelper, items::MediaItem,
    user_views::UserView,
};
use jellyhaj_context::TuiContext;
use jellyhaj_core::{
    state::NextScreen,
    widgets::shaded::widget::{Erased, make_new_erased},
};
use jellyhaj_fetch_view::make_fetch;
use jellyhaj_library_widget::{LibraryWidget, make_item_query};

async fn fetch_user_view(jellyfin: JellyfinClient, view: Box<UserView>) -> Result<NextScreen> {
    let res: JellyfinVec<_> = jellyfin
        .get_items(&make_item_query(0, &view.id))
        .deserialize()
        .await?;
    let seen: u32 =
        res.items.len().try_into().context(
            "Jellyfin returned a ginourmas array as result. Something is extremely broken.",
        )?;
    let seen = if let Some(total) = res.total_record_count
        && total <= seen
    {
        None
    } else if seen == 0 {
        None
    } else {
        Some(seen)
    };
    Ok(NextScreen::UserView {
        view,
        items: res.items,
        seen,
    })
}

pub fn render_fetch_user_view(cx: TuiContext, view: Box<UserView>) -> Erased {
    let title = format!("Loading user view {}", view.name);
    let inner = fetch_user_view(cx.jellyfin.clone(), view);
    make_fetch(cx, title, inner)
}

pub fn render_user_view(
    cx: TuiContext,
    view: Box<UserView>,
    items: Vec<MediaItem>,
    seen: Option<u32>,
) -> Erased {
    let widget = LibraryWidget::new(view, items, &cx, seen);
    make_new_erased(cx, widget)
}
