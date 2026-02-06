mod fetch;

use color_eyre::Result;
use jellyhaj_core::{
    context::TuiContext,
    entries::EntryResultExt,
    keybinds::HomeScreenCommand,
    state::{Navigation, NextScreen},
};
use jellyhaj_entry_widget::{Entry, EntryAction, EntryData};
use jellyhaj_item_screen::{
    DimensionsParameter, ItemList, ItemScreen, ItemScreenAction, ItemScreenData,
};
use jellyhaj_keybinds_widget::{CommandAction, KeybindWidget, MappedCommand};
use jellyhaj_render_widgets::{JellyhajWidget, TermExt};
use std::pin::Pin;

pub enum Pass {
    Reload,
    Stats,
    Logs,
    Quit,
}

pub async fn render_home_screen(
    cx: Pin<&mut TuiContext>,
    screen: ItemScreenData<EntryData>,
) -> Result<Navigation> {
    let cx = cx.project();
    let mut widget = KeybindWidget::new(
        ItemScreen::new(
            screen.lists.into_iter().map(|l| -> ItemList<Entry> {
                ItemList::new(
                    l.items.into_iter().map(|i| {
                        Entry::new(
                            i,
                            cx.jellyfin,
                            cx.cache,
                            cx.image_cache,
                            cx.image_picker,
                            cx.stats,
                            cx.config,
                        )
                    }),
                    l.current,
                    l.title,
                    DimensionsParameter {
                        config: cx.config,
                        font_size: cx.image_picker.font_size(),
                    },
                )
            }),
            screen.current,
            screen.title,
            DimensionsParameter {
                config: cx.config,
                font_size: cx.image_picker.font_size(),
            },
        ),
        &cx.config.help_prefixes,
        cx.config.keybinds.home_screen.clone(),
        |command| match command {
            HomeScreenCommand::Quit => MappedCommand::Up(Pass::Quit),
            HomeScreenCommand::Reload => MappedCommand::Up(Pass::Reload),
            HomeScreenCommand::Left => MappedCommand::Down(ItemScreenAction::Left),
            HomeScreenCommand::Right => MappedCommand::Down(ItemScreenAction::Right),
            HomeScreenCommand::Up => MappedCommand::Down(ItemScreenAction::Up),
            HomeScreenCommand::Down => MappedCommand::Down(ItemScreenAction::Down),
            HomeScreenCommand::Open => {
                MappedCommand::Down(ItemScreenAction::CurrentInner(EntryAction::Open))
            }
            HomeScreenCommand::Play => {
                MappedCommand::Down(ItemScreenAction::CurrentInner(EntryAction::Play))
            }
            HomeScreenCommand::PlayOpen => {
                MappedCommand::Down(ItemScreenAction::CurrentInner(EntryAction::Activate))
            }
            HomeScreenCommand::OpenEpisode => {
                MappedCommand::Down(ItemScreenAction::CurrentInner(EntryAction::OpenEpisode))
            }
            HomeScreenCommand::OpenSeason => {
                MappedCommand::Down(ItemScreenAction::CurrentInner(EntryAction::OpenSeason))
            }
            HomeScreenCommand::OpenSeries => {
                MappedCommand::Down(ItemScreenAction::CurrentInner(EntryAction::OpenSeries))
            }
            HomeScreenCommand::RefreshItem => {
                MappedCommand::Down(ItemScreenAction::CurrentInner(EntryAction::Refresh))
            }
            HomeScreenCommand::ShowStats => MappedCommand::Up(Pass::Stats),
            HomeScreenCommand::ShowLogs => MappedCommand::Up(Pass::Logs),
        },
    );
    Ok(loop {
        match cx
            .term
            .render(&mut widget, cx.events, cx.spawn.clone())
            .await?
        {
            CommandAction::Up(Pass::Reload) => {
                break Navigation::Replace(NextScreen::LoadHomeScreen);
            }
            CommandAction::Up(Pass::Stats) => {
                break Navigation::Push {
                    current: NextScreen::HomeScreen(widget.into_state()),
                    next: NextScreen::Stats,
                };
            }
            CommandAction::Up(Pass::Logs) => {
                break Navigation::Push {
                    current: NextScreen::HomeScreen(widget.into_state()),
                    next: NextScreen::Logs,
                };
            }
            CommandAction::Up(Pass::Quit) => break Navigation::PopContext,
            CommandAction::Action(action) => {
                if let Some(next) = action.to_next_screen() {
                    break Navigation::Push {
                        current: NextScreen::HomeScreen(widget.into_state()),
                        next,
                    };
                }
            }
            CommandAction::Exit => break Navigation::Exit,
        }
    })
}

pub async fn render_fetch_home_screen(cx: Pin<&mut TuiContext>) -> Result<Navigation> {
    let cx = cx.project();
    jellyhaj_fetch_view::render_fetch_future(
        "Fetching Home Screen",
        fetch::fetch(cx.jellyfin),
        cx.events,
        cx.config.keybinds.fetch.clone(),
        cx.term,
        &cx.config.help_prefixes,
        cx.spawn.clone(),
    )
    .await
}
