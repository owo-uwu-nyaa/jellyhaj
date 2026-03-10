mod fetch;

use jellyfin::{items::MediaItem, user_views::UserView};
use jellyhaj_core::{
    context::TuiContext,
    render::{NavigationResult, render_widget},
};
use jellyhaj_fetch_view::make_fetch;
use jellyhaj_home_screen_widget::HomeScreenState;
use std::pin::Pin;

#[derive(Debug)]
pub enum Pass {
    Reload,
    Stats,
    Logs,
    Quit,
}

pub fn render_home_screen(
    mut cx: Pin<&mut TuiContext>,
    cont: Vec<MediaItem>,
    next_up: Vec<MediaItem>,
    libraries: Vec<UserView>,
    library_latest: Vec<(String, Vec<MediaItem>)>,
) -> impl Future<Output = NavigationResult> {
    let state = HomeScreenState::new(cx.as_mut(), cont, next_up, libraries, library_latest);
    render_widget(cx, state)
}

pub fn render_fetch_home_screen(
    cx: Pin<&mut TuiContext>,
) -> impl Future<Output = NavigationResult> {
    let fut = fetch::fetch(cx.jellyfin.clone());
    make_fetch(cx, "Fetching Home Screen", fut)
}
