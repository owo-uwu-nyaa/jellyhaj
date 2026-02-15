use std::{convert::Infallible, ops::ControlFlow};

use jellyhaj_core::state::Navigation;
use jellyhaj_widgets_core::Rect;
use ratatui::widgets::Widget;

use crate::FormItem;

pub struct Label;

impl<AR: From<Infallible>> FormItem<AR> for Label {
    const HEIGHT: u16 = 1;

    const HEIGHT_BUF: u16 = 0;

    type SelectionInner = ();
    type R = Infallible;

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
        false
    }

    fn apply_action(
        &mut self,
        sel: &mut Self::SelectionInner,
        action: crate::FormAction,
    ) -> jellyhaj_widgets_core::Result<Option<ControlFlow<Navigation, Infallible>>> {
        Ok(None)
    }

    fn popup_area(
        &self,
        sel: &Self::SelectionInner,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Size,
    ) -> ratatui::prelude::Rect {
        Rect::ZERO
    }

    fn apply_click_active(
        &mut self,
        sel: &mut Self::SelectionInner,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Size,
        pos: ratatui::prelude::Position,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
    ) -> jellyhaj_widgets_core::Result<Option<ControlFlow<Navigation, Infallible>>> {
        Ok(None)
    }

    fn apply_click_inactive(
        &mut self,
        size: ratatui::prelude::Size,
        pos: ratatui::prelude::Position,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
    ) -> jellyhaj_widgets_core::Result<(
        Option<Self::SelectionInner>,
        Option<ControlFlow<Navigation, Infallible>>,
    )> {
        Ok((None, None))
    }

    fn render_pass_main(
        &mut self,
        mut area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        active: bool,
        name: &'static str,
    ) -> jellyhaj_widgets_core::Result<()> {
        if active {
            buf[area.as_position()].set_char('*');
            area.x += 2;
            area.width -= 2;
        }
        name.render(area, buf);
        Ok(())
    }

    fn render_pass_popup(
        &mut self,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        name: &'static str,
        sel: &mut Self::SelectionInner,
    ) -> jellyhaj_widgets_core::Result<()> {
        Ok(())
    }
}
