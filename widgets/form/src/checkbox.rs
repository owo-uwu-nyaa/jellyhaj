use std::{convert::Infallible, fmt::Debug, ops::ControlFlow};

use jellyhaj_core::state::Navigation;
use jellyhaj_widgets_core::{
    KeyModifiers, MouseEventKind, Rect, RenderFlag, Result, WidgetContext, Wrapper,
};
use ratatui::{crossterm::event::MouseButton, style::Modifier, widgets::Widget};

use crate::{FormAction, FormItem, FormItemBase};

impl<AR: From<Infallible> + Debug> FormItemBase<AR> for bool {
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

impl<R: 'static, AR: From<Infallible> + Debug> FormItem<R, AR> for bool {
    fn render_pass_main(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        mut area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        active: bool,
        name: &'static str,
    ) -> Result<()> {
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
        Ok(())
    }

    fn apply_movement(
        &mut self,
        sel: &mut Self::SelectionInner,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: FormAction<Infallible>,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<ControlFlow<Navigation, Self::Ret>>> {
        if matches!(action, FormAction::Enter) {
            render_flag.set();
            *self ^= true;
        }
        Ok(None)
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<ControlFlow<Navigation, Self::Ret>>> {
        unreachable!()
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

    fn apply_click_active(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        sel: &mut Self::SelectionInner,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Size,
        pos: ratatui::prelude::Position,
        kind: MouseEventKind,
        modifier: KeyModifiers,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<ControlFlow<Navigation, Infallible>>> {
        unreachable!()
    }

    fn apply_click_inactive(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        size: ratatui::prelude::Size,
        pos: ratatui::prelude::Position,
        kind: MouseEventKind,
        modifier: KeyModifiers,
        render_flag: &mut RenderFlag,
    ) -> Result<(
        Option<Self::SelectionInner>,
        Option<ControlFlow<Navigation, Infallible>>,
    )> {
        if kind == MouseEventKind::Down(MouseButton::Left) && pos.x < 3 {
            render_flag.set();
            *self ^= true;
        }
        Ok((None, None))
    }
}
