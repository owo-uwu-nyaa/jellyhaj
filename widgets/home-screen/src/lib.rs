use std::{fmt::Debug, ops::ControlFlow, sync::Arc};

use jellyfin::{JellyfinClient, items::MediaItem, socket::ChangedUserData, user_views::UserView};
use jellyhaj_core::{
    CommandMapper, Config,
    keybinds::HomeScreenCommand,
    state::{Navigation, NextScreen},
};
use jellyhaj_entry_widget::{Entry, EntryAction, EntryData, ImageCache, Picker, Stats};
use jellyhaj_event_listener::JellyfinEventInterests;
use jellyhaj_item_screen::{ItemScreen, ItemScreenAction, new_item_list, new_item_screen};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::{
    ContextRef, GetFromContext, JellyhajWidgetBase, Result, WidgetContext, Wrapper,
    mapper::{ActionMapper, ActionMapperBase, ActionMapperWidget},
    outer::{Named, UnwrapWidget},
};
use spawn::Spawner;
use sqlx::SqliteConnection;
use valuable::Valuable;

type DB = Arc<tokio::sync::Mutex<SqliteConnection>>;

#[derive(Debug)]
pub enum HomeScreenAction {
    Reload,
    PotentialReload(bool),
}

#[derive(Clone, Copy, Valuable)]
pub struct Mapper;
impl CommandMapper<HomeScreenCommand> for Mapper {
    type A = ItemScreenAction<EntryAction>;

    fn map(&self, command: HomeScreenCommand) -> ControlFlow<Navigation, Self::A> {
        match command {
            HomeScreenCommand::Quit => ControlFlow::Break(Navigation::PopContext),
            HomeScreenCommand::Reload => {
                ControlFlow::Break(Navigation::Replace(NextScreen::LoadHomeScreen))
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
impl Wrapper<String> for Mapper {
    type F = HomeScreenAction;

    fn wrap(&self, _: String) -> Self::F {
        HomeScreenAction::Reload
    }
}
impl Wrapper<ChangedUserData> for Mapper {
    type F = HomeScreenAction;

    fn wrap(&self, val: ChangedUserData) -> Self::F {
        HomeScreenAction::PotentialReload(val.user_data.playback_position_ticks == 0)
    }
}

pub struct Name;
impl Named for Name {
    const NAME: &str = "home-screen";
}

type InnerWidget = UnwrapWidget<KeybindWidget<HomeScreenCommand, ItemScreen<Entry>, Mapper>>;

impl ActionMapperBase<InnerWidget> for Mapper {
    type Action = HomeScreenAction;
}

impl<
    R: ContextRef<Spawner>
        + ContextRef<Config>
        + ContextRef<Picker>
        + ContextRef<Stats>
        + ContextRef<JellyfinClient>
        + ContextRef<JellyfinEventInterests>
        + ContextRef<DB>
        + ContextRef<ImageCache>
        + 'static,
> ActionMapper<R, InnerWidget> for Mapper
{
    fn init(
        &mut self,
        this: &mut InnerWidget,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        _this_cx: WidgetContext<
            '_,
            <InnerWidget as JellyhajWidgetBase>::Action,
            impl Wrapper<<InnerWidget as JellyhajWidgetBase>::Action>,
            R,
        >,
    ) {
        JellyfinEventInterests::get_ref(cx.refs).with(|interests| {
            for entry in this.inner.inner.iter().flat_map(|i| i.iter()) {
                match entry.data() {
                    EntryData::Item(item) => {
                        let submitter = cx.submitter.wrap_with(Self);
                        interests.register_changed_userdata(item.id.clone(), submitter);
                    }
                    EntryData::View(library) => {
                        let submitter = cx.submitter.wrap_with(Self);
                        interests.register_folder_modified(library.id.clone(), submitter);
                    }
                }
            }
        });
    }

    fn map_action(
        &mut self,
        _this: &mut InnerWidget,
        _cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        _this_cx: WidgetContext<
            '_,
            <InnerWidget as JellyhajWidgetBase>::Action,
            impl Wrapper<<InnerWidget as JellyhajWidgetBase>::Action>,
            R,
        >,
        action: Self::Action,
        _render_flag: &mut jellyhaj_widgets_core::RenderFlag,
    ) -> Result<Option<<InnerWidget as JellyhajWidgetBase>::ActionResult>> {
        match action {
            HomeScreenAction::Reload | HomeScreenAction::PotentialReload(true) => {
                Ok(Some(Navigation::Replace(NextScreen::LoadHomeScreen)))
            }
            HomeScreenAction::PotentialReload(false) => Ok(None),
        }
    }
}

pub type HomeScreenWidget = ActionMapperWidget<Name, InnerWidget, Mapper>;

pub fn new_home_screen(
    cx: &(
         impl ContextRef<Spawner>
         + ContextRef<Config>
         + ContextRef<Picker>
         + ContextRef<Stats>
         + ContextRef<JellyfinClient>
         + ContextRef<JellyfinEventInterests>
         + ContextRef<DB>
         + ContextRef<ImageCache>
         + 'static
     ),
    cont: Vec<MediaItem>,
    next_up: Vec<MediaItem>,
    libraries: Vec<UserView>,
    library_latest: Vec<(String, Vec<MediaItem>)>,
) -> HomeScreenWidget {
    let screen = new_item_screen(
        [
            new_item_list(
                cont.into_iter().map(|i| Entry::new(i, cx)),
                "Continue Watching".to_string(),
                cx,
            ),
            new_item_list(
                next_up.into_iter().map(|i| Entry::new(i, cx)),
                "Next Up".to_string(),
                cx,
            ),
            new_item_list(
                libraries.into_iter().map(|i| Entry::new(i, cx)),
                "Continue Watching".to_string(),
                cx,
            ),
        ]
        .into_iter()
        .chain(library_latest.into_iter().map(|(title, list)| {
            new_item_list(list.into_iter().map(|i| Entry::new(i, cx)), title, cx)
        }))
        .filter(|l| !l.is_empty()),
        "Home",
        cx,
    );
    let inner = KeybindWidget::new(
        screen,
        Config::get_ref(cx).keybinds.home_screen.clone(),
        Mapper,
    );
    let inner = UnwrapWidget::new(inner);
    HomeScreenWidget::new(inner, Mapper)
}
