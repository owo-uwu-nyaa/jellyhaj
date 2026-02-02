use std::{
    cmp::min,
    ops::{Index, IndexMut},
};

use jellyhaj_widgets_core::{
    ItemWidget, JellyhajWidget, JellyhajWidgetExt, Wrapper, async_task::TaskSubmitter,
};
use ratatui::{
    layout::{Position, Rect, Size},
    widgets::{
        Block, Padding, Scrollbar, ScrollbarOrientation::HorizontalBottom, ScrollbarState,
        StatefulWidget, Widget,
    },
};
use tracing::instrument;

pub use jellyhaj_widgets_core::DimensionsParameter;

#[derive(Debug)]
pub struct ItemList<T: ItemWidget> {
    items: Vec<T>,
    current: usize,
    title: String,
    pub active: bool,
    offset: usize,
    item_size: Size,
}

impl<T: ItemWidget> ItemList<T> {
    pub fn new(
        items: impl IntoIterator<Item = T>,
        current: usize,
        title: String,
        dim: DimensionsParameter<'_>,
    ) -> Self {
        Self {
            items: items.into_iter().collect(),
            current,
            title,
            active: false,
            offset: 0,
            item_size: <T as ItemWidget>::dimensions_static(dim),
        }
    }
    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.items.get_mut(index)
    }
}

impl<T: ItemWidget> Index<usize> for ItemList<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.items[index]
    }
}

impl<T: ItemWidget> IndexMut<usize> for ItemList<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.items[index]
    }
}

pub enum ItemListAction<T> {
    SpecificInner(usize, T),
    CurrentInner(T),
    Left,
    Right,
}

#[derive(Debug)]
pub struct ItemListData<T> {
    pub items: Vec<T>,
    pub title: String,
    pub current: usize,
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

impl<T: ItemWidget> JellyhajWidget for ItemList<T> {
    type State = ItemListData<<T as ItemWidget>::State>;

    type Action = ItemListAction<<T as ItemWidget>::Action>;

    type ActionResult = <T as ItemWidget>::ActionResult;

    fn into_state(self) -> Self::State {
        ItemListData {
            items: self.items.into_iter().map(ItemWidget::into_state).collect(),
            title: self.title,
            current: self.current,
        }
    }

    fn apply_action(
        &mut self,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        match action {
            ItemListAction::SpecificInner(index, action) => self
                .items
                .get_mut(index)
                .and_then(|v| {
                    v.apply_action(
                        TaskSubmitter::clone(&task).wrap_with(ListWrapper { index }),
                        action,
                    )
                    .transpose()
                })
                .transpose(),
            ItemListAction::CurrentInner(action) => self
                .items
                .get_mut(self.current)
                .and_then(|v| {
                    v.apply_action(
                        TaskSubmitter::clone(&task).wrap_with(ListWrapper {
                            index: self.current,
                        }),
                        action,
                    )
                    .transpose()
                })
                .transpose(),
            ItemListAction::Left => {
                self.current = min(self.items.len(), self.current + 1);
                Ok(None)
            }
            ItemListAction::Right => {
                self.current = self.current.saturating_sub(1);
                Ok(None)
            }
        }
    }

    fn click(
        &mut self,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
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
                ItemWidget::click(
                    item,
                    TaskSubmitter::clone(&task).wrap_with(ListWrapper { index }),
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
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
    ) -> jellyhaj_widgets_core::Result<()> {
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
            item.render_fallible(
                area,
                buf,
                TaskSubmitter::clone(&task).wrap_with(ListWrapper { index: i }),
            )?
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
