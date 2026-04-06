use std::fmt::Debug;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyModifiers, MouseEventKind},
    layout::{Position, Rect, Size},
};
use tracing::instrument;

use crate::{WidgetContext, WidgetTreeVisitor, Wrapper};
use color_eyre::Result;
use valuable::Valuable;

pub trait ItemWidget<R: 'static>: Valuable + Send + Sized + 'static {
    type IAction: Debug + Send + 'static;
    type IActionResult: Debug;

    const NAME: &str;

    fn visit_children(&self, visitor: &mut impl WidgetTreeVisitor);

    fn init(&mut self, cx: WidgetContext<'_, Self::IAction, impl Wrapper<Self::IAction>, R>);

    fn dimensions(&self) -> Size;
    fn dimensions_static(par: &R) -> Size;

    fn item_accepts_text_input(&self) -> bool;
    fn item_accept_char(&mut self, text: char);
    fn item_accept_text(&mut self, text: String);

    fn set_active(&mut self, active: bool);

    fn item_apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::IAction, impl Wrapper<Self::IAction>, R>,
        action: Self::IAction,
    ) -> Result<Option<Self::IActionResult>>;

    fn item_click(
        &mut self,
        cx: WidgetContext<'_, Self::IAction, impl Wrapper<Self::IAction>, R>,
        position: Position,
        size: Size,
        kind: MouseEventKind,
        modifier: KeyModifiers,
    ) -> Result<Option<Self::IActionResult>>;

    fn render_item_inner(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        cx: WidgetContext<'_, Self::IAction, impl Wrapper<Self::IAction>, R>,
    ) -> Result<()>;
}

pub trait ItemWidgetExt<R: 'static>: ItemWidget<R> {
    fn render_item(
        &mut self,
        mut area: Rect,
        buf: &mut Buffer,
        cx: WidgetContext<'_, Self::IAction, impl Wrapper<Self::IAction>, R>,
    ) -> Result<()> {
        #[instrument(name = "check_item_size")]
        fn inner(dim: Size, area: &mut Rect) {
            assert!(dim.width <= area.width, "width is too small");
            assert!(dim.height <= area.height, "height is too small");
            assert!(dim.width == area.width, "width is too large for position");
            assert!(dim.height == area.height, "width is too large for position");
        }
        inner(self.dimensions(), &mut area);
        self.render_item_inner(area, buf, cx)
    }
}

impl<R: 'static, I: ItemWidget<R>> ItemWidgetExt<R> for I {}
