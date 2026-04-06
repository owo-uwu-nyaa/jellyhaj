mod fetch;

pub use image_cache as cache;
pub use image_cache::ImageSize;
use jellyhaj_core::context::DB;
use std::{cmp::min, convert::Infallible, mem};
use valuable::{Fields, NamedField, NamedValues, StructDef, Structable, Valuable, Value};

use image_cache::{ImageProtocolCache, ImageProtocolKey, ImageProtocolKeyRef};

use crate::fetch::get_image;
use color_eyre::eyre::Context;
pub use jellyfin::{JellyfinClient, items::ImageType};
use jellyhaj_widgets_core::{ContextRef, GetFromContext, JellyhajWidget, WidgetContext, Wrapper};
use ratatui::{
    layout::{Rect, Size},
    widgets::Widget,
};
pub use ratatui_image::picker::Picker;
use ratatui_image::{Image, Resize, protocol::Protocol};
pub use sqlx::SqliteConnection;
pub use stats_data::Stats;
pub use tokio;
use tracing::info_span;

pub use fetch::ParsedImage;

struct ImageCacher {
    image: Option<(Protocol, ImageSize)>,
    cache: ImageProtocolCache,
    image_type: ImageType,
    item_id: String,
    tag: String,
}

static IMAGE_CACHER_FIELDS: &[NamedField] = &[
    NamedField::new("image"),
    NamedField::new("image_type"),
    NamedField::new("item_id"),
    NamedField::new("tag"),
];

static IMAGE_NONE: &Option<&str> = &None;
static IMAGE_SOME: &Option<&str> = &Some("image not inspectable");

impl Valuable for ImageCacher {
    fn as_value(&self) -> Value<'_> {
        Value::Structable(self)
    }

    fn visit(&self, visit: &mut dyn valuable::Visit) {
        visit.visit_named_fields(&NamedValues::new(
            IMAGE_CACHER_FIELDS,
            &[
                if self.image.is_none() {
                    IMAGE_NONE
                } else {
                    IMAGE_SOME
                }
                .as_value(),
                self.image_type.as_value(),
                self.item_id.as_value(),
                self.tag.as_value(),
            ],
        ));
    }
}

impl Structable for ImageCacher {
    fn definition(&self) -> StructDef<'_> {
        StructDef::new_static("ImageCacher", Fields::Named(IMAGE_CACHER_FIELDS))
    }
}

impl std::fmt::Debug for ImageCacher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageCacher")
            .field("image", &if self.image.is_some() { "Some" } else { "None" })
            .field("image_type", &self.image_type)
            .field("item_id", &self.item_id)
            .field("tag", &self.tag)
            .finish()
    }
}

#[derive(Valuable)]
pub struct JellyfinImage {
    #[valuable(skip)]
    size: Size,
    loading: bool,
    image: ImageCacher,
}

impl Drop for ImageCacher {
    fn drop(&mut self) {
        if let Some((protocol, size)) = self.image.take() {
            let key = ImageProtocolKey {
                image_type: self.image_type,
                item_id: mem::take(&mut self.item_id),
                tag: mem::take(&mut self.tag),
                size,
            };
            self.cache.store(protocol, key);
        }
    }
}

impl JellyfinImage {
    pub fn new(
        item_id: String,
        tag: String,
        image_type: ImageType,
        cx: &impl ContextRef<ImageProtocolCache>,
    ) -> Self {
        Self {
            size: Size::ZERO,
            loading: false,
            image: ImageCacher {
                image: None,
                cache: cx.as_ref().clone(),
                image_type,
                item_id,
                tag,
            },
        }
    }

    fn get_image<
        R: ContextRef<Picker> + ContextRef<Stats> + ContextRef<JellyfinClient> + ContextRef<DB>,
    >(
        &mut self,
        cx: WidgetContext<'_, ParsedImage, impl Wrapper<ParsedImage>, R>,
    ) -> Option<&Protocol> {
        if self.image.image.is_some() {
            self.image.image.as_ref().map(|(p, _)| p)
        } else {
            let image_picker: &Picker = cx.refs.as_ref();
            let p_height = (self.size.height as u32) * (image_picker.font_size().1 as u32);
            let p_width = (self.size.width as u32) * (image_picker.font_size().0 as u32);
            if !self.loading {
                let image_size = ImageSize { p_width, p_height };
                let cached = self.image.cache.remove(&ImageProtocolKeyRef::new(
                    self.image.image_type,
                    &self.image.item_id,
                    &self.image.tag,
                    image_size,
                ));
                if let Some(image) = cached {
                    Some(&self.image.image.insert((image, image_size)).0)
                } else {
                    let key = ImageProtocolKey {
                        image_type: self.image.image_type,
                        item_id: self.image.item_id.clone(),
                        tag: self.image.tag.clone(),
                        size: image_size,
                    };
                    let db = DB::get_ref(cx.refs).clone();
                    let jellyfin = JellyfinClient::get_ref(cx.refs).clone();
                    let size = self.size;
                    let stats = Stats::get_ref(cx.refs).clone();
                    cx.submitter.spawn_task_suppressed_error(
                        async move { get_image(key, db, jellyfin, size, stats).await },
                        info_span!("get_image"),
                        "get_image",
                    );
                    None
                }
            } else {
                None
            }
        }
    }
}

fn add_image<
    R: ContextRef<Picker> + ContextRef<Stats> + ContextRef<JellyfinClient> + ContextRef<DB>,
>(
    loading: &mut bool,
    size: Size,
    image_out: &mut Option<(Protocol, ImageSize)>,
    cx: WidgetContext<'_, ParsedImage, impl Wrapper<ParsedImage>, R>,
    action: ParsedImage,
) -> Result<Option<Infallible>, color_eyre::eyre::Error> {
    *loading = false;
    if action.size == size {
        let picker = Picker::get_ref(cx.refs);
        let width = min(
            size.width as u32,
            action.image.width().div_ceil(picker.font_size().0 as u32),
        ) as u16;
        let height = min(
            size.height as u32,
            action.image.height().div_ceil(picker.font_size().1 as u32),
        ) as u16;
        let image = picker
            .new_protocol(
                action.image,
                Rect {
                    x: 0,
                    y: 0,
                    width,
                    height,
                },
                Resize::Fit(None),
            )
            .context("generating protocol")?;
        *image_out = Some((image, action.image_size));
    }
    Ok(None)
}

impl<
    R: ContextRef<Picker> + ContextRef<Stats> + ContextRef<JellyfinClient> + ContextRef<DB> + 'static,
> JellyhajWidget<R> for JellyfinImage
{
    type Action = ParsedImage;

    type ActionResult = Infallible;

    const NAME: &str = "image";

    fn visit_children(&self, _visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {}

    fn init(&mut self, _cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {}

    fn render_fallible_inner(
        &mut self,
        mut area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> jellyhaj_widgets_core::Result<()> {
        let new_size = area.as_size();
        let old_size = self.size;
        if new_size != old_size {
            self.size = new_size;
            self.image.image = None;
        }
        if let Some(image) = self.get_image(cx) {
            area.x += (area.width - new_size.width) / 2;
            area.y += (area.height - new_size.height) / 2;
            area.width = new_size.width;
            area.height = new_size.height;
            Image::new(image).render(area, buf)
        }
        Ok(())
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        add_image(
            &mut self.loading,
            self.size,
            &mut self.image.image,
            cx,
            action,
        )
    }

    fn click(
        &mut self,
        _: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        _: ratatui::prelude::Position,
        _: Size,
        _: ratatui::crossterm::event::MouseEventKind,
        _: ratatui::crossterm::event::KeyModifiers,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        Ok(None)
    }

    fn min_width(&self) -> Option<u16> {
        Some(1)
    }
    fn min_height(&self) -> Option<u16> {
        Some(1)
    }
    #[inline(always)]
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
