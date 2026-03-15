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
    widgets::{
        Block, Padding, Scrollbar, ScrollbarOrientation::HorizontalBottom, ScrollbarState,
        StatefulWidget, Widget,
    },
};
use tracing::instrument;

#[derive(Debug)]
pub struct ItemList<R: 'static, T: ItemWidget<R>> {
    items: Vec<T>,
    current: usize,
    title: String,
    pub active: bool,
    offset: usize,
    item_size: Size,
    _r: PhantomData<fn(R) -> ()>,
}

impl<R: 'static, T: ItemWidget<R>> ItemList<R, T> {
    pub fn new(items: impl IntoIterator<Item = T>, current: usize, title: String, cx: &R) -> Self {
        Self {
            items: items.into_iter().collect(),
            current,
            title,
            active: false,
            offset: 0,
            item_size: <T as ItemWidget<R>>::dimensions_static(cx),
            _r: PhantomData,
        }
    }
    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.items.get_mut(index)
    }
    pub fn height(&self) -> u16 {
        self.item_size.height + 4
    }
}

impl<R: 'static, T: ItemWidget<R>> Index<usize> for ItemList<R, T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.items[index]
    }
}

impl<R: 'static, T: ItemWidget<R>> IndexMut<usize> for ItemList<R, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.items[index]
    }
}

#[derive(Debug)]
pub enum ItemListAction<T> {
    SpecificInner(usize, T),
    CurrentInner(T),
    Left,
    Right,
}

pub struct ItemListState<R: 'static, T: ItemState<R>> {
    pub items: Vec<T>,
    pub title: String,
    pub current: usize,
    _r: PhantomData<fn(R) -> ()>,
}

impl<R: 'static, T: ItemState<R>> std::fmt::Debug for ItemListState<R, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ItemListData")
            .field("items", &self.items)
            .field("title", &self.title)
            .field("current", &self.current)
            .finish()
    }
}

impl<R: 'static, T: ItemState<R>> JellyhajWidgetState<R> for ItemListState<R, T> {
    type Action = ItemListAction<T::IAction>;

    type ActionResult = <T as ItemState<R>>::IActionResult;

    type Widget = ItemList<R, T::IWidget>;

    const NAME: &str = "item-list";

    fn visit_children(visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        visitor.visit_item::<R, T>();
    }

    fn into_widget(self, cx: &R) -> Self::Widget {
        let item_size = <T::IWidget>::dimensions_static(cx);
        ItemList {
            items: self
                .items
                .into_iter()
                .map(|i| i.item_into_widget(cx))
                .collect(),
            current: self.current,
            title: self.title,
            active: false,
            offset: 0,
            item_size,
            _r: PhantomData,
        }
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        match action {
            ItemListAction::SpecificInner(index, action) => self
                .items
                .get_mut(index)
                .and_then(|v| {
                    v.item_apply_action(cx.wrap_with(ListWrapper { index }), action)
                        .transpose()
                })
                .transpose(),
            ItemListAction::CurrentInner(action) => self
                .items
                .get_mut(self.current)
                .and_then(|v| {
                    v.item_apply_action(
                        cx.wrap_with(ListWrapper {
                            index: self.current,
                        }),
                        action,
                    )
                    .transpose()
                })
                .transpose(),
            ItemListAction::Left => {
                self.current = self.current.saturating_sub(1);
                Ok(None)
            }
            ItemListAction::Right => {
                self.current = self.current.saturating_add(1);
                Ok(None)
            }
        }
    }
}

impl<R: 'static, T: ItemState<R>> ItemListState<R, T> {
    pub fn new(items: impl IntoIterator<Item = T>, title: String) -> Self {
        Self {
            items: items.into_iter().collect(),
            title,
            current: 0,
            _r: PhantomData,
        }
    }
}

#[derive(Clone, Copy)]
struct ListWrapper {
    index: usize,
}

impl<T: Send + 'static> Wrapper<T> for ListWrapper {
    type F = ItemListAction<T>;

    fn wrap(&self, val: T) -> Self::F {
        ItemListAction::SpecificInner(self.index, val)
    }
}

impl<R: 'static, T: ItemWidget<R>> JellyhajWidget<R> for ItemList<R, T> {
    type State = ItemListState<R, T::IState>;

    type Action = ItemListAction<<T as ItemWidget<R>>::IAction>;

    type ActionResult = <T as ItemWidget<R>>::IActionResult;

    fn into_state(self) -> Self::State {
        ItemListState {
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
            ItemListAction::SpecificInner(index, action) => self
                .items
                .get_mut(index)
                .and_then(|v| {
                    v.item_apply_action(cx.wrap_with(ListWrapper { index }), action)
                        .transpose()
                })
                .transpose(),
            ItemListAction::CurrentInner(action) => self
                .items
                .get_mut(self.current)
                .and_then(|v| {
                    v.item_apply_action(
                        cx.wrap_with(ListWrapper {
                            index: self.current,
                        }),
                        action,
                    )
                    .transpose()
                })
                .transpose(),
            ItemListAction::Left => {
                self.current = self.current.saturating_sub(1);
                Ok(None)
            }
            ItemListAction::Right => {
                self.current = self.current.saturating_add(1);
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
            let index = position.x / (self.item_size.width + 1);
            let index = (index as usize) + self.offset;
            let x_position = position.x % (self.item_size.width + 1);
            if x_position < self.item_size.width
                && let Some(item) = self.items.get_mut(index)
            {
                self.current = index;
                item.item_click(
                    cx.wrap_with(ListWrapper { index }),
                    Position {
                        x: x_position,
                        y: position.y,
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

    #[instrument(skip_all, name = "render_list")]
    fn render_fallible_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> jellyhaj_widgets_core::Result<()> {
        self.current = min(self.current, self.items.len().saturating_sub(1));
        let outer = Block::bordered()
            .title_top(self.title.as_str())
            .padding(Padding::uniform(1));
        let main = outer.inner(area);
        let visible = min(
            self.items.len(),
            ((main.width + 1) / (self.item_size.width + 1)).into(),
        );
        self.offset = if visible < self.items.len()
            && let position_in_visible = visible / 2
            && self.current > position_in_visible
        {
            min(
                self.current - position_in_visible,
                self.items.len() - visible,
            )
        } else {
            0
        };

        for ((i, item), x) in self
            .items
            .iter_mut()
            .enumerate()
            .skip(self.offset)
            .zip((0..visible as u16).map(|i| main.x + i * (self.item_size.width + 1)))
        {
            item.set_active(self.active && i == self.current);
            let area = Rect {
                x,
                y: main.y,
                width: self.item_size.width,
                height: main.height,
            };
            item.render_item(area, buf, cx.wrap_with(ListWrapper { index: i }))?
        }
        if visible < self.items.len() {
            Scrollbar::new(HorizontalBottom).render(
                area,
                buf,
                &mut ScrollbarState::new(self.items.len())
                    .position(self.current)
                    .viewport_content_length(self.item_size.width as usize + 1),
            );
        }
        outer.render(area, buf);
        Ok(())
    }

    fn min_width(&self) -> Option<u16> {
        Some(self.item_size.width + 4)
    }

    fn min_height(&self) -> Option<u16> {
        Some(self.item_size.height + 4)
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
