use crate::FormItem;

pub trait Selection: Clone + Copy + PartialEq + Eq + 'static {
    fn descr(self) -> &'static str;
    fn index(self) -> usize;
    const MAX_LEN: usize;
    const ALL: &[Self];
}

impl<AR,S: Selection> FormItem<AR> for S {
    const HEIGHT: u16 = 3;

    const HEIGHT_BUF: u16 = 4;

    type SelectionInner = Option<S>;

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
        sel.is_some()
    }

    fn apply_action(
        &mut self,
        sel: &mut Self::SelectionInner,
        action: crate::FormAction,
    ) -> jellyhaj_widgets_core::Result<Option<AR>> {
        todo!()
    }

    fn click_area(
        &self,
        sel: Self::SelectionInner,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Rect,
    ) -> ratatui::prelude::Rect {
        todo!()
    }

    fn apply_click(
        &mut self,
        sel: &mut Self::SelectionInner,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Rect,
        pos: ratatui::prelude::Position,
    ) -> jellyhaj_widgets_core::Result<Option<AR>> {
        todo!()
    }

    fn render_pass_main(
        &self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        active: bool,
        name: &'static str,
    ) {
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
