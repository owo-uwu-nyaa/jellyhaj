use jellyfin::{JellyfinClient, items::MediaItem};
use jellyhaj_entry_widget::ImageCache;
use jellyhaj_event_listener::JellyfinEventInterests;
use jellyhaj_image::{Picker, Stats};
use jellyhaj_tabs_widget::TabContainer;
use spawn::Spawner;

use crate::{
    DB,
    children::{ChildAction, ItemChilds},
    overview::{Overview, OverviewAction},
};
use jellyhaj_core::{Config, keybinds::EntryCommand, state::Navigation};
use jellyhaj_widgets_core::ContextRef;

#[derive(Debug)]
pub enum ItemListDetailsCommom {
    Up,
    Down,
    ScrollUp,
    ScrollDown,
    Entry(EntryCommand),
}

impl From<ItemListDetailsCommom> for Option<ChildAction> {
    fn from(value: ItemListDetailsCommom) -> Self {
        Some(match value {
            ItemListDetailsCommom::Up => ChildAction::Up,
            ItemListDetailsCommom::Down => ChildAction::Down,
            ItemListDetailsCommom::ScrollUp => ChildAction::ScrollUp,
            ItemListDetailsCommom::ScrollDown => ChildAction::ScrollDown,
            ItemListDetailsCommom::Entry(entry_action) => ChildAction::CurrentEntry(entry_action),
        })
    }
}

impl From<ItemListDetailsCommom> for Option<OverviewAction> {
    fn from(value: ItemListDetailsCommom) -> Self {
        match value {
            ItemListDetailsCommom::Up | ItemListDetailsCommom::ScrollUp => Some(OverviewAction::Up),
            ItemListDetailsCommom::Down | ItemListDetailsCommom::ScrollDown => {
                Some(OverviewAction::Down)
            }
            ItemListDetailsCommom::Entry(_) => None,
        }
    }
}

#[derive(TabContainer)]
#[tab(
    action_result = Navigation,
    common_action = ItemListDetailsCommom,
    cx_constr = ContextRef<Spawner> +
        ContextRef<Config> +
        ContextRef<Picker> +
        ContextRef<DB> +
        ContextRef<ImageCache> +
        ContextRef<JellyfinClient> +
        ContextRef<JellyfinEventInterests> +
        ContextRef<Stats>)
]
pub struct ItemListDetails {
    #[tab = "Children"]
    pub children: ItemChilds,
    #[tab = "Overview"]
    pub overview: Overview<String>,
}

impl ItemListDetails {
    pub fn new(
        parent: Box<MediaItem>,
        children: impl IntoIterator<Item = MediaItem>,
        cx: &(impl ContextRef<Config> + ContextRef<Picker>),
    ) -> Self {
        let children = ItemChilds::new(parent.id, children, cx);
        let overview = Overview::new(parent.overview.unwrap_or_default(), String::new());
        Self { children, overview }
    }
}
