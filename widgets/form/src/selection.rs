use std::cmp::min;

use jellyhaj_widgets_core::{KeyModifiers, MouseEventKind, Position, Rect, Result};
use ratatui::{
    crossterm::event::MouseButton,
    style::Modifier,
    widgets::{
        Block, BorderType, Clear, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Widget,
    },
};

use crate::{FormAction, FormItem, macro_impl::offset::calc_offset};

pub trait Selection: Clone + Copy + PartialEq + Eq + 'static {
    fn descr(self) -> &'static str;
    fn index(self) -> usize;
    const MAX_LEN: usize;
    const ALL: &[Self];
}

fn selection_next<S: Selection>(cur: S) -> S {
    let mut index = cur.index() + 1;
    if index >= S::ALL.len() {
        index = 0;
    }
    S::ALL[index]
}

fn selection_prev<S: Selection>(cur: S) -> S {
    let mut index = cur.index();
    if index == 0 {
        index = S::ALL.len();
    };
    index = index.strict_sub(1);
    S::ALL[index]
}

impl<AR, S: Selection> FormItem<AR> for S {
    const HEIGHT: u16 = 3;

    const HEIGHT_BUF: u16 = 4;

    type SelectionInner = Option<S>;

    fn accepts_text_input(&self, sel: Self::SelectionInner) -> bool {
        false
    }

    fn apply_char(&mut self, sel: &mut Self::SelectionInner, text: char) {
        unimplemented!()
    }

    fn apply_text(&mut self, sel: &mut Self::SelectionInner, text: String) {
        unimplemented!()
    }

    fn accepts_movement_action(&self, sel: Self::SelectionInner) -> bool {
        sel.is_some()
    }

    fn apply_action(
        &mut self,
        sel: &mut Self::SelectionInner,
        action: FormAction,
    ) -> Result<Option<AR>> {
        if let Some(sel_inner) = sel {
            match action {
                FormAction::Up => {
                    *sel_inner = selection_prev(*sel_inner);
                }
                FormAction::Down => {
                    *sel_inner = selection_next(*sel_inner);
                }
                FormAction::Enter => {
                    *self = *sel_inner;
                    *sel = None;
                }
                FormAction::Quit => {
                    *sel = None;
                }
                _ => {}
            }
        } else {
            if FormAction::Enter == action {
                *sel = Some(*self);
            }
        }
        Ok(None)
    }

    fn render_pass_main(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        active: bool,
        name: &'static str,
    ) -> Result<()> {
        let mut outer = Block::bordered().title(name);
        if active {
            outer = outer.border_type(BorderType::Double);
        }
        let main = outer.inner(area);
        name.render(main, buf);
        outer.render(area, buf);
        buf[Position {
            x: area.x + area.width - 1,
            y: area.y + 1,
        }]
        .set_char('⮛');
        Ok(())
    }

    fn render_pass_popup(
        &mut self,
        area: ratatui::prelude::Rect,
        mut full_area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        name: &'static str,
        sel: Self::SelectionInner,
    ) -> Result<()> {
        if let Some(sel_inner) = sel {
            buf[Position {
                x: area.x + area.width - 1,
                y: area.y + 1,
            }]
            .set_char('⮙');
            let offset = area.y - full_area.y + 2;
            full_area.y += offset;
            full_area.height -= offset;
            full_area.width = min(S::MAX_LEN as u16 + 2, area.width);
            let needed_height = S::ALL.len() as u16 + 2;
            let mut items = S::ALL;
            let mut scrollbar = false;
            if needed_height < full_area.height {
                let window = full_area.height;
                let offset = calc_offset(
                    S::ALL.len().try_into().expect("len is to large"),
                    window,
                    sel_inner.index().try_into().expect("index is to large"),
                );
                items = &items[offset as usize..(offset + window) as usize];
                scrollbar = true;
            } else {
                full_area.height = needed_height;
            }
            Clear.render(full_area, buf);
            let selection_block = Block::bordered().border_type(BorderType::Thick);
            let inner = selection_block.inner(full_area);
            for (i, c) in items.iter().copied().enumerate() {
                let mut area = inner;
                area.y += i as u16;
                area.height = 1;
                c.descr().render(area, buf);
                if sel_inner == c {
                    for i in 0..area.width {
                        buf[(area.x + i, area.y)].set_style(Modifier::REVERSED);
                    }
                }
            }
            selection_block.render(area, buf);
            if scrollbar {
                full_area.height = full_area.height.strict_sub(2);
                full_area.y += 1;
                Scrollbar::new(ScrollbarOrientation::VerticalRight).render(
                    full_area,
                    buf,
                    &mut ScrollbarState::new(S::ALL.len()).position(sel_inner.index()),
                );
            }
        }
        Ok(())
    }

    fn popup_area(
        &self,
        sel: Self::SelectionInner,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Size,
    ) -> ratatui::prelude::Rect {
        if sel.is_some() {
            let mut full_area: Rect = ((0, 0).into(), full_area).into();
            let offset = area.y - full_area.y + 2;
            full_area.y += offset;
            full_area.height -= offset;
            full_area.width = min(S::MAX_LEN as u16 + 2, area.width);
            let needed_height = S::ALL.len() as u16 + 2;
            if needed_height >= full_area.height {
                full_area.height = needed_height;
            }
            full_area
        } else {
            Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            }
        }
    }

    fn apply_click_active(
        &mut self,
        sel: &mut Self::SelectionInner,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Size,
        pos: ratatui::prelude::Position,
        kind: MouseEventKind,
        modifier: KeyModifiers,
    ) -> Result<Option<AR>> {
        if let MouseEventKind::Down(MouseButton::Left) = kind {
            let sel_inner = sel.as_mut().expect("inner must be set");
            let mut full_area: Rect = ((0, 0).into(), full_area).into();
            let offset = area.y - full_area.y + 2;
            full_area.y += offset;
            full_area.height -= offset;
            full_area.width = min(S::MAX_LEN as u16 + 2, area.width);
            let needed_height = S::ALL.len() as u16 + 2;
            let mut items = S::ALL;
            if needed_height < full_area.height {
                let window = full_area.height;
                let offset = calc_offset(
                    S::ALL.len().try_into().expect("len is to large"),
                    window,
                    sel_inner.index().try_into().expect("index is to large"),
                );
                items = &items[offset as usize..(offset + window) as usize];
            } else {
                full_area.height = needed_height;
            }
            full_area.x += 1;
            full_area.y += 1;
            full_area.width -= 2;
            full_area.height -= 2;
            if pos.x >= full_area.x
                && pos.x < full_area.x + full_area.width
                && pos.y >= full_area.y
                && pos.y < full_area.y + full_area.height
            {
                let index = pos.y - full_area.y;
                *self = items[index as usize];
                *sel = None;
            }
        }
        Ok(None)
    }

    fn apply_click_inactive(
        &mut self,
        size: ratatui::prelude::Size,
        pos: ratatui::prelude::Position,
        kind: MouseEventKind,
        modifier: KeyModifiers,
    ) -> Result<(Option<Self::SelectionInner>, Option<AR>)> {
        if let MouseEventKind::Down(MouseButton::Left) = kind {
            Ok((Some(Some(*self)), None))
        } else {
            Ok((None, None))
        }
    }
}
