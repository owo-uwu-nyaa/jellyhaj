use std::{fmt::Debug, pin::Pin};

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyModifiers, MouseEventKind},
    layout::{Position, Rect, Size},
};

use crate::{
    DimensionsParameter, JellyhajWidget, WidgetContext, WidgetTreeVisitor, Wrapper,
    jellyhaj::JellyhajWidgetState,
};
use color_eyre::{Result, eyre::ensure};

pub trait ItemState: Debug + Send + 'static {
    type IAction: Debug + Send + 'static;
    type IActionResult: Debug;
    type IWidget: ItemWidget<IState = Self, IAction = Self::IAction, IActionResult = Self::IActionResult>;

    const NAME: &str;

    fn item_visit_children(visitor: &mut impl WidgetTreeVisitor);

    fn item_into_widget(self, cx: Pin<&mut jellyhaj_context::TuiContext>) -> Self::IWidget;
    fn item_apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::IAction, impl Wrapper<Self::IAction>>,
        action: Self::IAction,
    ) -> Result<Option<Self::IActionResult>>;
}

impl<I: ItemState> JellyhajWidgetState for I {
    type Action = <Self as ItemState>::IAction;

    type ActionResult = <Self as ItemState>::IActionResult;

    type Widget = <Self as ItemState>::IWidget;

    const NAME: &str = <Self as ItemState>::NAME;

    fn visit_children(visitor: &mut impl WidgetTreeVisitor) {
        Self::item_visit_children(visitor);
    }

    fn into_widget(self, cx: Pin<&mut jellyhaj_context::TuiContext>) -> Self::Widget {
        self.item_into_widget(cx)
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        self.item_apply_action(cx, action)
    }
}

pub trait ItemWidget: Send + Sized + 'static {
    type IAction: Debug + Send + 'static;
    type IActionResult: Debug;
    type IState: ItemState<IWidget = Self, IAction = Self::IAction, IActionResult = Self::IActionResult>;

    fn dimensions(&self) -> Size;
    fn dimensions_static(par: DimensionsParameter<'_>) -> Size;

    fn item_into_state(self) -> Self::IState;

    fn item_accepts_text_input(&self) -> bool;
    fn item_accept_char(&mut self, text: char);
    fn item_accept_text(&mut self, text: String);

    fn set_active(&mut self, active: bool);

    fn item_apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::IAction, impl Wrapper<Self::IAction>>,
        action: Self::IAction,
    ) -> Result<Option<Self::IActionResult>>;

    fn item_click(
        &mut self,
        cx: WidgetContext<'_, Self::IAction, impl Wrapper<Self::IAction>>,
        position: Position,
        size: Size,
        kind: MouseEventKind,
        modifier: KeyModifiers,
    ) -> Result<Option<Self::IActionResult>>;

    fn render_item_inner(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        cx: WidgetContext<'_, Self::IAction, impl Wrapper<Self::IAction>>,
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

    type State = <I as ItemWidget>::IState;
    type Action = <I as ItemWidget>::IAction;
    type ActionResult = <I as ItemWidget>::IActionResult;

    #[inline(always)]
    fn into_state(self) -> Self::State {
        self.item_into_state()
    }

    #[inline(always)]
    fn render_fallible_inner(
        &mut self,
        mut area: Rect,
        buf: &mut Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
    ) -> Result<()> {
        fn inner(dim: Size, area: &mut Rect) -> Result<()> {
            assert!(dim.width <= area.width, "width is too small");
            assert!(dim.height <= area.height, "height is too small");
            *area = area.resize(dim);
            ensure!(dim.width == area.width, "width is too large for position");
            ensure!(dim.height == area.height, "width is too large for position");
            Ok(())
        }
        inner(self.dimensions(), &mut area)?;
        Self::render_item_inner(self, area, buf, cx)
    }

    #[inline(always)]
    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        self.item_apply_action(cx, action)
    }

    #[inline(always)]
    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
        position: Position,
        size: Size,
        kind: MouseEventKind,
        modifier: KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        self.item_click(cx, position, size, kind, modifier)
    }

    #[inline(always)]
    fn accepts_text_input(&self) -> bool {
        self.item_accepts_text_input()
    }

    #[inline(always)]
    fn accept_char(&mut self, text: char) {
        self.item_accept_char(text);
    }

    #[inline(always)]
    fn accept_text(&mut self, text: String) {
        self.item_accept_text(text);
    }
}
