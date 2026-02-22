use keybinds::{BindingMap, Command, keybind_config};

#[derive(Debug)]
#[keybind_config]
pub struct Keybinds {
    pub stats: BindingMap<StatsCommand>,
    pub logger: BindingMap<LoggerCommand>,
    pub fetch: BindingMap<LoadingCommand>,
    pub play_mpv: BindingMap<MpvCommand>,
    pub user_view: BindingMap<UserViewCommand>,
    pub home_screen: BindingMap<HomeScreenCommand>,
    pub login_info: BindingMap<LoginInfoCommand>,
    pub error: BindingMap<ErrorCommand>,
    pub item_details: BindingMap<ItemDetailsCommand>,
    pub item_list_details: BindingMap<ItemListDetailsCommand>,
    pub form: BindingMap<FormCommand>,
}

#[derive(Debug, Clone, Copy, Command)]
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
}

#[derive(Debug, Clone, Copy, Command)]
pub enum StatsCommand {
    Quit,
}

#[derive(Debug, Clone, Copy, Command)]
pub enum LoadingCommand {
    Quit,
}

#[derive(Debug, Clone, Copy, Command)]
pub enum MpvCommand {
    Quit,
    Pause,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Command)]
pub enum FormCommand {
    Quit,
    Up,
    Down,
    Left,
    Right,
    Delete,
    Enter,
}

#[derive(Debug, Clone, Copy, Command)]
pub enum EntryCommand {
    Activate,
    Play,
    Open,
    OpenSeries,
    OpenSeason,
    OpenEpisode,
    RefreshItem,
}

#[derive(Debug, Clone, Copy, Command)]
pub enum UserViewCommand {
    Quit,
    Reload,
    Prev,
    Next,
    Up,
    Down,
    #[command(flatten)]
    Entry(EntryCommand),
}

#[derive(Debug, Clone, Copy, Command)]
pub enum HomeScreenCommand {
    Quit,
    Reload,
    Left,
    Right,
    Up,
    Down,
    #[command(flatten)]
    Entry(EntryCommand),
    ShowStats,
    ShowLogs,
}

#[derive(Debug, Clone, Copy, Command)]
pub enum LoginInfoCommand {
    Delete,
    Submit,
    Next,
    Prev,
    Quit,
}

#[derive(Debug, Clone, Copy, Command)]
pub enum ErrorCommand {
    Quit,
    Kill,
    Up,
    Down,
    Left,
    Right,
    ShowLogs,
}

#[derive(Debug, Clone, Copy, Command)]
pub enum ItemDetailsCommand {
    Quit,
    Up,
    Down,
    Reload,
    #[command(flatten)]
    Entry(EntryCommand),
}

#[derive(Debug, Clone, Copy, Command)]
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
}
