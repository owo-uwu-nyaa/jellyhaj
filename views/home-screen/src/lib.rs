mod fetch;

use jellyfin::{items::MediaItem, user_views::UserView};
use jellyhaj_core::{
    context::{DefaultTerminal, KeybindEvents, TuiContext},
    render::{NavigationResult, render_widget},
};
use jellyhaj_fetch_view::make_fetch;
use jellyhaj_home_screen_widget::HomeScreenState;

#[derive(Debug)]
pub enum Pass {
    Reload,
    Stats,
    Logs,
    Quit,
}

pub fn render_home_screen(
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    cx: TuiContext,
    cont: Vec<MediaItem>,
    next_up: Vec<MediaItem>,
    libraries: Vec<UserView>,
    library_latest: Vec<(String, Vec<MediaItem>)>,
) -> impl Future<Output = NavigationResult> {
    let state = HomeScreenState::new(&cx, cont, next_up, libraries, library_latest);
    render_widget(term, events, cx, state)
}

pub fn render_fetch_home_screen(
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    cx: TuiContext,
) -> impl Future<Output = NavigationResult> {
    let fut = fetch::fetch(cx.jellyfin.clone());
    make_fetch(term, events, cx, "Fetching Home Screen", fut)
}
