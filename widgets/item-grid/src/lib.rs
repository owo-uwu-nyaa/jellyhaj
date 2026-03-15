use std::{
    cmp::min,
    fmt::Debug,
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use jellyhaj_widgets_core::{
    ItemState, ItemWidget, ItemWidgetExt, JellyhajWidget, JellyhajWidgetState, WidgetContext,
    Wrapper,
};
use ratatui::{
    layout::{Position, Rect, Size},
    widgets::{Block, Padding, Scrollbar, ScrollbarState, StatefulWidget, Widget},
};

pub struct ItemGrid<R: 'static, T: ItemWidget<R>> {
    items: Vec<T>,
    current: usize,
    width: usize,
    title: String,
    item_size: Size,
    skip_rows: usize,
    _r: PhantomData<fn(R)>,
}

impl<R: 'static, T: ItemWidget<R>> ItemGrid<R, T> {
    pub fn new(items: Vec<T>, current: usize, title: String, cx: &R) -> Self {
        Self {
            items,
            current,
            width: 1,
            title,
            item_size: <T as ItemWidget<R>>::dimensions_static(cx),
            skip_rows: 0,
            _r: PhantomData,
        }
    }
    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.items.get_mut(index)
    }
}

impl<R: 'static, T: ItemWidget<R>> Index<usize> for ItemGrid<R, T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.items[index]
    }
}

impl<R: 'static, T: ItemWidget<R>> IndexMut<usize> for ItemGrid<R, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.items[index]
    }
}

pub struct ItemGridState<R: 'static, W: ItemState<R>> {
    pub items: Vec<W>,
    pub title: String,
    pub current: usize,
    _r: PhantomData<fn(R)>,
}

impl<R: 'static, W: ItemState<R>> ItemGridState<R, W> {
    pub fn new(items: Vec<W>, title: String, current: usize) -> Self {
        Self {
            items,
            title,
            current,
            _r: PhantomData,
        }
    }
}

impl<R: 'static, W: ItemState<R>> Debug for ItemGridState<R, W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ItemGridData")
            .field("items", &self.items)
            .field("title", &self.title)
            .field("current", &self.current)
            .finish()
    }
}

impl<R: 'static, T: ItemState<R>> JellyhajWidgetState<R> for ItemGridState<R, T> {
    type Action = ItemGridAction<T::IAction>;

    type ActionResult = T::IActionResult;

    type Widget = ItemGrid<R, T::IWidget>;

    const NAME: &str = "grid";

    fn visit_children(visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        visitor.visit_item::<R, T>();
    }

    fn into_widget(self, cx: &R) -> Self::Widget {
        ItemGrid {
            items: self
                .items
                .into_iter()
                .map(|s| s.item_into_widget(cx))
                .collect(),
            current: self.current,
            width: 1,
            title: self.title,
            item_size: T::IWidget::dimensions_static(cx),
            skip_rows: 0,
            _r: PhantomData,
        }
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        match action {
            ItemGridAction::SpecificInner(index, action) => self
                .items
                .get_mut(index)
                .and_then(|v| {
                    ItemState::item_apply_action(v, cx.wrap_with(GridWrapper { index }), action)
                        .transpose()
                })
                .transpose(),
            ItemGridAction::CurrentInner(action) => self
                .items
                .get_mut(self.current)
                .and_then(|v| {
                    ItemState::item_apply_action(
                        v,
                        cx.wrap_with(GridWrapper {
                            index: self.current,
                        }),
                        action,
                    )
                    .transpose()
                })
                .transpose(),
            _ => Ok(None),
        }
    }
}

#[derive(Debug)]
pub enum ItemGridAction<T> {
    SpecificInner(usize, T),
    CurrentInner(T),
    Up,
    Left,
    Right,
    Down,
}

#[derive(Clone, Copy)]
struct GridWrapper {
    index: usize,
}

impl<T: Send + 'static> Wrapper<T> for GridWrapper {
    type F = ItemGridAction<T>;

    fn wrap(&self, val: T) -> Self::F {
        ItemGridAction::SpecificInner(self.index, val)
    }
}

impl<R: 'static, T: ItemWidget<R>> JellyhajWidget<R> for ItemGrid<R, T> {
    type State = ItemGridState<R, T::IState>;
    type Action = ItemGridAction<<T as ItemWidget<R>>::IAction>;
    type ActionResult = <T as ItemWidget<R>>::IActionResult;

    fn into_state(self) -> Self::State {
        ItemGridState {
            items: self
                .items
                .into_iter()
                .map(ItemWidget::item_into_state)
                .collect(),
            title: self.title,
            current: self.current,
            _r: PhantomData,
        }
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        match action {
            ItemGridAction::SpecificInner(index, action) => self
                .items
                .get_mut(index)
                .and_then(|v| {
                    v.item_apply_action(cx.wrap_with(GridWrapper { index }), action)
                        .transpose()
                })
                .transpose(),
            ItemGridAction::CurrentInner(action) => self
                .items
                .get_mut(self.current)
                .and_then(|v| {
                    v.item_apply_action(
                        cx.wrap_with(GridWrapper {
                            index: self.current,
                        }),
                        action,
                    )
                    .transpose()
                })
                .transpose(),
            ItemGridAction::Up => {
                self.current = self.current.saturating_sub(self.width);
                Ok(None)
            }
            ItemGridAction::Left => {
                self.current = self.current.saturating_sub(1);
                Ok(None)
            }
            ItemGridAction::Right => {
                self.current = min(self.items.len().saturating_sub(1), self.current + 1);
                Ok(None)
            }
            ItemGridAction::Down => {
                self.current = min(
                    self.items.len().saturating_sub(1),
                    self.current + self.width,
                );
                Ok(None)
            }
        }
    }

    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        mut position: ratatui::prelude::Position,
        size: Size,
        kind: ratatui::crossterm::event::MouseEventKind,
        modifier: ratatui::crossterm::event::KeyModifiers,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        if position.x < 2
            || position.y < 2
            || position.x >= size.width - 2
            || position.y >= size.height - 2
        {
            Ok(None)
        } else {
            position.x -= 2;
            position.y -= 2;
            let row = position.y / (self.item_size.height + 1);
            let row = (row as usize) + self.skip_rows;
            let y_position = position.y % (self.item_size.height + 1);
            let col = position.x / (self.item_size.width + 1);
            let index = row * self.width + (col as usize);
            let x_position = position.x % (self.item_size.width + 1);
            if x_position < self.item_size.width
                && y_position < self.item_size.height
                && let Some(item) = self.items.get_mut(index)
            {
                item.item_click(
                    cx.wrap_with(GridWrapper { index }),
                    Position {
                        x: x_position,
                        y: y_position,
                    },
                    self.item_size,
                    kind,
                    modifier,
                )
            } else {
                Ok(None)
            }
        }
    }

    fn render_fallible_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> jellyhaj_widgets_core::Result<()> {
        let outer = Block::bordered()
            .title_top(self.title.as_str())
            .padding(Padding::uniform(1));
        let main = outer.inner(area);
        self.width = ((main.width + 1) / (self.item_size.width + 1)).into();
        let height: usize = ((main.height + 1) / (self.item_size.height + 1)).into();
        let rows = self.items.len().div_ceil(self.width);
        let row_index = self.current / self.width;
        self.skip_rows = if height < rows
            && let position = height / 2
            && row_index > position
        {
            min(row_index - position, rows - height)
        } else {
            0
        };
        let position = (0..height)
            .map(|row| main.y + (self.item_size.height + 1) * (row as u16))
            .flat_map(|y| {
                (0..self.width)
                    .map(|col| main.x + (self.item_size.width + 1) * (col as u16))
                    .map(move |x| Position { x, y })
            });
        for ((index, item), position) in self
            .items
            .iter_mut()
            .enumerate()
            .skip(self.skip_rows * self.width)
            .zip(position)
        {
            item.set_active(self.current == index);
            item.render_item(
                Rect::from((position, self.item_size)),
                buf,
                cx.wrap_with(GridWrapper { index }),
            )?
        }
        outer.render(area, buf);
        if height < rows {
            Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight).render(
                area,
                buf,
                &mut ScrollbarState::new(rows)
                    .position(row_index)
                    .viewport_content_length(self.item_size.height as usize + 1),
            );
        }
        Ok(())
    }
    fn min_width(&self) -> Option<u16> {
        Some(self.item_size.width + 4)
    }

    fn min_height(&self) -> Option<u16> {
        Some(self.item_size.width + 4)
    }

    fn accepts_text_input(&self) -> bool {
        self.get(self.current)
            .map(|i| i.item_accepts_text_input())
            .unwrap_or(false)
    }

    fn accept_char(&mut self, text: char) {
        if let Some(i) = self.get_mut(self.current)
            && i.item_accepts_text_input()
        {
            i.item_accept_char(text);
        }
    }

    fn accept_text(&mut self, text: String) {
        if let Some(i) = self.get_mut(self.current)
            && i.item_accepts_text_input()
        {
            i.item_accept_text(text);
        }
    }
}
