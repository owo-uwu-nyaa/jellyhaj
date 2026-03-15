use std::{
    cmp::min,
    fmt::Debug,
    ops::{Index, IndexMut},
};

pub use jellyhaj_item_list::{ItemList, ItemListAction, ItemListState};
use jellyhaj_widgets_core::{
    ItemState, ItemWidget, JellyhajWidget, JellyhajWidgetExt, JellyhajWidgetState, Result,
    WidgetContext, Wrapper,
};
use ratatui::{
    layout::{Position, Rect, Size},
    widgets::{
        Block, Padding, Scrollbar, ScrollbarOrientation::HorizontalBottom, ScrollbarState,
        StatefulWidget, Widget,
    },
};

pub struct ItemScreen<R: 'static, T: ItemWidget<R>> {
    lists: Vec<ItemList<R, T>>,
    current: usize,
    title: String,
    item_size: Size,
    offset: usize,
}

impl<R: 'static, T: ItemWidget<R>> ItemScreen<R, T> {
    pub fn get(&self, index: usize) -> Option<&ItemList<R, T>> {
        self.lists.get(index)
    }
    pub fn get_mut(&mut self, index: usize) -> Option<&mut ItemList<R, T>> {
        self.lists.get_mut(index)
    }
}

impl<R: 'static, T: ItemWidget<R>> Index<usize> for ItemScreen<R, T> {
    type Output = ItemList<R, T>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.lists[index]
    }
}

impl<R: 'static, T: ItemWidget<R>> IndexMut<usize> for ItemScreen<R, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.lists[index]
    }
}

#[derive(Debug)]
pub enum ItemScreenAction<T> {
    SpecificInner { row: usize, item: usize, action: T },
    CurrentInner(T),
    Left,
    Right,
    Up,
    Down,
}

pub struct ItemScreenState<R: 'static, T: ItemState<R>> {
    pub lists: Vec<ItemListState<R, T>>,
    pub title: String,
    pub current: usize,
}

impl<R: 'static, T: ItemState<R>> ItemScreenState<R, T> {
    pub fn new(lists: Vec<ItemListState<R, T>>, title: String) -> Self {
        Self {
            lists,
            title,
            current: 0,
        }
    }
    pub fn get(&self, index: usize) -> Option<&ItemListState<R, T>> {
        self.lists.get(index)
    }
    pub fn get_mut(&mut self, index: usize) -> Option<&mut ItemListState<R, T>> {
        self.lists.get_mut(index)
    }
}

impl<R: 'static, T: ItemState<R>> Debug for ItemScreenState<R, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ItemScreenState")
            .field("lists", &self.lists)
            .field("title", &self.title)
            .field("current", &self.current)
            .finish()
    }
}

impl<R: 'static, T: ItemState<R>> JellyhajWidgetState<R> for ItemScreenState<R, T> {
    type Action = ItemScreenAction<T::IAction>;

    type ActionResult = T::IActionResult;

    type Widget = ItemScreen<R, T::IWidget>;

    const NAME: &str = "item-screen";

    fn visit_children(visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        visitor.visit::<R, ItemListState<R, T>>();
    }

    fn into_widget(self, cx: &R) -> Self::Widget {
        let item_size = T::IWidget::dimensions_static(cx);
        ItemScreen {
            lists: self.lists.into_iter().map(|l| l.into_widget(cx)).collect(),
            current: self.current,
            title: self.title,
            offset: 0,
            item_size,
        }
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        fn apply<R: 'static, T: ItemState<R>>(
            this: &mut ItemScreenState<R, T>,
            cx: WidgetContext<
                '_,
                ItemScreenAction<T::IAction>,
                impl Wrapper<ItemScreenAction<T::IAction>>,
                R,
            >,
            index: usize,
            action: ItemListAction<T::IAction>,
        ) -> Result<Option<T::IActionResult>> {
            this.lists
                .get_mut(index)
                .and_then(|r| {
                    r.apply_action(cx.wrap_with(ScreenWrapper { index }), action)
                        .transpose()
                })
                .transpose()
        }
        match action {
            ItemScreenAction::SpecificInner { row, item, action } => {
                apply(self, cx, row, ItemListAction::SpecificInner(item, action))
            }
            ItemScreenAction::CurrentInner(action) => {
                apply(self, cx, self.current, ItemListAction::CurrentInner(action))
            }
            ItemScreenAction::Left => apply(self, cx, self.current, ItemListAction::Left),
            ItemScreenAction::Right => apply(self, cx, self.current, ItemListAction::Right),
            ItemScreenAction::Up => {
                self.current = self.current.saturating_sub(1);
                Ok(None)
            }
            ItemScreenAction::Down => {
                self.current = min(self.lists.len(), self.current + 1);
                Ok(None)
            }
        }
    }
}

#[derive(Clone, Copy)]
struct ScreenWrapper {
    index: usize,
}

impl<T: Send + 'static> Wrapper<ItemListAction<T>> for ScreenWrapper {
    type F = ItemScreenAction<T>;

    fn wrap(&self, val: ItemListAction<T>) -> Self::F {
        match val {
            ItemListAction::SpecificInner(item, action) => ItemScreenAction::SpecificInner {
                row: self.index,
                item,
                action,
            },
            _ => unimplemented!(),
        }
    }
}

impl<R: 'static, T: ItemWidget<R>> JellyhajWidget<R> for ItemScreen<R, T> {
    type State = ItemScreenState<R, T::IState>;
    type Action = ItemScreenAction<<T as ItemWidget<R>>::IAction>;
    type ActionResult = <T as ItemWidget<R>>::IActionResult;

    fn into_state(self) -> Self::State {
        ItemScreenState {
            lists: self
                .lists
                .into_iter()
                .map(JellyhajWidget::into_state)
                .collect(),
            title: self.title,
            current: self.current,
        }
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        fn apply<R: 'static, T: ItemWidget<R>>(
            this: &mut ItemScreen<R, T>,
            cx: WidgetContext<
                '_,
                ItemScreenAction<T::IAction>,
                impl Wrapper<ItemScreenAction<T::IAction>>,
                R,
            >,
            index: usize,
            action: ItemListAction<<T as ItemWidget<R>>::IAction>,
        ) -> Result<Option<<T as ItemWidget<R>>::IActionResult>> {
            this.lists
                .get_mut(index)
                .and_then(|r| {
                    r.apply_action(cx.wrap_with(ScreenWrapper { index }), action)
                        .transpose()
                })
                .transpose()
        }
        match action {
            ItemScreenAction::SpecificInner { row, item, action } => {
                apply(self, cx, row, ItemListAction::SpecificInner(item, action))
            }
            ItemScreenAction::CurrentInner(action) => {
                apply(self, cx, self.current, ItemListAction::CurrentInner(action))
            }
            ItemScreenAction::Left => apply(self, cx, self.current, ItemListAction::Left),
            ItemScreenAction::Right => apply(self, cx, self.current, ItemListAction::Right),
            ItemScreenAction::Up => {
                self.current = self.current.saturating_sub(1);
                Ok(None)
            }
            ItemScreenAction::Down => {
                self.current = min(self.lists.len(), self.current + 1);
                Ok(None)
            }
        }
    }

    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        mut position: ratatui::prelude::Position,
        size: ratatui::prelude::Size,
        kind: ratatui::crossterm::event::MouseEventKind,
        modifier: ratatui::crossterm::event::KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        if position.x < 2
            || position.y < 2
            || position.x >= size.width - 2
            || position.y >= size.height - 2
        {
            Ok(None)
        } else {
            position.x -= 2;
            position.y -= 2;
            let index = position.y / (self.item_size.height + 5);
            let index = (index as usize) + self.offset;
            let y_position = position.y % (self.item_size.height + 5);
            if y_position < self.item_size.width + 4
                && let Some(list) = self.lists.get_mut(index)
            {
                JellyhajWidget::click(
                    list,
                    cx.wrap_with(ScreenWrapper { index }),
                    Position {
                        x: position.x,
                        y: y_position,
                    },
                    Size {
                        width: size.width - 4,
                        height: self.item_size.height + 4,
                    },
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
    ) -> Result<()> {
        let outer = Block::bordered()
            .title_top(self.title.as_str())
            .padding(Padding::uniform(1));
        let main = outer.inner(area);
        let visible = min(
            self.lists.len(),
            ((main.height + 1) / (self.item_size.height + 5)).into(),
        );
        self.offset = if visible < self.lists.len()
            && let position_in_visible = visible / 2
            && self.current > position_in_visible
        {
            min(
                self.current - position_in_visible,
                self.lists.len() - visible,
            )
        } else {
            0
        };

        for ((i, list), y) in self
            .lists
            .iter_mut()
            .enumerate()
            .skip(self.offset)
            .zip((0..visible as u16).map(|i| main.y + i * (self.item_size.height + 5)))
        {
            list.active = i == self.current;
            let area = Rect {
                x: main.x,
                y,
                width: main.width,
                height: self.item_size.height + 4,
            };
            list.render_fallible(area, buf, cx.wrap_with(ScreenWrapper { index: i }))?
        }
        if visible < self.lists.len() {
            Scrollbar::new(HorizontalBottom).render(
                area,
                buf,
                &mut ScrollbarState::new(self.lists.len())
                    .position(self.current)
                    .viewport_content_length(self.item_size.width as usize + 1),
            );
        }
        outer.render(area, buf);
        Ok(())
    }

    fn min_width(&self) -> Option<u16> {
        Some(self.item_size.width + 8)
    }

    fn min_height(&self) -> Option<u16> {
        Some(self.item_size.height + 8)
    }

    fn accepts_text_input(&self) -> bool {
        self.get(self.current)
            .map(|i| i.accepts_text_input())
            .unwrap_or(false)
    }

    fn accept_char(&mut self, text: char) {
        if let Some(i) = self.get_mut(self.current)
            && i.accepts_text_input()
        {
            i.accept_char(text);
        }
    }

    fn accept_text(&mut self, text: String) {
        if let Some(i) = self.get_mut(self.current)
            && i.accepts_text_input()
        {
            i.accept_text(text);
        }
    }
}
