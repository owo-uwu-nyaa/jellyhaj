use std::convert::Infallible;

use color_eyre::Result;
use color_eyre::eyre::Report;
use config::keybind_defs::GlobalCommand;
use futures_util::future::BoxFuture;
use jellyfin::{
    items::{MediaItem, PlaybackInfo},
    user_views::UserView,
};

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum LoadPlay {
    Movie(MediaItem),
    Series { id: String },
    Season { series_id: String, id: String },
    Episode { series_id: String, id: String },
    Playlist { id: String },
    Music { id: String, album_id: String },
    MusicAlbum { id: String },
}

#[derive(Debug, Default)]
pub enum NextScreen {
    #[default]
    LoadHomeScreen,
    HomeScreen {
        cont: Vec<MediaItem>,
        next_up: Vec<MediaItem>,
        libraries: Vec<UserView>,
        library_latest: Vec<(String, Vec<MediaItem>)>,
    },
    LoadUserView(UserView),
    UserView {
        view: UserView,
        items: Vec<MediaItem>,
    },
    FetchPlay(LoadPlay),
    Play {
        items: Vec<(MediaItem, PlaybackInfo)>,
        index: usize,
    },
    Error(Report),
    ItemDetails(MediaItem),
    ItemListDetails(MediaItem, Vec<MediaItem>),
    FetchItemListDetails(MediaItem),
    FetchItemListDetailsRef(String),
    FetchItemDetails(String),
    RefreshItem(String),
    Stats,
    Logs,
}

pub type Next = Box<NextScreen>;

#[allow(clippy::large_enum_variant)]
pub enum Navigation {
    PopContext,
    Push(Next),
    Replace(Next),
    Exit,
    PushWithoutTui(BoxFuture<'static, Result<()>>),
}

impl From<Infallible> for Navigation {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

impl From<GlobalCommand> for Navigation {
    fn from(value: GlobalCommand) -> Self {
        match value {
            GlobalCommand::ShowStats => Navigation::Push(Box::new(NextScreen::Stats)),
            GlobalCommand::ShowLogs => Navigation::Push(Box::new(NextScreen::Logs)),
        }
    }
}

impl std::fmt::Debug for Navigation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PopContext => write!(f, "PopContext"),
            Self::Push(arg0) => f.debug_tuple("Push").field(arg0).finish(),
            Self::Replace(arg0) => f.debug_tuple("Replace").field(arg0).finish(),
            Self::Exit => write!(f, "Exit"),
            Self::PushWithoutTui(_) => write!(f, "PushWithoutTui"),
        }
    }
}
