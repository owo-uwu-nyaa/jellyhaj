use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyModifiers, MouseEventKind},
    layout::{Position, Rect, Size},
};

use crate::{DimensionsParameter, JellyhajWidget, Wrapper, async_task::TaskSubmitter};
use color_eyre::{Result, eyre::ensure};

pub trait ItemWidget {
    type State;
    type Action: Send + 'static;
    type ActionResult;

    fn dimensions(&self) -> Size;
    fn dimensions_static(par: DimensionsParameter<'_>) -> Size;

    fn into_state(self) -> Self::State;

    fn accepts_text_input(&self) -> bool;
    fn accept_char(&mut self, text: char);
    fn accept_text(&mut self, text: String);

    fn set_active(&mut self, active: bool);

    fn apply_action(&mut self, action: Self::Action) -> Result<Option<Self::ActionResult>>;
    fn click(
        &mut self,
        position: Position,
        size: Size,
        kind: MouseEventKind,
        modifier: KeyModifiers,
    ) -> Result<Option<Self::ActionResult>>;

    fn render_item_inner(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
    ) -> Result<()>;
}

impl<I: ItemWidget> JellyhajWidget for I {
    #[inline(always)]
    fn min_width(&self) -> Option<u16> {
        Some(self.dimensions().width)
    }

    #[inline(always)]
    fn min_height(&self) -> Option<u16> {
        Some(self.dimensions().height)
    }

    #[inline(always)]
    fn min_width_static(par: DimensionsParameter<'_>) -> Option<u16> {
        Some(Self::dimensions_static(par).width)
    }

    #[inline(always)]
    fn min_height_static(par: DimensionsParameter<'_>) -> Option<u16> {
        Some(Self::dimensions_static(par).height)
    }

    type State = <I as ItemWidget>::State;
    type Action = <I as ItemWidget>::Action;
    type ActionResult = <I as ItemWidget>::ActionResult;

    #[inline(always)]
    fn into_state(self) -> Self::State {
        ItemWidget::into_state(self)
    }

    fn render_fallible_inner(
        &mut self,
        mut area: Rect,
        buf: &mut Buffer,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
    ) -> Result<()> {
        let dim = self.dimensions();
        assert!(dim.width <= area.width, "width is too small");
        assert!(dim.height <= area.height, "height is too small");
        area = area.resize(dim);
        ensure!(dim.width == area.width, "width is too large for position");
        ensure!(dim.height == area.height, "width is too large for position");
        Self::render_item_inner(self, area, buf, task)
    }

    #[inline(always)]
    fn apply_action(&mut self, action: Self::Action) -> Result<Option<Self::ActionResult>> {
        ItemWidget::apply_action(self, action)
    }

    #[inline(always)]
    fn click(
        &mut self,
        position: Position,
        size: Size,
        kind: MouseEventKind,
        modifier: KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        ItemWidget::click(self, position, size, kind, modifier)
    }

    #[inline(always)]
    fn accepts_text_input(&self) -> bool {
        ItemWidget::accepts_text_input(self)
    }

    #[inline(always)]
    fn accept_char(&mut self, text: char) {
        ItemWidget::accept_char(self, text);
    }

    #[inline(always)]
    fn accept_text(&mut self, text: String) {
        ItemWidget::accept_text(self, text);
    }
}
