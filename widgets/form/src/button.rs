use std::{fmt::Debug, ops::ControlFlow};

use jellyhaj_core::state::Navigation;
use jellyhaj_widgets_core::{MouseEventKind, Rect};
use ratatui::{
    crossterm::event::MouseButton,
    widgets::{Block, BorderType, Widget},
};

use crate::{FormAction, FormItem};

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

impl<C: ActionCreator, AR: From<C::T>> FormItem<AR> for Button<C> {
    const HEIGHT: u16 = 3;

    const HEIGHT_BUF: u16 = 0;

    type SelectionInner = ();
    type R = C::T;

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
    ) -> jellyhaj_widgets_core::Result<Option<ControlFlow<Navigation, C::T>>> {
        if action == FormAction::Enter {
            Ok(Some(ControlFlow::Continue(self.creator.make_action())))
        } else {
            Ok(None)
        }
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
    ) -> jellyhaj_widgets_core::Result<Option<ControlFlow<Navigation, C::T>>> {
        unimplemented!()
    }

    fn apply_click_inactive(
        &mut self,
        size: ratatui::prelude::Size,
        pos: ratatui::prelude::Position,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
    ) -> jellyhaj_widgets_core::Result<(
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
        mut area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        active: bool,
        name: &'static str,
    ) -> jellyhaj_widgets_core::Result<()> {
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
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        name: &'static str,
        sel: &mut Self::SelectionInner,
    ) -> jellyhaj_widgets_core::Result<()> {
        Ok(())
    }
}
