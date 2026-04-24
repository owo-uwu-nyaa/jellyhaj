use std::{convert::Infallible, ops::ControlFlow};

use color_eyre::Result;
use color_eyre::eyre::Report;
use config::keybind_defs::GlobalCommand;
use futures_util::future::BoxFuture;
use jellyfin::{
    items::{MediaItem, PlaybackInfo},
    user_views::UserView,
};

pub fn flatten_control_flow(
    v: Result<Option<ControlFlow<Navigation, Navigation>>>,
) -> Result<Option<Navigation>> {
    match v {
        Err(e) => Err(e),
        Ok(None) => Ok(None),
        Ok(Some(ControlFlow::Continue(v))) => Ok(Some(v)),
        Ok(Some(ControlFlow::Break(v))) => Ok(Some(v)),
    }
}

#[derive(Debug)]
pub enum LoadPlay {
    Movie(Box<MediaItem>),
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
    LoadUserView(Box<UserView>),
    UserView {
        view: Box<UserView>,
        items: Vec<MediaItem>,
    },
    FetchPlay(LoadPlay),
    Play {
        items: Vec<(MediaItem, PlaybackInfo)>,
        index: usize,
    },
    Error(Report),
    ItemDetails(Box<MediaItem>),
    ItemListDetails(Box<MediaItem>, Vec<MediaItem>),
    FetchItemListDetails(Box<MediaItem>),
    FetchItemListDetailsRef(String),
    FetchItemDetails(String),
    RefreshItem(String),
    Stats,
    Logs,
    Inspect,
}

#[allow(clippy::large_enum_variant)]
pub enum Navigation {
    PopContext,
    Push(NextScreen),
    Replace(NextScreen),
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
            GlobalCommand::ShowStats => Navigation::Push(NextScreen::Stats),
            GlobalCommand::ShowLogs => Navigation::Push(NextScreen::Logs),
            GlobalCommand::ShowInspect => Navigation::Push(NextScreen::Inspect),
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
