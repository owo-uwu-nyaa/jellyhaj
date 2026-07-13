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

use crate::WidgetContext;
use valuable::Valuable;

pub trait TreeVisitor {
    fn enter(
        &mut self,
        name: &'static str,
        state: &dyn Valuable,
        visit_children: &dyn Fn(&mut dyn TreeVisitor),
    );
}

pub trait WidgetTreeVisitor: Sized {
    fn visit<S: JellyhajWidgetBase>(&mut self, state: &S);
}

impl WidgetTreeVisitor for &mut dyn TreeVisitor {
    fn visit<S: JellyhajWidgetBase>(&mut self, state: &S) {
        self.enter(S::NAME, state, &|mut this| {
            state.visit_children(&mut this);
        });
    }
}

pub trait JellyhajWidgetBase: Valuable + Send + Sized + 'static {
    type Action: Debug + Send + 'static;
    type ActionResult: Debug;

    const NAME: &str;

    fn visit_children(&self, visitor: &mut impl WidgetTreeVisitor);

    fn min_width(&self) -> Option<u16>;
    fn min_height(&self) -> Option<u16>;

    fn accepts_text_input(&self) -> bool {
        false
    }
    fn accept_char(&mut self, _text: char) {
        unimplemented!()
    }
    fn accept_text(&mut self, _text: String) {
        unimplemented!()
    }
}

pub trait JellyhajWidget<R: 'static>: JellyhajWidgetBase {
    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>);

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
