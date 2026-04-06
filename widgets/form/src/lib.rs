#![allow(unused_variables)]

pub mod button;
pub mod checkbox;
pub mod form;
pub mod label;
pub mod label_block;
#[doc(hidden)]
pub mod macro_impl;
mod offset;
pub mod secret_field;
pub mod selection;
pub mod text_field;

use std::{convert::Infallible, fmt::Debug, ops::ControlFlow};

use color_eyre::Result;
use jellyhaj_core::state::Navigation;
pub use jellyhaj_form_derive::{Selection, form_widget};
use jellyhaj_widgets_core::{
    KeyModifiers, MouseEventKind, Size, WidgetContext, Wrapper, valuable::Valuable,
};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
};
pub use selection::Selection;

#[derive(Debug)]
pub enum FormAction<A: Debug + Send + 'static> {
    Quit,
    Up,
    Down,
    Left,
    Right,
    Delete,
    Enter,
    Inner(A),
}

pub trait FormItemInfo<AR>: Valuable {
    const HEIGHT: u16;
    const HEIGHT_BUF: u16;
    type SelectionInner: Default + Debug + Valuable;
    type Ret: Into<AR>;
    type Action: Debug + Send + 'static;
}

pub trait FormItem<R: 'static, AR>: FormItemInfo<AR> {
    fn accepts_text_input(&self, sel: &Self::SelectionInner) -> bool;
    fn apply_char(&mut self, sel: &mut Self::SelectionInner, text: char);
    fn apply_text(&mut self, sel: &mut Self::SelectionInner, text: String);

    fn accepts_movement_action(&self, sel: &Self::SelectionInner) -> bool;
    fn apply_movement(
        &mut self,
        sel: &mut Self::SelectionInner,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: FormAction<Infallible>,
    ) -> Result<Option<ControlFlow<Navigation, Self::Ret>>>;

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<ControlFlow<Navigation, Self::Ret>>>;

    fn popup_area(&self, sel: &Self::SelectionInner, area: Rect, full_area: Size) -> Rect;

    #[allow(clippy::too_many_arguments)]
    fn apply_click_active(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        sel: &mut Self::SelectionInner,
        area: Rect,
        full_area: Size,
        pos: Position,
        kind: MouseEventKind,
        modifier: KeyModifiers,
    ) -> Result<Option<ControlFlow<Navigation, Self::Ret>>>;

    #[allow(clippy::type_complexity)]
    fn apply_click_inactive(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        size: Size,
        pos: Position,
        kind: MouseEventKind,
        modifier: KeyModifiers,
    ) -> Result<(
        Option<Self::SelectionInner>,
        Option<ControlFlow<Navigation, Self::Ret>>,
    )>;

    fn render_pass_main(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        area: Rect,
        buf: &mut Buffer,
        active: bool,
        name: &'static str,
    ) -> Result<()>;

    fn render_pass_popup(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        area: Rect,
        full_area: Rect,
        buf: &mut Buffer,
        name: &'static str,
        sel: &mut Self::SelectionInner,
    ) -> Result<()>;
}
