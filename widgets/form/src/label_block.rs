use std::{convert::Infallible, io::stdout, ops::ControlFlow};

use crossterm::clipboard::CopyToClipboard;
use jellyhaj_core::state::Navigation;
use jellyhaj_widgets_core::{Position, Result, WidgetContext, Wrapper};
use ratatui::{
    crossterm::execute,
    prelude::Rect,
    widgets::{Block, Clear, Padding, Paragraph, Widget},
};

use crate::{FormAction, FormItem, FormItemInfo};

#[derive(Debug)]
pub struct LabelBlock {
    pub text: String,
}

impl LabelBlock {
    pub fn new(text: String) -> Self {
        Self { text }
    }
}

impl<AR: From<Infallible>> FormItemInfo<AR> for LabelBlock {
    const HEIGHT: u16 = 5;

    const HEIGHT_BUF: u16 = 0;

    type SelectionInner = Option<Position>;

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
                FormAction::Up => pos.y = pos.y.saturating_sub(1),
                FormAction::Down => pos.y = pos.y.saturating_add(1),
                FormAction::Left => pos.x = pos.x.saturating_sub(1),
                FormAction::Right => pos.x = pos.x.saturating_add(1),
                FormAction::Delete => {}
                FormAction::Enter => {
                    let _ = execute!(stdout(), CopyToClipboard::to_clipboard_from(&self.text));
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
        Ok((Some(Some(Position::ORIGIN)), None))
    }

    fn render_pass_main(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        active: bool,
        name: &'static str,
    ) -> Result<()> {
        Paragraph::new(self.text.as_str())
            .block(Block::bordered().padding(Padding::uniform(1)))
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
            Clear.render(area, buf);
            Paragraph::new(self.text.as_str())
                .scroll((*pos).into())
                .block(Block::bordered().padding(Padding::uniform(1)))
                .render(area, buf);
        }
        Ok(())
    }
}
