use color_eyre::Result;
use config::Config;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyModifiers, MouseEventKind},
    layout::{Position, Rect, Size},
    widgets::{Paragraph, Widget, Wrap},
};
use ratatui_image::FontSize;
use std::any::type_name;
use tracing::warn;

use crate::async_task::TaskSubmitter;

pub struct DimensionsParameter<'c> {
    pub config: &'c Config,
    pub font_size: FontSize,
}

pub trait Wrapper<C>: Clone + Copy + Send + Sync + 'static {
    type F: Send + 'static;
    fn wrap(&self, val: C) -> Self::F;
}

impl<A, R: Send + 'static, F: Clone + Copy + Send + Sync + 'static + Fn(A) -> R> Wrapper<A> for F {
    type F = R;
    fn wrap(&self, val: A) -> Self::F {
        self(val)
    }
}

pub trait JellyhajWidget {
    type State;
    type Action: Send + 'static;
    type ActionResult;
    fn min_width(&self) -> Option<u16>;
    fn min_height(&self) -> Option<u16>;

    fn into_state(self) -> Self::State;

    fn accepts_text_input(&self) -> bool;
    fn accept_char(&mut self, text: char);
    fn accept_text(&mut self, text: String);

    fn apply_action(
        &mut self,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>>;
    fn click(
        &mut self,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        position: Position,
        size: Size,
        kind: MouseEventKind,
        modifier: KeyModifiers,
    ) -> Result<Option<Self::ActionResult>>;

    fn render_fallible_inner(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
    ) -> Result<()>;
}

pub trait JellyhajWidgetExt: JellyhajWidget {
    fn render_fallible(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
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
            self.render_fallible_inner(area, buf, task)
        } else {
            Ok(())
        }
    }
}

impl<W: JellyhajWidget> JellyhajWidgetExt for W {}
