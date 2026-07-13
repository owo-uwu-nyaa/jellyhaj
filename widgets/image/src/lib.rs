mod fetch;

use image::DynamicImage;
pub use image_cache as cache;
pub use image_cache::ImageSize;
use jellyhaj_core::context::DB;
use std::{cmp::min, convert::Infallible};
use valuable::Valuable;

use image_cache::{ImageCache, ImageKey, ImageProtocolKeyRef};

use crate::fetch::get_image;
use color_eyre::eyre::Context;
pub use jellyfin::{JellyfinClient, items::ImageType};
use jellyhaj_widgets_core::{
    ContextRef, GetFromContext, JellyhajWidget, JellyhajWidgetBase, WidgetContext, Wrapper,
};
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

#[derive(Valuable)]
pub struct JellyfinImage {
    #[valuable(skip)]
    size: Size,
    image_type: ImageType,
    item_id: String,
    tag: String,
    loading: bool,
    image: Option<ImageProtocol>,
}

#[derive(Valuable)]
struct ImageProtocol {
    #[valuable(skip)]
    protocol: Protocol,
    size: ImageSize,
}

impl JellyfinImage {
    #[must_use]
    pub const fn new(item_id: String, tag: String, image_type: ImageType) -> Self {
        Self {
            size: Size::ZERO,
            loading: false,
            image: None,
            image_type,
            item_id,
            tag,
        }
    }

    fn get_image<
        R: ContextRef<Picker>
            + ContextRef<Stats>
            + ContextRef<JellyfinClient>
            + ContextRef<DB>
            + ContextRef<ImageCache>,
    >(
        &mut self,
        cx: WidgetContext<'_, ParsedImage, impl Wrapper<ParsedImage>, R>,
    ) -> Option<&Protocol> {
        if self.image.is_some() {
            self.image.as_ref().map(|p| &p.protocol)
        } else {
            let image_picker: &Picker = cx.refs.as_ref();
            let p_height = u32::from(self.size.height) * u32::from(image_picker.font_size().height);
            let p_width = u32::from(self.size.width) * u32::from(image_picker.font_size().width);
            if self.loading {
                None
            } else {
                let image_size = ImageSize { p_width, p_height };
                let cached = ImageCache::get_ref(cx.refs).get(&ImageProtocolKeyRef::new(
                    self.image_type,
                    &self.item_id,
                    &self.tag,
                    image_size,
                ));
                if let Some(image) = cached {
                    let picker = Picker::get_ref(cx.refs);
                    let (width, height) = self.calc_dimensions(&image, picker);
                    let protocol = match picker
                        .new_protocol(image, Size { width, height }, Resize::Fit(None))
                        .context("generating protocol")
                    {
                        Ok(p) => p,
                        Err(e) => {
                            tracing::error!("error creating protocol: {e:?}");
                            return None;
                        }
                    };
                    Some(
                        &self
                            .image
                            .insert(ImageProtocol {
                                protocol,
                                size: image_size,
                            })
                            .protocol,
                    )
                } else {
                    self.loading = true;
                    let key = ImageKey {
                        image_type: self.image_type,
                        item_id: self.item_id.clone(),
                        tag: self.tag.clone(),
                        size: image_size,
                    };
                    let db = DB::get_ref(cx.refs).clone();
                    let jellyfin = JellyfinClient::get_ref(cx.refs).clone();
                    let size = self.size;
                    let stats = Stats::get_ref(cx.refs).clone();
                    let cache = ImageCache::get_ref(cx.refs).clone();
                    cx.submitter.spawn_task_suppressed_error(
                        async move { get_image(key, db, jellyfin, size, stats, cache).await },
                        info_span!("get_image"),
                        "get_image",
                    );
                    None
                }
            }
        }
    }

    fn calc_dimensions(&self, image: &DynamicImage, picker: &Picker) -> (u16, u16) {
        let width = u16::try_from(min(
            u32::from(self.size.width),
            image.width().div_ceil(u32::from(picker.font_size().width)),
        ))
        .expect("width center calc failed");
        let height = u16::try_from(min(
            u32::from(self.size.height),
            image
                .height()
                .div_ceil(u32::from(picker.font_size().height)),
        ))
        .expect("height center calc failed");
        (width, height)
    }
}

impl JellyhajWidgetBase for JellyfinImage {
    type Action = ParsedImage;

    type ActionResult = Infallible;

    const NAME: &str = "image";

    fn visit_children(&self, _visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {}

    fn min_width(&self) -> Option<u16> {
        Some(1)
    }
    fn min_height(&self) -> Option<u16> {
        Some(1)
    }
}

impl<
    R: ContextRef<Picker>
        + ContextRef<Stats>
        + ContextRef<JellyfinClient>
        + ContextRef<DB>
        + ContextRef<ImageCache>
        + 'static,
> JellyhajWidget<R> for JellyfinImage
{
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
            self.image = None;
        }
        if let Some(image) = self.get_image(cx) {
            area.x += (area.width - new_size.width) / 2;
            area.y += (area.height - new_size.height) / 2;
            area.width = new_size.width;
            area.height = new_size.height;
            Image::new(image).render(area, buf);
        }
        Ok(())
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        self.loading = false;
        if action.size == self.size {
            let picker = Picker::get_ref(cx.refs);
            let (width, height) = self.calc_dimensions(&action.image, picker);
            let protocol = picker
                .new_protocol(action.image, Size { width, height }, Resize::Fit(None))
                .context("generating protocol")?;
            self.image = Some(ImageProtocol {
                protocol,
                size: action.image_size,
            });
        }
        Ok(None)
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
}
