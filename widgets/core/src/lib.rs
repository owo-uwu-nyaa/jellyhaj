pub mod flatten;
mod item;
mod jellyhaj;
pub mod mapper;
pub mod outer;
use std::sync::Arc;

pub use color_eyre::Result;
pub use config::Config;
pub use item::{ItemState, ItemWidget};
pub use jellyhaj::{
    DimensionsParameter, JellyhajWidget, JellyhajWidgetExt, JellyhajWidgetState, TreeVisitor,
    WidgetTreeVisitor,
};
pub use jellyhaj_async_task as async_task;
pub use jellyhaj_async_task::Wrapper;
use jellyhaj_async_task::{TaskSubmitterRef, Wrapped};
pub use jellyhaj_context::{Auth, JellyfinClient, JellyfinEventInterests, TuiContext};
use jellyhaj_context::{DB, ImageProtocolCache, Picker, Stats};
pub use ratatui::{
    self,
    buffer::Buffer,
    crossterm::event::{KeyModifiers, MouseEventKind},
    layout::{Position, Rect, Size},
};
pub use ratatui_image::FontSize;
pub use spawn;

pub trait RectExt {
    fn contains(self, pos: Position) -> bool;
}

impl RectExt for Rect {
    fn contains(self, pos: Position) -> bool {
        self.x <= pos.x
            && self.y <= pos.y
            && self.x + self.width > pos.x
            && self.y + self.height > pos.y
    }
}

pub struct WidgetContext<'p, A, W: Wrapper<A>> {
    pub config: &'p Arc<Config>,
    pub image_picker: &'p Arc<Picker>,
    pub cache: &'p DB,
    pub image_cache: &'p ImageProtocolCache,
    pub stats: &'p Stats,
    pub jellyfin_events: &'p JellyfinEventInterests,
    pub submitter: TaskSubmitterRef<'p, A, W>,
}

impl<'p, A, W: Wrapper<A> + Copy> Copy for WidgetContext<'p, A, W> {}

impl<'p, A, W: Wrapper<A> + Clone> Clone for WidgetContext<'p, A, W> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'p, A, W: Wrapper<A>> WidgetContext<'p, A, W> {
    pub fn wrap_with<AN, WN: Wrapper<AN, F = A>>(
        self,
        wrapper: WN,
    ) -> WidgetContext<'p, AN, Wrapped<WN, W>> {
        WidgetContext {
            submitter: self.submitter.wrap_with(wrapper),
            config: self.config,
            image_picker: self.image_picker,
            cache: self.cache,
            image_cache: self.image_cache,
            jellyfin_events: self.jellyfin_events,
            stats: self.stats,
        }
    }
}
