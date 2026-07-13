use std::fmt::Debug;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyModifiers, MouseEventKind},
    layout::{Position, Rect, Size},
};
use tracing::instrument;
use valuable::Valuable;

use crate::{JellyhajWidgetBase, WidgetContext, WidgetTreeVisitor, Wrapper};
use color_eyre::Result;

pub trait ItemWidgetBase: Valuable + Send + Sized + 'static {
    type Action: Debug + Send + 'static;
    type ActionResult: Debug;

    const NAME: &str;

    fn visit_children(&self, visitor: &mut impl WidgetTreeVisitor);

    fn dimensions(&self) -> Size;

    fn accepts_text_input(&self) -> bool {
        false
    }
    fn accept_char(&mut self, _text: char) {
        unimplemented!()
    }
    fn accept_text(&mut self, _text: String) {
        unimplemented!()
    }

    fn set_active(&mut self, active: bool);
}

pub trait ItemWidget<R: 'static>: ItemWidgetBase {
    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>);

    fn dimensions_static(par: &R) -> Size;

    fn item_apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>>;

    fn item_click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        position: Position,
        size: Size,
        kind: MouseEventKind,
        modifier: KeyModifiers,
    ) -> Result<Option<Self::ActionResult>>;

    fn render_item_inner(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()>;
}

pub trait ItemWidgetExt<R: 'static>: ItemWidget<R> {
    fn render_item(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()> {
        #[instrument(name = "check_item_size")]
        fn inner(dim: Size, area: Rect) {
            assert!(dim.width <= area.width, "width is too small");
            assert!(dim.height <= area.height, "height is too small");
            assert!(dim.width == area.width, "width is too large for position");
            assert!(dim.height == area.height, "width is too large for position");
        }
        inner(self.dimensions(), area);
        self.render_item_inner(area, buf, cx)
    }
}

impl<R: 'static, I: ItemWidget<R>> ItemWidgetExt<R> for I {}

impl<I: ItemWidgetBase> JellyhajWidgetBase for I {
    type Action = I::Action;

    type ActionResult = I::ActionResult;

    const NAME: &str = I::NAME;

    #[inline]
    fn visit_children(&self, visitor: &mut impl WidgetTreeVisitor) {
        ItemWidgetBase::visit_children(self, visitor);
    }

    #[inline]
    fn min_width(&self) -> Option<u16> {
        Some(self.dimensions().width)
    }

    #[inline]
    fn min_height(&self) -> Option<u16> {
        Some(self.dimensions().height)
    }

    #[inline]
    fn accepts_text_input(&self) -> bool {
        ItemWidgetBase::accepts_text_input(self)
    }

    #[inline]
    fn accept_char(&mut self, text: char) {
        ItemWidgetBase::accept_char(self, text);
    }

    #[inline]
    fn accept_text(&mut self, text: String) {
        ItemWidgetBase::accept_text(self, text);
    }
}
