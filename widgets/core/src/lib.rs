pub mod flatten;
mod item;
mod jellyhaj;
pub mod mapper;
pub mod outer;

use std::{
    fmt::Debug,
    ops::{BitOrAssign, Deref},
};

pub use color_eyre::Result;
pub use config::Config;
pub use item::{ItemWidget, ItemWidgetBase, ItemWidgetExt};
pub use jellyhaj::{
    JellyhajWidget, JellyhajWidgetBase, JellyhajWidgetExt, TreeVisitor, WidgetTreeVisitor,
};
pub use jellyhaj_async_task as async_task;
pub use jellyhaj_async_task::Wrapper;
use jellyhaj_async_task::{TaskSubmitterRef, Wrapped};
use ratatui::crossterm::event::KeyEvent;
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

impl<A, W: Wrapper<A> + Copy, R> Copy for WidgetContext<'_, A, W, R> {}

impl<A, W: Wrapper<A> + Clone, R> Clone for WidgetContext<'_, A, W, R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'p, A, W: Wrapper<A>, R> WidgetContext<'p, A, W, R> {
    pub const fn wrap_with<AN, WN: Wrapper<AN, F = A>>(
        self,
        wrapper: WN,
    ) -> WidgetContext<'p, AN, Wrapped<WN, W>, R> {
        WidgetContext {
            submitter: self.submitter.wrap_with(wrapper),
            refs: self.refs,
        }
    }
    pub const fn with_cx<'pn, RN: 'static>(self, r: &'pn RN) -> WidgetContext<'pn, A, W, RN>
    where
        'p: 'pn,
    {
        WidgetContext {
            refs: r,
            submitter: self.submitter,
        }
    }
}

pub struct RenderFlag {
    should_render: bool,
}

impl RenderFlag {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            should_render: false,
        }
    }

    #[inline]
    pub const fn set(&mut self) {
        self.should_render = true;
    }

    #[must_use]
    pub const fn reset(&mut self) -> bool {
        std::mem::replace(&mut self.should_render, false)
    }
}

impl Default for RenderFlag {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for RenderFlag {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.should_render
    }
}

impl BitOrAssign<bool> for RenderFlag {
    #[inline]
    fn bitor_assign(&mut self, rhs: bool) {
        self.should_render |= rhs;
    }
}

#[derive(Debug)]
pub enum KeybindAction<A: Debug + Send + 'static> {
    Inner(A),
    Key(KeyEvent),
}
