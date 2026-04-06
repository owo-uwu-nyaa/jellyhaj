mod fetch;

use jellyfin::{items::MediaItem, user_views::UserView};
use jellyhaj_core::{
    context::TuiContext,
    render::{Erased, make_new_erased},
};
use jellyhaj_fetch_view::make_fetch;
use jellyhaj_home_screen_widget::HomeScreen;

#[derive(Debug)]
pub enum Pass {
    Reload,
    Stats,
    Logs,
    Quit,
}

pub fn render_home_screen(
    cx: TuiContext,
    cont: Vec<MediaItem>,
    next_up: Vec<MediaItem>,
    libraries: Vec<UserView>,
    library_latest: Vec<(String, Vec<MediaItem>)>,
) -> Erased {
    let widget = HomeScreen::new(&cx, cont, next_up, libraries, library_latest);
    make_new_erased(cx, widget)
}

pub fn make_fetch_home_screen(cx: TuiContext) -> Erased {
    let fut = fetch::fetch(cx.jellyfin.clone());
    make_fetch(cx, "Fetching Home Screen", fut)
}
