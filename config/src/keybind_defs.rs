use keybinds::{BindingMap, Command, keybind_config};
use valuable::Valuable;

#[derive(Debug)]
#[keybind_config]
pub struct Keybinds {
    pub logger: BindingMap<LoggerCommand>,
    pub stats: BindingMap<StatsCommand>,
    pub fetch: BindingMap<LoadingCommand>,
    pub play_mpv: BindingMap<MpvCommand>,
    pub form: BindingMap<FormCommand>,
    pub user_view: BindingMap<UserViewCommand>,
    pub home_screen: BindingMap<HomeScreenCommand>,
    pub error: BindingMap<ErrorCommand>,
    pub item_details: BindingMap<ItemDetailsCommand>,
    pub item_list_details: BindingMap<ItemListDetailsCommand>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Command, Valuable)]
pub enum GlobalCommand {
    ShowStats,
    ShowLogs,
}

#[derive(Debug, Clone, Copy, Command, Valuable)]
pub enum EntryCommand {
    Activate,
    Play,
    Open,
    OpenSeries,
    OpenSeason,
    OpenEpisode,
    RefreshItem,
    SetWatched,
    UnsetWatched,
}

#[derive(Debug, Clone, Copy, Command, Valuable)]
pub enum LoggerCommand {
    Space,
    TargetUp,
    TargetDown,
    Left,
    Right,
    Plus,
    Minus,
    Hide,
    Focus,
    MessagesUp,
    MessagesDown,
    Escape,
    Quit,
    #[command(flatten)]
    Global(GlobalCommand),
}

#[derive(Debug, Clone, Copy, Command, Valuable)]
pub enum StatsCommand {
    Quit,
    #[command(flatten)]
    Global(GlobalCommand),
}

#[derive(Debug, Clone, Copy, Command, Valuable)]
pub enum LoadingCommand {
    Quit,
    #[command(flatten)]
    Global(GlobalCommand),
}

#[derive(Debug, Clone, Copy, Command, Valuable)]
pub enum MpvCommand {
    Quit,
    Pause,
    #[command(flatten)]
    Global(GlobalCommand),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Command, Valuable)]
pub enum FormCommand {
    Quit,
    Up,
    Down,
    Left,
    Right,
    Delete,
    Enter,
    #[command(flatten)]
    Global(GlobalCommand),
}

#[derive(Debug, Clone, Copy, Command, Valuable)]
pub enum UserViewCommand {
    Quit,
    Reload,
    Prev,
    Next,
    Up,
    Down,
    #[command(flatten)]
    Entry(EntryCommand),
    #[command(flatten)]
    Global(GlobalCommand),
}

#[derive(Debug, Clone, Copy, Command, Valuable)]
pub enum HomeScreenCommand {
    Quit,
    Reload,
    Left,
    Right,
    Up,
    Down,
    #[command(flatten)]
    Entry(EntryCommand),
    #[command(flatten)]
    Global(GlobalCommand),
}

#[derive(Debug, Clone, Copy, Command, Valuable)]
pub enum ErrorCommand {
    Quit,
    Kill,
    Up,
    Down,
    Left,
    Right,
    #[command(flatten)]
    Global(GlobalCommand),
}

#[derive(Debug, Clone, Copy, Command, Valuable)]
pub enum ItemDetailsCommand {
    Quit,
    Up,
    Down,
    Reload,
    #[command(flatten)]
    Entry(EntryCommand),
    #[command(flatten)]
    Global(GlobalCommand),
}

#[derive(Debug, Clone, Copy, Command, Valuable)]
pub enum ItemListDetailsCommand {
    Quit,
    Reload,
    Up,
    Down,
    Left,
    Right,
    #[command(flatten)]
    Entry(EntryCommand),
    RefreshParentItem,
    #[command(flatten)]
    Global(GlobalCommand),
}
