use std::{cmp::min, convert::Infallible};

use ansi_to_tui::IntoText;
use color_eyre::eyre::Context;
use jellyhaj_widgets_core::{JellyhajWidget, WidgetContext, Wrapper};
use ratatui::{
    layout::Margin,
    widgets::{
        Block, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Widget,
    },
};
use valuable::Valuable;

#[derive(Valuable)]
pub struct ErrorWidget {
    text: String,
    #[valuable(skip)]
    pos_x: u16,
    #[valuable(skip)]
    pos_y: u16,
}

impl ErrorWidget {
    #[must_use]
    pub const fn new(text: String) -> Self {
        Self {
            text,
            pos_x: 0,
            pos_y: 0,
        }
    }
}

#[derive(Debug)]
pub enum ErrorAction {
    Up,
    Down,
    Left,
    Right,
}

impl<R: 'static> JellyhajWidget<R> for ErrorWidget {
    type Action = ErrorAction;

    type ActionResult = Infallible;

    const NAME: &str = "error";

    fn visit_children(&self, _visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {}

    fn init(&mut self, _cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {}

    fn min_width(&self) -> Option<u16> {
        Some(5)
    }

    fn min_height(&self) -> Option<u16> {
        Some(5)
    }

    fn accepts_text_input(&self) -> bool {
        false
    }

    fn accept_char(&mut self, _: char) {
        unimplemented!()
    }

    fn accept_text(&mut self, _: String) {
        unimplemented!()
    }

    fn apply_action(
        &mut self,
        _: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        match action {
            ErrorAction::Up => self.pos_y = self.pos_y.saturating_sub(1),
            ErrorAction::Down => self.pos_y = self.pos_y.saturating_add(1),
            ErrorAction::Left => self.pos_x = self.pos_x.saturating_sub(1),
            ErrorAction::Right => self.pos_x = self.pos_x.saturating_add(1),
        }
        Ok(None)
    }

    fn click(
        &mut self,
        _: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        _: ratatui::prelude::Position,
        _: ratatui::prelude::Size,
        _: jellyhaj_widgets_core::MouseEventKind,
        _: jellyhaj_widgets_core::KeyModifiers,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        Ok(None)
    }

    fn render_fallible_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        _: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> jellyhaj_widgets_core::Result<()> {
        let text = self
            .text
            .to_text()
            .context("handling color eyre error message")?;
        let width = u16::try_from(text.width()).context("error message to wide")?;
        let height = u16::try_from(text.height()).context("error message has to many lines")?;
        let outer = Block::bordered()
            .title("Error encountered")
            .padding(Padding::uniform(1));
        let main = outer.inner(area);
        self.pos_x = min(width.saturating_sub(main.width), self.pos_x);
        self.pos_y = min(height.saturating_sub(main.height), self.pos_y);
        let text = Paragraph::new(text).scroll((self.pos_y, self.pos_x));
        text.render(main, buf);
        if height > main.height {
            Scrollbar::new(ScrollbarOrientation::VerticalRight).render(
                main.inner(Margin::new(0, 2)),
                buf,
                &mut ScrollbarState::new(self.pos_y as usize).position(self.pos_y as usize),
            );
        }
        if width > main.width {
            Scrollbar::new(ScrollbarOrientation::HorizontalBottom).render(
                main.inner(Margin::new(2, 0)),
                buf,
                &mut ScrollbarState::new(self.pos_x as usize).position(self.pos_x as usize),
            );
        }
        outer.render(area, buf);
        Ok(())
    }
}
