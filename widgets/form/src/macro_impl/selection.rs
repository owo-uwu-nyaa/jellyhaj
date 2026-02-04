use std::cmp::min;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Modifier,
    widgets::{
        Block, BorderType, Clear, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Widget,
    },
};

use crate::{Selection, macro_impl::offset::calc_offset};

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

pub fn render_selection<S: Selection>(cur: S, mut area: Rect, buf: &mut Buffer) {
    area.width = min(S::MAX_LEN as u16 + 2, area.width);
    let needed_height = S::ALL.len() as u16 + 2;
    let mut items = S::ALL;
    let mut scrollbar = false;
    if needed_height < area.height {
        let window = area.height.strict_sub(2);
        let offset = calc_offset(
            S::ALL.len().try_into().expect("len is to large"),
            window,
            cur.index().try_into().expect("index is to large"),
        );
        items = &items[offset as usize..(offset + window) as usize];
        scrollbar = true;
    } else {
        area.height = needed_height;
    }
    Clear.render(area, buf);
    let selection_block = Block::bordered().border_type(BorderType::Thick);
    let inner = selection_block.inner(area);
    for (i, c) in items.iter().copied().enumerate() {
        let mut area = inner;
        area.y += i as u16;
        area.height = 1;
        c.descr().render(area, buf);
        if cur == c {
            for i in 0..area.width {
                buf[(area.x + i, area.y)].set_style(Modifier::REVERSED);
            }
        }
    }
    selection_block.render(area, buf);
    if scrollbar {
        area.height = area.height.strict_sub(2);
        area.y += 1;
        Scrollbar::new(ScrollbarOrientation::VerticalRight).render(
            area,
            buf,
            &mut ScrollbarState::new(S::ALL.len()).position(cur.index()),
        );
    }
}
