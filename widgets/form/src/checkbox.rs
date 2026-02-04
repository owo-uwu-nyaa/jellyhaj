use ratatui::{style::Modifier, widgets::Widget};

use crate::FormItem;

impl<T> FormItem<T> for bool {
    const HEIGHT: u16 = 1;
    const HEIGHT_BUF: u16 = 0;
    type SelectionInner = ();
    fn render_pass_main(
        &self,
        mut area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        active: bool,
        name: &'static str,
    ) {
        buf[(area.x, area.y)].set_char('[');
        let mark = &mut buf[(area.x + 1, area.y)];
        if *self {
            mark.set_char('X');
        }
        if active {
            mark.set_style(Modifier::REVERSED);
        }
        buf[(area.x + 2, area.y)].set_char(']');
        area.x += 4;
        name.render(area, buf);
    }
    fn accepts_text_input(&self, sel: Self::SelectionInner) -> bool {
        todo!()
    }

    fn apply_char(&mut self, sel: &mut Self::SelectionInner, text: char) {
        todo!()
    }

    fn apply_text(&mut self, sel: &mut Self::SelectionInner, text: String) {
        todo!()
    }

    fn accepts_movement_action(&self, sel: Self::SelectionInner) -> bool {
        false
    }

    fn apply_action(
        &mut self,
        sel: &mut Self::SelectionInner,
        action: crate::FormAction,
    ) -> jellyhaj_widgets_core::Result<Option<T>> {
        todo!()
    }

    fn click_area(&self, sel: Self::SelectionInner, area: ratatui::prelude::Rect, full_area: ratatui::prelude::Rect) -> ratatui::prelude::Rect {
        todo!()
    }

    fn apply_click(
        &mut self,
        sel: &mut Self::SelectionInner,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Rect,
        pos: ratatui::prelude::Position,
    ) -> jellyhaj_widgets_core::Result<Option<T>> {
        todo!()
    }

    fn render_pass_popup(
        &self,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        name: &'static str,
        sel: Self::SelectionInner,
    ) {
        todo!()
    }
}
