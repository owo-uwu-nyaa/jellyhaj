pub mod map;

use color_eyre::Result;
use jellyfin::{
    image::select_images,
    items::{ItemType, MediaItem},
    user_views::UserView,
};
use jellyhaj_core::{keybinds::EntryCommand, state::Navigation};
use jellyhaj_image::{JellyfinImage, JellyfinImageState};
pub use jellyhaj_image::{Picker, SqliteConnection, Stats, cache::ImageProtocolCache};
use jellyhaj_widgets_core::{
    Config, FontSize, ItemWidget, JellyhajWidget, JellyhajWidgetExt, JellyhajWidgetState,
    TuiContext, Wrapper, async_task::TaskSubmitter,
};
use ratatui::{
    crossterm::event::{MouseButton, MouseEventKind},
    layout::{Rect, Size},
    style::Color,
    text::Span,
    widgets::{Block, BorderType, Paragraph, Widget},
};
use std::{borrow::Cow, fmt::Debug, pin::Pin};
use tracing::instrument;

#[derive(Debug, Clone)]
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

#[derive(Debug)]
pub struct EntryState {
    image: Option<JellyfinImageState>,
    title: String,
    subtitle: Option<String>,
    inner: EntryData,
    watch_status: Option<Cow<'static, str>>,
}

impl JellyhajWidgetState for EntryState {
    type Action = EntryAction;

    type ActionResult = Navigation;

    type Widget = Entry;

    const NAME: &str = "entry";

    fn visit_children(visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        visitor.visit::<JellyfinImageState>();
    }

    fn into_widget(
        self,
        cx: std::pin::Pin<&mut jellyhaj_core::context::TuiContext>,
    ) -> Self::Widget {
        let size = calc_dimensions(&cx.config, cx.image_picker.font_size());
        Entry {
            image: self.image.map(move |i| i.into_widget(cx)),
            title: self.title,
            subtitle: self.subtitle,
            inner: self.inner,
            watch_status: self.watch_status,
            size,
            active: false,
        }
    }

    fn apply_action(
        &mut self,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        match action {
            EntryAction::Inner(a) => {
                if let Some(image) = self.image.as_mut() {
                    let None = image.apply_action(task.wrap_with(EntryWrapper), a)?;
                }
                Ok(None)
            }
            EntryAction::Command(entry_command) => Ok(self.inner.apply_command(entry_command)),
        }
    }
}

pub struct Entry {
    image: Option<JellyfinImage>,
    title: String,
    subtitle: Option<String>,
    inner: EntryData,
    watch_status: Option<Cow<'static, str>>,
    size: Size,
    active: bool,
}

impl Entry {
    pub fn data(&self) -> &EntryData {
        &self.inner
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

impl EntryState {
    pub fn data(&self) -> &EntryData {
        &self.inner
    }
    pub fn new(state: impl Into<EntryData>, cx: Pin<&mut TuiContext>) -> EntryState {
        match state.into() {
            EntryData::Item(media_item) => from_media_item(media_item, cx),
            EntryData::View(user_view) => from_user_view(user_view, cx),
        }
    }
}

#[derive(Debug)]
pub enum EntryAction {
    Inner(<JellyfinImage as JellyhajWidget>::Action),
    Command(EntryCommand),
}

#[derive(Clone, Copy)]
struct EntryWrapper;

impl Wrapper<<JellyfinImage as JellyhajWidget>::Action> for EntryWrapper {
    type F = EntryAction;

    fn wrap(&self, val: <JellyfinImage as JellyhajWidget>::Action) -> Self::F {
        EntryAction::Inner(val)
    }
}

impl ItemWidget for Entry {
    type State = EntryState;
    type Action = EntryAction;
    type ActionResult = Navigation;

    fn dimensions_static(par: jellyhaj_widgets_core::DimensionsParameter<'_>) -> Size {
        calc_dimensions(par.config, par.font_size)
    }
    fn dimensions(&self) -> Size {
        self.size
    }
    fn into_state(self) -> Self::State {
        EntryState {
            image: self.image.map(JellyfinImage::into_state),
            title: self.title,
            subtitle: self.subtitle,
            inner: self.inner,
            watch_status: self.watch_status,
        }
    }
    fn apply_action(
        &mut self,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        Ok(match action {
            EntryAction::Inner(action) => {
                if let Some(image) = self.image.as_mut() {
                    let None = image.apply_action(task.wrap_with(EntryWrapper), action)?;
                }
                None
            }
            EntryAction::Command(entry_command) => self.inner.apply_command(entry_command),
        })
    }
    fn click(
        &mut self,
        _: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        _: ratatui::prelude::Position,
        _: Size,
        kind: ratatui::crossterm::event::MouseEventKind,
        _: ratatui::crossterm::event::KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        if kind == MouseEventKind::Down(MouseButton::Left) {
            Ok(self.inner.apply_command(EntryCommand::Activate))
        } else {
            Ok(None)
        }
    }

    #[instrument(skip_all, name = "render_entry")]
    fn render_item_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
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
            image.render_fallible(inner, buf, task.wrap_with(EntryWrapper))?
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

    fn accepts_text_input(&self) -> bool {
        false
    }

    fn accept_char(&mut self, _: char) {
        unimplemented!()
    }

    fn accept_text(&mut self, _: String) {
        unimplemented!()
    }
}

fn from_media_item(item: MediaItem, mut cx: Pin<&mut TuiContext>) -> EntryState {
    let (title, subtitle) = match &item.item_type {
        ItemType::Movie | ItemType::Unknown | ItemType::CollectionFolder => {
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
        .map(|(image_type, tag)| {
            JellyfinImageState::new(item.id.clone(), tag.to_string(), image_type, cx.as_mut())
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
    EntryState {
        image,
        title,
        subtitle,
        inner: EntryData::Item(item),
        watch_status,
    }
}

fn from_user_view(item: UserView, cx: Pin<&mut TuiContext>) -> EntryState {
    let title = item.name.clone();
    let image = item
        .image_tags
        .iter()
        .flat_map(|map| map.iter())
        .next()
        .map(|(image_type, tag)| {
            JellyfinImageState::new(item.id.clone(), tag.clone(), *image_type, cx)
        });
    EntryState {
        image,
        title,
        subtitle: None,
        inner: EntryData::View(item),
        watch_status: None,
    }
}
