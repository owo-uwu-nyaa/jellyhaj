use std::{convert::Infallible, fmt::Debug, ops::ControlFlow};

use jellyhaj_core::state::Navigation;
use jellyhaj_widgets_core::{MouseEventKind, Rect, Result, WidgetContext, Wrapper};
use ratatui::{
    crossterm::event::MouseButton,
    widgets::{Block, BorderType, Widget},
};

use crate::{FormAction, FormItem, FormItemInfo};

pub trait ActionCreator: Debug {
    type T;
    fn make_action(&self) -> Self::T;
}

#[derive(Default)]
pub struct Button<Creator: ActionCreator> {
    creator: Creator,
    width: u16,
}

impl<Creator: ActionCreator> std::fmt::Debug for Button<Creator> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Button")
            .field("creator", &self.creator)
            .finish()
    }
}

impl<Creator: ActionCreator> Button<Creator> {
    pub fn new(creator: Creator) -> Self {
        Self { creator, width: 0 }
    }
}

struct Centered {
    offset: u16,
    size: u16,
}

fn center(full: u16, requested: u16) -> Centered {
    if full > requested {
        let buf = full - requested;
        Centered {
            offset: buf / 2,
            size: requested,
        }
    } else {
        Centered {
            offset: 0,
            size: full,
        }
    }
}

impl<C: ActionCreator, AR: From<C::T>> FormItemInfo<AR> for Button<C> {
    const HEIGHT: u16 = 3;

    const HEIGHT_BUF: u16 = 0;

    type SelectionInner = ();

    type Ret = C::T;

    type Action = Infallible;
}

impl<R: 'static, C: ActionCreator, AR: From<C::T>> FormItem<R, AR> for Button<C> {
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

    fn apply_movement(
        &mut self,
        sel: &mut Self::SelectionInner,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: crate::FormAction<Infallible>,
    ) -> Result<Option<ControlFlow<Navigation, C::T>>> {
        if let FormAction::Enter = action {
            Ok(Some(ControlFlow::Continue(self.creator.make_action())))
        } else {
            Ok(None)
        }
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
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Size,
    ) -> ratatui::prelude::Rect {
        Rect::ZERO
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
    ) -> Result<Option<ControlFlow<Navigation, C::T>>> {
        unimplemented!()
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
        Option<ControlFlow<Navigation, C::T>>,
    )> {
        let centered = center(size.width, self.width);
        if let MouseEventKind::Down(MouseButton::Left) = kind
            && pos.x >= centered.offset
            && pos.x < centered.offset + centered.size
        {
            Ok((
                Some(()),
                Some(ControlFlow::Continue(self.creator.make_action())),
            ))
        } else {
            Ok((None, None))
        }
    }

    fn render_pass_main(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        mut area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        active: bool,
        name: &'static str,
    ) -> Result<()> {
        self.width = name.chars().map(|_| 1u16).sum::<u16>() + 2;
        let centered = center(area.width, self.width);
        area.x += centered.offset;
        area.width = centered.size;
        let mut block = Block::bordered();
        if active {
            block = block.border_type(BorderType::Double);
        }
        let main = block.inner(area);
        name.render(main, buf);
        block.render(area, buf);
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
