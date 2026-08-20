use std::{cmp::min, convert::Infallible, fmt::Debug, ops::ControlFlow};

use color_eyre::eyre::OptionExt;
use jellyhaj_core::state::Navigation;
use jellyhaj_widgets_core::{
    KeyModifiers, MouseEventKind, Position, Rect, RenderFlag, Result, WidgetContext, Wrapper,
};
use ratatui::{
    crossterm::event::MouseButton,
    style::Modifier,
    widgets::{
        Block, BorderType, Clear, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Widget, WidgetRef,
    },
};
use valuable::Valuable;

use crate::{FormAction, FormItem, FormItemBase, offset::calc_offset};

pub trait Selection: Clone + Copy + PartialEq + Eq + Debug + Valuable + 'static {
    fn descr(self) -> &'static str;
    fn index(self) -> usize;
    const MAX_LEN: u16;
    const ALL: &[Self];
    const ALL_LEN: u16;
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
    }
    index = index.strict_sub(1);
    S::ALL[index]
}

impl<S: Selection, AR: From<Infallible> + Debug> FormItemBase<AR> for S {
    type SelectionInner = Option<S>;

    type Ret = Infallible;

    type Action = Infallible;

    fn accepts_movement_action(&self, sel: &Self::SelectionInner) -> bool {
        sel.is_some()
    }

    fn popup_area(
        &self,
        sel: &Self::SelectionInner,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Size,
    ) -> ratatui::prelude::Rect {
        if sel.is_some() {
            let mut full_area: Rect = ((0, 0).into(), full_area).into();
            let offset = area.y - full_area.y + 2;
            full_area.y += offset;
            full_area.height -= offset;
            full_area.width = min(S::MAX_LEN + 2, area.width);
            let needed_height = S::ALL_LEN + 2;
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

    fn height(&self) -> u16 {
        3
    }

    fn height_buf(&self) -> u16 {
        4
    }
}

impl<R: 'static, S: Selection, AR: From<Infallible> + Debug> FormItem<R, AR> for S {
    fn apply_movement(
        &mut self,
        sel: &mut Self::SelectionInner,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: FormAction<Infallible>,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<ControlFlow<Navigation, Infallible>>> {
        if let Some(sel_inner) = sel {
            match action {
                FormAction::Up => {
                    *sel_inner = selection_prev(*sel_inner);
                    render_flag.set();
                }
                FormAction::Down => {
                    *sel_inner = selection_next(*sel_inner);
                    render_flag.set();
                }
                FormAction::Enter => {
                    *self = *sel_inner;
                    *sel = None;
                    render_flag.set();
                }
                FormAction::Quit => {
                    *sel = None;
                    render_flag.set();
                }
                _ => {}
            }
        } else if matches!(action, FormAction::Enter) {
            *sel = Some(*self);
            render_flag.set();
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

    fn render_pass_main(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
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
        self.descr().render(main, buf);
        outer.render(area, buf);
        buf[Position {
            x: area.x + area.width - 2,
            y: area.y + 1,
        }]
        .set_char('⮛');
        Ok(())
    }

    fn render_pass_popup(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        area: ratatui::prelude::Rect,
        mut full_area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        name: &'static str,
        sel: &mut Self::SelectionInner,
    ) -> Result<()> {
        if let Some(sel_inner) = sel {
            buf[Position {
                x: area.x + area.width - 2,
                y: area.y + 1,
            }]
            .set_char('⮙');
            let offset = area.y - full_area.y + 2;
            full_area.y += offset;
            full_area.height -= offset;
            full_area.width = min(S::MAX_LEN + 2, area.width) - 1;
            full_area.x += 1;
            let needed_height = S::ALL_LEN + 2;
            let mut items = S::ALL;
            let scrollbar = if needed_height > full_area.height {
                let window = full_area.height;
                let offset = calc_offset(
                    S::ALL.len().try_into().expect("len is to large"),
                    window,
                    sel_inner.index().try_into().expect("index is to large"),
                );
                items = &items[offset as usize..(offset + window) as usize];
                true
            } else {
                full_area.height = needed_height;
                false
            };
            Clear.render(full_area, buf);
            let selection_block = Block::bordered().border_type(BorderType::Thick);
            let inner = selection_block.inner(full_area);
            for (i, c) in items.iter().copied().enumerate() {
                let mut area = inner;
                area.y += u16::try_from(i).expect("bounded size");
                area.height = 1;
                c.descr().render(area, buf);
                if *sel_inner == c {
                    for i in 0..area.width {
                        buf[(area.x + i, area.y)].set_style(Modifier::REVERSED);
                    }
                }
            }
            selection_block.render(full_area, buf);
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
        if kind == MouseEventKind::Down(MouseButton::Left) {
            let sel_inner = sel.as_mut().expect("inner must be set");
            let mut full_area: Rect = ((0, 0).into(), full_area).into();
            let offset = area.y - full_area.y + 2;
            full_area.y += offset;
            full_area.height -= offset;
            full_area.width = min(S::MAX_LEN + 2, area.width);
            let needed_height = S::ALL_LEN + 2;
            let mut items = S::ALL;
            if needed_height > full_area.height {
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
                render_flag.set();
            }
        }
        Ok(None)
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
        if kind == MouseEventKind::Down(MouseButton::Left) {
            render_flag.set();
            Ok((Some(Some(*self)), None))
        } else {
            Ok((None, None))
        }
    }
}

#[derive(Debug, Valuable)]
pub struct DynamicSelection {
    optons: Vec<String>,
    cur: usize,
}

impl DynamicSelection {
    fn max_len(&self) -> u16 {
        let max: usize = self.optons.iter().map(String::len).max().unwrap_or(0);
        max.try_into().expect("option length to long")
    }
    fn len(&self) -> u16 {
        self.optons.len().try_into().expect("to many options")
    }
}

impl<AR: From<Infallible> + Debug> FormItemBase<AR> for DynamicSelection {
    type SelectionInner = Option<usize>;

    type Ret = Infallible;

    type Action = Infallible;

    fn accepts_movement_action(&self, sel: &Self::SelectionInner) -> bool {
        sel.is_some()
    }

    fn popup_area(
        &self,
        sel: &Self::SelectionInner,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Size,
    ) -> ratatui::prelude::Rect {
        if sel.is_some() {
            let mut full_area: Rect = ((0, 0).into(), full_area).into();
            let offset = area.y - full_area.y + 2;
            full_area.y += offset;
            full_area.height -= offset;
            full_area.width = min(self.max_len() + 2, area.width);
            let needed_height = self.len() + 2;
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

    fn height(&self) -> u16 {
        3
    }

    fn height_buf(&self) -> u16 {
        4
    }
}

impl<R: 'static, AR: From<Infallible> + Debug> FormItem<R, AR> for DynamicSelection {
    fn apply_movement(
        &mut self,
        sel: &mut Self::SelectionInner,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: FormAction<Infallible>,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<ControlFlow<Navigation, Infallible>>> {
        if let Some(sel_inner) = sel {
            match action {
                FormAction::Up => {
                    render_flag.set();
                    let mut index = *sel_inner;
                    if index == 0 {
                        index = self.optons.len();
                    }
                    *sel_inner = index.strict_sub(1);
                }
                FormAction::Down => {
                    render_flag.set();
                    let mut index = *sel_inner + 1;
                    if index >= self.optons.len() {
                        index = 0;
                    }
                    *sel_inner = index;
                }
                FormAction::Enter => {
                    render_flag.set();
                    self.cur = *sel_inner;
                    *sel = None;
                }
                FormAction::Quit => {
                    render_flag.set();
                    *sel = None;
                }
                _ => {}
            }
        } else if matches!(action, FormAction::Enter) {
            render_flag.set();
            *sel = Some(self.cur);
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

    fn render_pass_main(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
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
        self.optons
            .get(self.cur)
            .ok_or_eyre("selection out of bounds")?
            .render_ref(main, buf);
        outer.render(area, buf);
        buf[Position {
            x: area.x + area.width - 2,
            y: area.y + 1,
        }]
        .set_char('⮛');
        Ok(())
    }

    fn render_pass_popup(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        area: ratatui::prelude::Rect,
        mut full_area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        name: &'static str,
        sel: &mut Self::SelectionInner,
    ) -> Result<()> {
        if let Some(sel_inner) = sel {
            buf[Position {
                x: area.x + area.width - 2,
                y: area.y + 1,
            }]
            .set_char('⮙');
            let offset = area.y - full_area.y + 2;
            full_area.y += offset;
            full_area.height -= offset;
            full_area.width = min(self.max_len() + 2, area.width) - 1;
            full_area.x += 1;
            let needed_height = self.len() + 2;
            let mut items = self.optons.as_slice();
            let offset;
            let scrollbar = if needed_height > full_area.height {
                let window = full_area.height;
                offset = calc_offset(
                    self.optons.len().try_into().expect("len is to large"),
                    window,
                    (*sel_inner).try_into().expect("index is to large"),
                );
                items = &items[offset as usize..(offset + window) as usize];
                true
            } else {
                full_area.height = needed_height;
                offset = 0;
                false
            };
            Clear.render(full_area, buf);
            let selection_block = Block::bordered().border_type(BorderType::Thick);
            let inner = selection_block.inner(full_area);
            for (i, c) in items.iter().enumerate() {
                let mut area = inner;
                area.y += u16::try_from(i).expect("bounded size");
                area.height = 1;
                c.render_ref(area, buf);
                if *sel_inner == (i + offset as usize) {
                    for i in 0..area.width {
                        buf[(area.x + i, area.y)].set_style(Modifier::REVERSED);
                    }
                }
            }
            selection_block.render(full_area, buf);
            if scrollbar {
                full_area.height = full_area.height.strict_sub(2);
                full_area.y += 1;
                Scrollbar::new(ScrollbarOrientation::VerticalRight).render(
                    full_area,
                    buf,
                    &mut ScrollbarState::new(self.optons.len()).position(*sel_inner),
                );
            }
        }
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
        if kind == MouseEventKind::Down(MouseButton::Left) {
            let sel_inner = sel.as_mut().expect("inner must be set");
            let mut full_area: Rect = ((0, 0).into(), full_area).into();
            let offset = area.y - full_area.y + 2;
            full_area.y += offset;
            full_area.height -= offset;
            full_area.width = min(self.max_len() + 2, area.width);
            let needed_height = self.len() + 2;
            let offset;
            if needed_height < full_area.height {
                let window = full_area.height;
                offset = calc_offset(
                    self.optons.len().try_into().expect("len is to large"),
                    window,
                    (*sel_inner).try_into().expect("index is to large"),
                );
            } else {
                offset = 0;
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
                self.cur = index as usize + offset as usize;
                *sel = None;
                render_flag.set();
            }
        }
        Ok(None)
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
        if kind == MouseEventKind::Down(MouseButton::Left) {
            render_flag.set();
            Ok((Some(Some(self.cur)), None))
        } else {
            Ok((None, None))
        }
    }
}
