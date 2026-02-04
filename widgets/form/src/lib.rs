pub mod checkbox;
#[doc(hidden)]
pub mod macro_impl;
pub mod selection;
use color_eyre::Result;
pub use jellyhaj_form_derive::{form, Selection};
pub use selection::Selection;
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormAction {
    Up,
    Down,
    Left,
    Right,
    Delete,
    Enter,
    Quit,
}

pub struct QuitForm;

pub trait FormItem<AR> {
    const HEIGHT: u16;
    const HEIGHT_BUF: u16;
    type SelectionInner: Clone + Copy + Default;

    fn accepts_text_input(&self, sel: Self::SelectionInner) -> bool;
    fn apply_char(&mut self, sel: &mut Self::SelectionInner, text: char);
    fn apply_text(&mut self, sel: &mut Self::SelectionInner, text: String);

    fn accepts_movement_action(&self, sel: Self::SelectionInner) -> bool;
    fn apply_action(
        &mut self,
        sel: &mut Self::SelectionInner,
        action: FormAction,
    ) -> Result<Option<AR>>;

    fn click_area(&self, sel: Self::SelectionInner, area: Rect, full_area: Rect) -> Rect;

    fn apply_click(
        &mut self,
        sel: &mut Self::SelectionInner,
        area: Rect,
        full_area: Rect,
        pos: Position,
    ) -> Result<Option<AR>>;

    fn render_pass_main(
        &self,
        area: Rect,
        buf: &mut Buffer,
        active: bool,
        name: &'static str,
    );
    fn render_pass_popup(
        &self,
        area: Rect,
        full_area: Rect,
        buf: &mut Buffer,
        name: &'static str,
        sel: Self::SelectionInner,
    );
}
