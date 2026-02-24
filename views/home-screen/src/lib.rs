mod fetch;

use jellyfin::{items::MediaItem, user_views::UserView};
use jellyhaj_core::{
    CommandMapper,
    context::TuiContext,
    keybinds::HomeScreenCommand,
    render::{NavigationResult, render_widget},
    state::{Navigation, NextScreen},
};
use jellyhaj_entry_widget::{Entry, EntryAction, EntryState};
use jellyhaj_fetch_view::make_fetch;
use jellyhaj_item_screen::{ItemListState, ItemScreenAction, ItemScreenState};
use jellyhaj_keybinds_widget::KeybindState;
use jellyhaj_widgets_core::outer::{Named, OuterState};
use std::{ops::ControlFlow, pin::Pin};

#[derive(Debug)]
pub enum Pass {
    Reload,
    Stats,
    Logs,
    Quit,
}

struct Mapper;
impl CommandMapper<HomeScreenCommand> for Mapper {
    type A = ItemScreenAction<EntryAction>;

    fn map(&self, command: HomeScreenCommand) -> ControlFlow<Navigation, Self::A> {
        match command {
            HomeScreenCommand::Quit => ControlFlow::Break(Navigation::Exit),
            HomeScreenCommand::Reload => {
                ControlFlow::Break(Navigation::Replace(Box::new(NextScreen::LoadHomeScreen)))
            }
            HomeScreenCommand::Left => ControlFlow::Continue(ItemScreenAction::Left),
            HomeScreenCommand::Right => ControlFlow::Continue(ItemScreenAction::Right),
            HomeScreenCommand::Up => ControlFlow::Continue(ItemScreenAction::Up),
            HomeScreenCommand::Down => ControlFlow::Continue(ItemScreenAction::Down),
            HomeScreenCommand::Entry(entry_command) => ControlFlow::Continue(
                ItemScreenAction::CurrentInner(EntryAction::Command(entry_command)),
            ),
            HomeScreenCommand::Global(global_show) => ControlFlow::Break(global_show.into()),
        }
    }
}

struct Name;
impl Named for Name {
    const NAME: &str = "home-screen";
}

pub fn render_home_screen(
    mut cx: Pin<&mut TuiContext>,
    cont: Vec<MediaItem>,
    next_up: Vec<MediaItem>,
    libraries: Vec<UserView>,
    library_latest: Vec<(String, Vec<MediaItem>)>,
) -> impl Future<Output = NavigationResult> {
    let screen = ItemScreenState::new(
        [
            ItemListState::<Entry>::new(
                cont.into_iter().map(|i| EntryState::new(i, cx.as_mut())),
                "Continue Watching".to_string(),
            ),
            ItemListState::new(
                next_up.into_iter().map(|i| EntryState::new(i, cx.as_mut())),
                "Next Up".to_string(),
            ),
            ItemListState::new(
                libraries
                    .into_iter()
                    .map(|i| EntryState::new(i, cx.as_mut())),
                "Continue Watching".to_string(),
            ),
        ]
        .into_iter()
        .chain(library_latest.into_iter().map(|(title, list)| {
            ItemListState::new(
                list.into_iter().map(|i| EntryState::new(i, cx.as_mut())),
                title,
            )
        }))
        .filter(|l| !l.items.is_empty())
        .collect(),
        "Home".to_string(),
    );
    let state = OuterState::<Name, _, _, _>::new(KeybindState::new(
        screen,
        cx.config.help_prefixes.clone(),
        cx.config.keybinds.home_screen.clone(),
        Mapper,
    ));
    render_widget(cx, state)
}

pub fn render_fetch_home_screen(
    cx: Pin<&mut TuiContext>,
) -> impl Future<Output = NavigationResult> {
    let fut = fetch::fetch(cx.jellyfin.clone());
    make_fetch(cx, "Fetching Home Screen", fut)
}
