use color_eyre::Result;
use jellyhaj_async_task::Wrapper;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyModifiers, MouseEventKind},
    layout::{Position, Rect, Size},
    widgets::{Paragraph, Widget, Wrap},
};
use std::any::type_name;
use std::fmt::Debug;
use tracing::warn;

use crate::{ItemState, WidgetContext};

pub trait TreeVisitor {
    fn enter(&mut self, name: &'static str, visit_children: fn(&mut dyn TreeVisitor));
}

pub trait WidgetTreeVisitor: Sized {
    fn visit<R: 'static, S: JellyhajWidgetState<R>>(&mut self);
    fn visit_item<R: 'static, S: ItemState<R>>(&mut self);
}

impl WidgetTreeVisitor for &mut dyn TreeVisitor {
    fn visit<R: 'static, S: JellyhajWidgetState<R>>(&mut self) {
        self.enter(S::NAME, |mut this| {
            S::visit_children(&mut this);
        });
    }

    fn visit_item<R: 'static, S: ItemState<R>>(&mut self) {
        self.enter(S::NAME, |mut this| {
            S::item_visit_children(&mut this);
        });
    }
}

pub trait JellyhajWidgetState<R: 'static>: Debug + Send + 'static {
    type Action: Debug + Send + 'static;
    type ActionResult: Debug;
    type Widget: JellyhajWidget<R, State = Self, Action = Self::Action, ActionResult = Self::ActionResult>;

    const NAME: &str;

    fn visit_children(visitor: &mut impl WidgetTreeVisitor);

    fn into_widget(self, cx: &R) -> Self::Widget;
    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>>;
}

pub trait JellyhajWidget<R: 'static>: Send + Sized + 'static {
    type Action: Debug + Send + 'static;
    type ActionResult: Debug;
    type State: JellyhajWidgetState<
            R,
            Widget = Self,
            Action = Self::Action,
            ActionResult = Self::ActionResult,
        >;

    fn min_width(&self) -> Option<u16>;
    fn min_height(&self) -> Option<u16>;

    fn into_state(self) -> Self::State;

    fn accepts_text_input(&self) -> bool;
    fn accept_char(&mut self, text: char);
    fn accept_text(&mut self, text: String);

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>>;

    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        position: Position,
        size: Size,
        kind: MouseEventKind,
        modifier: KeyModifiers,
    ) -> Result<Option<Self::ActionResult>>;

    fn render_fallible_inner(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()>;
}

pub trait JellyhajWidgetExt<R: 'static>: JellyhajWidget<R> {
    fn render_fallible(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()> {
        fn size_ok(
            widget: &'static str,
            min_width: Option<u16>,
            min_height: Option<u16>,
            area: Rect,
            buf: &mut Buffer,
        ) -> bool {
            if let Some(min_width) = min_width
                && min_width > area.width
            {
                let message = if let Some(min_height) = min_height
                    && min_height > area.height
                {
                    format!("Terminal not large enough. Requires at least {min_width}x{min_height}")
                } else {
                    format!("Terminal not large enough. Requires a width of at least {min_width}")
                };
                warn!(widget, "{message}");
                Paragraph::new(message)
                    .wrap(Wrap { trim: true })
                    .render(area, buf);
                false
            } else if let Some(min_height) = min_height
                && min_height > area.height
            {
                let message = format!(
                    "Terminal not large enough. Requires a height of at least {min_height}"
                );
                warn!(widget, "{message}");
                Paragraph::new(message)
                    .wrap(Wrap { trim: true })
                    .render(area, buf);
                false
            } else {
                true
            }
        }
        let min_width = self.min_width();
        let min_height = self.min_height();
        if (min_width.is_none() && min_height.is_none())
            || size_ok(type_name::<Self>(), min_width, min_height, area, buf)
        {
            self.render_fallible_inner(area, buf, cx)
        } else {
            Ok(())
        }
    }
}

impl<R: 'static, W: JellyhajWidget<R>> JellyhajWidgetExt<R> for W {}
