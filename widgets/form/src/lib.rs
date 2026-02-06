#![allow(unused_variables)]

pub mod button;
pub mod checkbox;
pub mod label;
#[doc(hidden)]
pub mod macro_impl;
pub mod secret_field;
pub mod selection;
pub mod text_field;
use color_eyre::Result;
pub use jellyhaj_form_derive::{Selection, form};
use jellyhaj_widgets_core::{KeyModifiers, MouseEventKind, Size};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
};
pub use selection::Selection;

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

    fn popup_area(&self, sel: Self::SelectionInner, area: Rect, full_area: Size) -> Rect;

    fn apply_click_active(
        &mut self,
        sel: &mut Self::SelectionInner,
        area: Rect,
        full_area: Size,
        pos: Position,
        kind: MouseEventKind,
        modifier: KeyModifiers,
    ) -> Result<Option<AR>>;

    fn apply_click_inactive(
        &mut self,
        size: Size,
        pos: Position,
        kind: MouseEventKind,
        modifier: KeyModifiers,
    ) -> Result<(Option<Self::SelectionInner>, Option<AR>)>;

    fn render_pass_main(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        active: bool,
        name: &'static str,
    ) -> Result<()>;

    fn render_pass_popup(
        &mut self,
        area: Rect,
        full_area: Rect,
        buf: &mut Buffer,
        name: &'static str,
        sel: Self::SelectionInner,
    ) -> Result<()>;
}
