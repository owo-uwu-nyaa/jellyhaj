use std::{convert::Infallible, io::stdout, ops::ControlFlow};

use crossterm::clipboard::CopyToClipboard;
use jellyhaj_core::state::Navigation;
use jellyhaj_widgets_core::{Position, Result, WidgetContext, Wrapper};
use ratatui::{
    crossterm::execute,
    prelude::Rect,
    widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget},
};
use valuable::Valuable;

use crate::{FormAction, FormItem, FormItemInfo};
use ansi_to_tui::IntoText;

#[derive(Debug, Valuable)]
pub struct LabelBlock {
    pub text: String,
}

impl LabelBlock {
    pub fn new(text: String) -> Self {
        Self { text }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Valuable)]
pub struct Pos {
    x: u16,
    y: u16,
}

impl From<Pos> for Position {
    fn from(value: Pos) -> Self {
        Position {
            x: value.x,
            y: value.y,
        }
    }
}

impl From<Pos> for (u16, u16) {
    fn from(value: Pos) -> Self {
        (value.x, value.y)
    }
}

impl From<Position> for Pos {
    fn from(value: Position) -> Self {
        Pos {
            x: value.x,
            y: value.y,
        }
    }
}

impl<AR: From<Infallible>> FormItemInfo<AR> for LabelBlock {
    const HEIGHT: u16 = 10;

    const HEIGHT_BUF: u16 = 0;

    type SelectionInner = Option<Pos>;

    type Ret = Infallible;

    type Action = Infallible;
}

impl<R: 'static, AR: From<Infallible>> FormItem<R, AR> for LabelBlock {
    fn accepts_text_input(&self, sel: &Self::SelectionInner) -> bool {
        false
    }

    fn apply_char(&mut self, sel: &mut Self::SelectionInner, text: char) {
        unimplemented!()
    }

    fn apply_text(&mut self, sel: &mut Self::SelectionInner, text: String) {
        unimplemented!()
    }

    fn accepts_movement_action(&self, sel: &Self::SelectionInner) -> bool {
        sel.is_some()
    }

    fn apply_movement(
        &mut self,
        sel: &mut Self::SelectionInner,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: FormAction<Infallible>,
    ) -> Result<Option<ControlFlow<Navigation, Self::Ret>>> {
        if let Some(pos) = sel {
            match action {
                FormAction::Left => pos.y = pos.y.saturating_sub(1),
                FormAction::Right => pos.y = pos.y.saturating_add(1),
                FormAction::Up => pos.x = pos.x.saturating_sub(1),
                FormAction::Down => pos.x = pos.x.saturating_add(1),
                FormAction::Delete => {}
                FormAction::Enter => {
                    if sel.is_some() {
                        let _ = execute!(stdout(), CopyToClipboard::to_clipboard_from(&self.text));
                    } else {
                        *sel = Some(Position::ORIGIN.into())
                    }
                }
                FormAction::Quit => *sel = None,
            }
        }
        Ok(None)
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<ControlFlow<Navigation, Self::Ret>>> {
        unreachable!()
    }

    fn popup_area(
        &self,
        sel: &Self::SelectionInner,
        area: Rect,
        full_area: ratatui::prelude::Size,
    ) -> Rect {
        if sel.is_some() {
            full_area.into()
        } else {
            Rect::ZERO
        }
    }

    fn apply_click_active(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        sel: &mut Self::SelectionInner,
        area: Rect,
        full_area: ratatui::prelude::Size,
        pos: Position,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
    ) -> Result<Option<ControlFlow<Navigation, Self::Ret>>> {
        Ok(None)
    }

    fn apply_click_inactive(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        size: ratatui::prelude::Size,
        pos: Position,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
    ) -> Result<(
        Option<Self::SelectionInner>,
        Option<ControlFlow<Navigation, Self::Ret>>,
    )> {
        if kind.is_down() {
            Ok((Some(Some(Position::ORIGIN.into())), None))
        } else {
            Ok((None, None))
        }
    }

    fn render_pass_main(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        active: bool,
        name: &'static str,
    ) -> Result<()> {
        let mut block = Block::bordered().padding(Padding::uniform(1));
        if active {
            block = block.border_type(BorderType::Double);
        }
        Paragraph::new(self.text.to_text()?)
            .block(block)
            .render(area, buf);
        Ok(())
    }

    fn render_pass_popup(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        area: Rect,
        full_area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        name: &'static str,
        sel: &mut Self::SelectionInner,
    ) -> Result<()> {
        if let Some(pos) = sel {
            Clear.render(full_area, buf);
            Paragraph::new(self.text.to_text()?)
                .scroll((*pos).into())
                .block(Block::bordered().border_type(BorderType::Rounded).padding(Padding::uniform(1)))
                .render(full_area, buf);
        }
        Ok(())
    }
}
