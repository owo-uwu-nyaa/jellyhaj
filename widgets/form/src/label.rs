use std::{convert::Infallible, ops::ControlFlow};

use jellyhaj_core::state::Navigation;
use jellyhaj_widgets_core::{Rect, Result, WidgetContext, Wrapper};
use ratatui::{buffer::CellWidth, widgets::Widget};
use valuable::Valuable;

use crate::{FormItem, FormItemBase};

#[derive(Debug, Default, Valuable)]
pub struct Label;

impl<AR: From<Infallible>> FormItemBase<AR> for Label {
    type SelectionInner = ();

    type Ret = Infallible;

    type Action = Infallible;

    fn accepts_movement_action(&self, sel: &Self::SelectionInner) -> bool {
        false
    }
    fn popup_area(
        &self,
        sel: &Self::SelectionInner,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Size,
    ) -> ratatui::prelude::Rect {
        Rect::ZERO
    }

    fn height(&self) -> u16 {
        1
    }

    fn height_buf(&self) -> u16 {
        0
    }
}

impl<R: 'static, AR: From<Infallible>> FormItem<R, AR> for Label {
    fn apply_movement(
        &mut self,
        sel: &mut Self::SelectionInner,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: crate::FormAction<Infallible>,
    ) -> Result<Option<ControlFlow<Navigation, Self::Ret>>> {
        Ok(None)
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<ControlFlow<Navigation, Self::Ret>>> {
        unreachable!()
    }

    fn apply_click_active(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        sel: &mut Self::SelectionInner,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Size,
        pos: ratatui::prelude::Position,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
    ) -> Result<Option<ControlFlow<Navigation, Infallible>>> {
        Ok(None)
    }

    fn apply_click_inactive(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        size: ratatui::prelude::Size,
        pos: ratatui::prelude::Position,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
    ) -> Result<(
        Option<Self::SelectionInner>,
        Option<ControlFlow<Navigation, Infallible>>,
    )> {
        Ok((None, None))
    }

    fn render_pass_main(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        mut area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        active: bool,
        name: &'static str,
    ) -> Result<()> {
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
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        name: &'static str,
        sel: &mut Self::SelectionInner,
    ) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Valuable)]
pub struct DynamicLabel {
    pub val: String,
}

impl DynamicLabel {
    #[must_use]
    pub const fn new(val: String) -> Self {
        Self { val }
    }
}
impl<AR: From<Infallible>> FormItemBase<AR> for DynamicLabel {
    type SelectionInner = ();

    type Ret = Infallible;

    type Action = Infallible;

    fn accepts_movement_action(&self, sel: &Self::SelectionInner) -> bool {
        false
    }
    fn popup_area(
        &self,
        sel: &Self::SelectionInner,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Size,
    ) -> ratatui::prelude::Rect {
        Rect::ZERO
    }

    fn height(&self) -> u16 {
        1
    }

    fn height_buf(&self) -> u16 {
        0
    }
}
impl<R: 'static, AR: From<Infallible>> FormItem<R, AR> for DynamicLabel {
    fn apply_movement(
        &mut self,
        sel: &mut Self::SelectionInner,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: crate::FormAction<Infallible>,
    ) -> Result<Option<ControlFlow<Navigation, Self::Ret>>> {
        Ok(None)
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<ControlFlow<Navigation, Self::Ret>>> {
        unreachable!()
    }

    fn apply_click_active(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        sel: &mut Self::SelectionInner,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Size,
        pos: ratatui::prelude::Position,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
    ) -> Result<Option<ControlFlow<Navigation, Infallible>>> {
        Ok(None)
    }

    fn apply_click_inactive(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        size: ratatui::prelude::Size,
        pos: ratatui::prelude::Position,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
    ) -> Result<(
        Option<Self::SelectionInner>,
        Option<ControlFlow<Navigation, Infallible>>,
    )> {
        Ok((None, None))
    }

    fn render_pass_main(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        mut area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        active: bool,
        name: &'static str,
    ) -> Result<()> {
        if active {
            buf[area.as_position()].set_char('*');
            area.x += 2;
            area.width -= 2;
        }
        name.render(area, buf);
        let name_width = name.cell_width();
        if name_width > 0 {
            area.x += name_width + 1;
        }
        if let Some(w) = area.width.checked_sub(name_width + 1) {
            area.width = w;
            self.val.as_str().render(area, buf);
        }
        Ok(())
    }

    fn render_pass_popup(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        name: &'static str,
        sel: &mut Self::SelectionInner,
    ) -> Result<()> {
        Ok(())
    }
}
