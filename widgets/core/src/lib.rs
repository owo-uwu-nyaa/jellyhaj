pub mod flatten;
mod item;
mod jellyhaj;
pub mod mapper;
pub mod outer;

pub use color_eyre::Result;
pub use config::Config;
pub use item::{ItemWidget, ItemWidgetExt};
pub use jellyhaj::{JellyhajWidget, JellyhajWidgetExt, TreeVisitor, WidgetTreeVisitor};
pub use jellyhaj_async_task as async_task;
pub use jellyhaj_async_task::Wrapper;
use jellyhaj_async_task::{TaskSubmitterRef, Wrapped};
pub use ratatui::{
    self,
    buffer::Buffer,
    crossterm::event::{KeyModifiers, MouseEventKind},
    layout::{Position, Rect, Size},
};
pub use ratatui_image::FontSize;
pub use spawn;
pub use valuable;

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

pub trait ContextRef<O> {
    fn as_ref(&self) -> &O;
}

pub trait GetFromContext<CX> {
    fn get_ref(cx: &CX) -> &Self;
}

impl<O, CX: ContextRef<O>> GetFromContext<CX> for O {
    #[inline]
    fn get_ref(cx: &CX) -> &Self {
        cx.as_ref()
    }
}

pub struct WidgetContext<'p, A, W: Wrapper<A>, R: 'static> {
    pub refs: &'p R,
    pub submitter: TaskSubmitterRef<'p, A, W>,
}

impl<'p, A, W: Wrapper<A> + Copy, R> Copy for WidgetContext<'p, A, W, R> {}

impl<'p, A, W: Wrapper<A> + Clone, R> Clone for WidgetContext<'p, A, W, R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'p, A, W: Wrapper<A>, R> WidgetContext<'p, A, W, R> {
    pub fn wrap_with<AN, WN: Wrapper<AN, F = A>>(
        self,
        wrapper: WN,
    ) -> WidgetContext<'p, AN, Wrapped<WN, W>, R> {
        WidgetContext {
            submitter: self.submitter.wrap_with(wrapper),
            refs: self.refs,
        }
    }
}
