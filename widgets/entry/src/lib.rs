pub mod map;

use color_eyre::Result;
use jellyfin::{
    JellyfinClient,
    image::select_images,
    items::{ItemType, MediaItem, UserData},
    socket::ChangedUserData,
    user_views::UserView,
};
use jellyhaj_core::{
    context::{DB, JellyfinEventInterests, Spawner},
    keybinds::EntryCommand,
    state::Navigation,
};
use jellyhaj_image::{JellyfinImage, ParsedImage};
pub use jellyhaj_image::{Picker, SqliteConnection, Stats, cache::ImageProtocolCache};
use jellyhaj_widgets_core::{
    Config, ContextRef, FontSize, GetFromContext, ItemWidget, JellyhajWidget, JellyhajWidgetExt,
    WidgetContext, Wrapper,
};
use ratatui::{
    crossterm::event::{MouseButton, MouseEventKind},
    layout::{Rect, Size},
    style::Color,
    text::Span,
    widgets::{Block, BorderType, Paragraph, Widget},
};
use std::{borrow::Cow, fmt::Debug};
use tracing::instrument;
use valuable::Valuable;

#[derive(Debug, Clone, Valuable)]
#[allow(clippy::large_enum_variant)]
pub enum EntryData {
    Item(MediaItem),
    View(UserView),
}

impl From<MediaItem> for EntryData {
    fn from(value: MediaItem) -> Self {
        EntryData::Item(value)
    }
}
impl From<UserView> for EntryData {
    fn from(value: UserView) -> Self {
        EntryData::View(value)
    }
}

impl EntryData {
    pub fn item(&self) -> Option<&MediaItem> {
        if let EntryData::Item(i) = self {
            Some(i)
        } else {
            None
        }
    }
    pub fn item_mut(&mut self) -> Option<&mut MediaItem> {
        if let EntryData::Item(i) = self {
            Some(i)
        } else {
            None
        }
    }

    pub fn into_item(self) -> Option<MediaItem> {
        if let EntryData::Item(i) = self {
            Some(i)
        } else {
            None
        }
    }
}

fn calc_dimensions(config: &Config, font_size: FontSize) -> Size {
    let image_width = config.entry_image_width;
    let image_height = {
        let width = image_width * font_size.0;
        let width: f64 = width.into();
        let height = (width / 16.0) * 9.0;
        let height = height / f64::from(font_size.1);
        height.ceil() as u16
    };
    Size {
        width: image_width + 2,
        height: image_height + 2,
    }
}

fn updated_user_data(
    data: UserData,
    entry: &mut EntryData,
    watch_status: &mut Option<Cow<'static, str>>,
) -> Result<Option<Navigation>> {
    *watch_status = if let Some(num @ 1..) = data.unplayed_item_count {
        Some(format!("{num}").into())
    } else if data.played {
        Some("✓".into())
    } else {
        None
    };
    entry
        .item_mut()
        .expect("should only be requested for item inners")
        .user_data = Some(data);
    Ok(None)
}

#[derive(Valuable)]
pub struct Entry {
    #[valuable(skip)]
    image: Option<JellyfinImage>,
    title: String,
    subtitle: Option<String>,
    inner: EntryData,
    #[valuable(skip)]
    watch_status: Option<Cow<'static, str>>,
    #[valuable(skip)]
    size: Size,
    active: bool,
}

impl Entry {
    pub fn data(&self) -> &EntryData {
        &self.inner
    }
    pub fn new(
        data: impl Into<EntryData>,
        cx: &(impl ContextRef<ImageProtocolCache> + ContextRef<Config> + ContextRef<Picker>),
    ) -> Self {
        let size = calc_dimensions(cx.as_ref(), Picker::get_ref(cx).font_size());
        match data.into() {
            EntryData::Item(item) => from_media_item(item, cx, size),
            EntryData::View(user_view) => from_user_view(user_view, cx, size),
        }
    }
}

impl Debug for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Entry")
            .field("title", &self.title)
            .field("subtitle", &self.subtitle)
            .field("watch_status", &self.watch_status)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub enum EntryAction {
    Inner(ParsedImage),
    Command(EntryCommand),
    UpdatedUserData(UserData),
}

#[derive(Clone, Copy)]
struct EntryWrapper;

impl Wrapper<ParsedImage> for EntryWrapper {
    type F = EntryAction;

    fn wrap(&self, val: ParsedImage) -> Self::F {
        EntryAction::Inner(val)
    }
}

impl<
    R: ContextRef<Spawner>
        + ContextRef<Config>
        + ContextRef<Picker>
        + ContextRef<Stats>
        + ContextRef<JellyfinClient>
        + ContextRef<JellyfinEventInterests>
        + ContextRef<DB>
        + 'static,
> ItemWidget<R> for Entry
{
    const NAME: &str = "entry";
    type IAction = EntryAction;
    type IActionResult = Navigation;

    fn visit_children(&self, visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        if let Some(image) = self.image.as_ref() {
            visitor.visit::<R, JellyfinImage>(image);
        }
    }

    fn init(&mut self, cx: WidgetContext<'_, Self::IAction, impl Wrapper<Self::IAction>, R>) {
        if let EntryData::Item(item) = &self.inner {
            JellyfinEventInterests::get_ref(cx.refs).with(|interests| {
                interests.register_changed_userdata(
                    item.id.clone(),
                    cx.submitter.wrap_with(|changed: ChangedUserData| {
                        EntryAction::UpdatedUserData(changed.user_data)
                    }),
                )
            });
        }
        if let Some(image) = self.image.as_mut() {
            image.init(cx.wrap_with(EntryWrapper));
        }
    }

    fn dimensions_static(cx: &R) -> Size {
        calc_dimensions(cx.as_ref(), Picker::get_ref(cx).font_size())
    }
    fn dimensions(&self) -> Size {
        self.size
    }
    fn item_apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::IAction, impl Wrapper<Self::IAction>, R>,
        action: Self::IAction,
    ) -> Result<Option<Self::IActionResult>> {
        Ok(match action {
            EntryAction::Inner(action) => {
                if let Some(image) = self.image.as_mut() {
                    let None = image.apply_action(cx.wrap_with(EntryWrapper), action)?;
                }
                None
            }
            EntryAction::Command(entry_command) => self.inner.apply_command(entry_command, cx.refs),
            EntryAction::UpdatedUserData(user_data) => {
                return updated_user_data(user_data, &mut self.inner, &mut self.watch_status);
            }
        })
    }
    fn item_click(
        &mut self,
        cx: WidgetContext<'_, Self::IAction, impl Wrapper<Self::IAction>, R>,
        _: ratatui::prelude::Position,
        _: Size,
        kind: ratatui::crossterm::event::MouseEventKind,
        _: ratatui::crossterm::event::KeyModifiers,
    ) -> Result<Option<Self::IActionResult>> {
        if kind == MouseEventKind::Down(MouseButton::Left) {
            Ok(self.inner.apply_command(EntryCommand::Activate, cx.refs))
        } else {
            Ok(None)
        }
    }

    #[instrument(skip_all, name = "render_entry")]
    fn render_item_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        cx: WidgetContext<'_, Self::IAction, impl Wrapper<Self::IAction>, R>,
    ) -> Result<()> {
        let mut outer = Block::bordered()
            .border_type(if self.active {
                BorderType::Double
            } else {
                BorderType::Plain
            })
            .title_top(self.title.as_str());
        if let Some(subtitle) = &self.subtitle {
            outer = outer.title_bottom(subtitle.as_str());
        }
        let inner = outer.inner(area);
        if let Some(image) = &mut self.image {
            image.render_fallible(inner, buf, cx.wrap_with(EntryWrapper))?
        }
        outer.render(area, buf);
        if let Some(watch_status) = self.watch_status.as_ref() {
            Paragraph::new(Span::styled(watch_status.clone(), Color::LightBlue))
                .right_aligned()
                .render(
                    Rect {
                        x: area.x,
                        y: area.y,
                        width: area.width,
                        height: 1,
                    },
                    buf,
                );
        }
        Ok(())
    }
    fn set_active(&mut self, active: bool) {
        self.active = active
    }

    fn item_accepts_text_input(&self) -> bool {
        false
    }

    fn item_accept_char(&mut self, _: char) {
        unimplemented!()
    }

    fn item_accept_text(&mut self, _: String) {
        unimplemented!()
    }
}

fn from_media_item(item: MediaItem, cx: &impl ContextRef<ImageProtocolCache>, size: Size) -> Entry {
    let (title, subtitle) = match &item.item_type {
        ItemType::Movie | ItemType::Unknown { item_type: _ } | ItemType::CollectionFolder => {
            (item.name.clone(), None)
        }
        ItemType::Episode {
            season_id: _,
            season_name: _,
            series_id: _,
            series_name,
        } => (series_name.clone(), item.name.clone().into()),
        ItemType::Season {
            series_id: _,
            series_name,
        } => (series_name.clone(), item.name.clone().into()),
        ItemType::Series | ItemType::MusicAlbum => (item.name.clone(), None),
        ItemType::Playlist | ItemType::Folder => (item.name.clone(), None),
        ItemType::Music { album_id: _, album } => (album.clone(), item.name.clone().into()),
    };
    let image = select_images(&item)
        .map(|(image_type, tag)| -> _ {
            JellyfinImage::new(item.id.clone(), tag.to_string(), image_type, cx)
        })
        .next();
    let watch_status = if let Some(user_data) = item.user_data.as_ref() {
        if let Some(num @ 1..) = user_data.unplayed_item_count {
            Some(format!("{num}").into())
        } else if user_data.played {
            Some("✓".into())
        } else {
            None
        }
    } else {
        None
    };
    Entry {
        image,
        title,
        subtitle,
        inner: EntryData::Item(item),
        watch_status,
        active: false,
        size,
    }
}

fn from_user_view(item: UserView, cx: &impl ContextRef<ImageProtocolCache>, size: Size) -> Entry {
    let title = item.name.clone();
    let image = item
        .image_tags
        .iter()
        .flat_map(|map| map.iter())
        .next()
        .map(|(image_type, tag)| JellyfinImage::new(item.id.clone(), tag.clone(), *image_type, cx));
    Entry {
        image,
        title,
        subtitle: None,
        inner: EntryData::View(item),
        watch_status: None,
        size,
        active: false,
    }
}
