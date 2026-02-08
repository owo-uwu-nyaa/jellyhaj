use std::{cmp::min, convert::Infallible};

use ansi_to_tui::IntoText;
use color_eyre::eyre::Context;
use jellyhaj_widgets_core::JellyhajWidget;
use ratatui::{
    layout::Margin,
    widgets::{
        Block, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Widget,
    },
};

pub struct ErrorWidget {
    text: String,
    pos_x: usize,
    pos_y: usize,
}

impl ErrorWidget {
    pub fn new(text: String) -> Self {
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

impl JellyhajWidget for ErrorWidget {
    type State = ();

    type Action = ErrorAction;

    type ActionResult = Infallible;

    fn min_width(&self) -> Option<u16> {
        Some(5)
    }

    fn min_height(&self) -> Option<u16> {
        Some(5)
    }

    fn into_state(self) -> Self::State {}

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
        _: jellyhaj_widgets_core::async_task::TaskSubmitter<
            Self::Action,
            impl jellyhaj_widgets_core::Wrapper<Self::Action>,
        >,
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
        _: jellyhaj_widgets_core::async_task::TaskSubmitter<
            Self::Action,
            impl jellyhaj_widgets_core::Wrapper<Self::Action>,
        >,
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
        _: jellyhaj_widgets_core::async_task::TaskSubmitter<
            Self::Action,
            impl jellyhaj_widgets_core::Wrapper<Self::Action>,
        >,
    ) -> jellyhaj_widgets_core::Result<()> {
        let text = self
            .text
            .to_text()
            .context("handling color eyre error message")?;
        let width = text.width();
        let height = text.height();
        let outer = Block::bordered()
            .title("Error encountered")
            .padding(Padding::uniform(1));
        let main = outer.inner(area);
        self.pos_x = min(width.saturating_sub(main.width as usize), self.pos_x);
        self.pos_y = min(height.saturating_sub(main.height as usize), self.pos_y);
        let text = Paragraph::new(text).scroll((self.pos_y as u16, self.pos_x as u16));
        text.render(area, buf);
        if height > main.height as usize {
            Scrollbar::new(ScrollbarOrientation::VerticalRight).render(
                area.inner(Margin::new(0, 2)),
                buf,
                &mut ScrollbarState::new(self.pos_y).position(self.pos_y),
            );
        }
        if width > main.width as usize {
            Scrollbar::new(ScrollbarOrientation::HorizontalBottom).render(
                area.inner(Margin::new(2, 0)),
                buf,
                &mut ScrollbarState::new(self.pos_x).position(self.pos_x),
            );
        }
        Ok(())
    }
}
