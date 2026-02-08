use std::{
    cmp::min,
    ops::{Index, IndexMut},
};

pub use jellyhaj_item_list::{DimensionsParameter, ItemList, ItemListAction, ItemListData};
use jellyhaj_widgets_core::{
    ItemWidget, JellyhajWidget, JellyhajWidgetExt, Result, Wrapper, async_task::TaskSubmitter,
};
use ratatui::{
    layout::{Position, Rect, Size},
    widgets::{
        Block, Padding, Scrollbar, ScrollbarOrientation::HorizontalBottom, ScrollbarState,
        StatefulWidget, Widget,
    },
};

pub struct ItemScreen<T: ItemWidget> {
    lists: Vec<ItemList<T>>,
    current: usize,
    title: String,
    item_size: Size,
    offset: usize,
}

impl<T: ItemWidget> ItemScreen<T> {
    pub fn new(
        lists: impl IntoIterator<Item = ItemList<T>>,
        current: usize,
        title: String,
        dim: DimensionsParameter<'_>,
    ) -> Self {
        Self {
            lists: lists.into_iter().collect(),
            current,
            title,
            item_size: <T as ItemWidget>::dimensions_static(dim),
            offset: 0,
        }
    }
    pub fn get(&self, index: usize) -> Option<&ItemList<T>> {
        self.lists.get(index)
    }
    pub fn get_mut(&mut self, index: usize) -> Option<&mut ItemList<T>> {
        self.lists.get_mut(index)
    }
}

impl<T: ItemWidget> Index<usize> for ItemScreen<T> {
    type Output = ItemList<T>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.lists[index]
    }
}

impl<T: ItemWidget> IndexMut<usize> for ItemScreen<T> {
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

#[derive(Debug)]
pub struct ItemScreenData<T> {
    pub lists: Vec<ItemListData<T>>,
    pub title: String,
    pub current: usize,
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
            _ => unreachable!(),
        }
    }
}

impl<T: ItemWidget> JellyhajWidget for ItemScreen<T> {
    type State = ItemScreenData<<T as ItemWidget>::State>;
    type Action = ItemScreenAction<<T as ItemWidget>::Action>;
    type ActionResult = <T as ItemWidget>::ActionResult;

    fn into_state(self) -> Self::State {
        ItemScreenData {
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
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        fn apply<T: ItemWidget>(
            this: &mut ItemScreen<T>,
            task: TaskSubmitter<
                ItemScreenAction<T::Action>,
                impl Wrapper<ItemScreenAction<T::Action>>,
            >,
            index: usize,
            action: ItemListAction<<T as ItemWidget>::Action>,
        ) -> Result<Option<<T as ItemWidget>::ActionResult>> {
            this.lists
                .get_mut(index)
                .and_then(|r| {
                    r.apply_action(task.wrap_with(ScreenWrapper { index }), action)
                        .transpose()
                })
                .transpose()
        }
        match action {
            ItemScreenAction::SpecificInner { row, item, action } => {
                apply(self, task, row, ItemListAction::SpecificInner(item, action))
            }
            ItemScreenAction::CurrentInner(action) => apply(
                self,
                task,
                self.current,
                ItemListAction::CurrentInner(action),
            ),
            ItemScreenAction::Left => apply(self, task, self.current, ItemListAction::Left),
            ItemScreenAction::Right => apply(self, task, self.current, ItemListAction::Right),
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
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
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
                    task.wrap_with(ScreenWrapper { index }),
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
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
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
            list.render_fallible(
                area,
                buf,
                TaskSubmitter::clone(&task).wrap_with(ScreenWrapper { index: i }),
            )?
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
