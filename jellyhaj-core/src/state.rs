use std::{convert::Infallible, ops::ControlFlow, sync::Arc};

use color_eyre::Result;
use color_eyre::eyre::Report;
use config::keybind_defs::GlobalCommand;
use futures_util::future::BoxFuture;
use jellyfin::{
    JellyfinClient, NoAuth,
    items::{MediaItem, RefreshItemQuery},
    user_views::UserView,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use valuable::Valuable;

pub fn flatten_control_flow(
    v: Result<Option<ControlFlow<Navigation, Navigation>>>,
) -> Result<Option<Navigation>> {
    match v {
        Err(e) => Err(e),
        Ok(None) => Ok(None),
        Ok(Some(ControlFlow::Continue(v) | ControlFlow::Break(v))) => Ok(Some(v)),
    }
}

#[derive(Debug, Valuable, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct LoginState {
    #[serde(default)]
    pub server_url: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub passwort_cmd: Vec<String>,
}

pub type ClientOut = Arc<Mutex<Option<JellyfinClient>>>;

#[derive(Debug)]
pub enum LoadPlay {
    Movie(Box<MediaItem>),
    Audio(Box<MediaItem>),
    Series { id: String },
    Season { series_id: String, id: String },
    Episode { series_id: String, id: String },
    Playlist { id: String },
    Music { id: String, album_id: String },
    MusicAlbum { id: String },
}

#[derive(Debug)]
pub enum NextScreen {
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
        seen: Option<u32>,
    },
    FetchPlay(LoadPlay),
    Play {
        items: Vec<MediaItem>,
        index: usize,
    },
    Error(Report),
    ItemDetails(Box<MediaItem>),
    ItemListDetails(Box<MediaItem>, Vec<MediaItem>),
    FetchItemListDetails(Box<MediaItem>),
    FetchItemListDetailsRef(String),
    FetchItemDetails(String),
    RefreshItem(String),
    DoRefreshItem {
        id: String,
        query: RefreshItemQuery,
    },
    Stats,
    Logs,
    Inspect,
    QuickConnect,
    QuickConnectAuth(String),
    SelectServer {
        state: LoginState,
        out: ClientOut,
    },
    ConnectToServer {
        state: LoginState,
        out: ClientOut,
    },
    SelectAuthMethod {
        state: LoginState,
        out: ClientOut,
        client: JellyfinClient<NoAuth>,
        quick_connect_available: bool,
        server_id: String,
    },
    AuthPassword {
        state: LoginState,
        out: ClientOut,
        client: JellyfinClient<NoAuth>,

        server_id: String,
    },
    AuthPasswordFetch {
        state: LoginState,
        out: ClientOut,
        client: JellyfinClient<NoAuth>,
        server_id: String,
    },
    AuthQuickConnectFetch {
        state: LoginState,
        out: ClientOut,
        client: JellyfinClient<NoAuth>,
        server_id: String,
    },
    AuthQuickConnectWait {
        state: LoginState,
        out: ClientOut,
        client: JellyfinClient<NoAuth>,
        secret: String,
        code: String,
        server_id: String,
    },
    AuthFinished {
        state: LoginState,
        out: ClientOut,
        client: JellyfinClient,
        server_id: String,
    },
    InspectValue(serde_json::Value),
    HttpClient,
    HttpClientFetch {
        url: String,
    },
    Exit,
}

impl From<Result<Self>> for NextScreen {
    fn from(value: Result<Self>) -> Self {
        match value {
            Ok(v) => v,
            Err(e) => Self::Error(e),
        }
    }
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
        Self::Push(match value {
            GlobalCommand::ShowStats => NextScreen::Stats,
            GlobalCommand::ShowLogs => NextScreen::Logs,
            GlobalCommand::ShowInspect => NextScreen::Inspect,
            GlobalCommand::QuickConnect => NextScreen::QuickConnect,
            GlobalCommand::ShowHome => NextScreen::LoadHomeScreen,
            GlobalCommand::HttpClient => NextScreen::HttpClient,
        })
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
