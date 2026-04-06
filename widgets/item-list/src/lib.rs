use std::{
    cmp::min,
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use jellyhaj_widgets_core::{
    ItemWidget, ItemWidgetExt, JellyhajWidget, WidgetContext, WidgetTreeVisitor, Wrapper,
    valuable::{Fields, NamedField, NamedValues, StructDef, Structable, Valuable, Value, Visit},
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
pub struct ItemList<T> {
    items: Vec<T>,
    current: usize,
    title: String,
    pub active: bool,
    offset: usize,
    item_size: Size,
}

pub fn new_item_list<R: 'static, T: ItemWidget<R>>(
    items: impl IntoIterator<Item = T>,
    title: String,
    cx: &R,
) -> ItemList<T> {
    ItemList {
        items: items.into_iter().collect(),
        current: 0,
        title,
        active: false,
        offset: 0,
        item_size: T::dimensions_static(cx),
    }
}

static ITEM_LIST_FIELDS: &[NamedField] = &[
    NamedField::new("current"),
    NamedField::new("title"),
    NamedField::new("active"),
];

impl<T> Valuable for ItemList<T> {
    fn as_value(&self) -> Value<'_> {
        Value::Structable(self)
    }

    fn visit(&self, visit: &mut dyn Visit) {
        visit.visit_named_fields(&NamedValues::new(
            ITEM_LIST_FIELDS,
            &[
                self.current.as_value(),
                self.title.as_value(),
                self.active.as_value(),
            ],
        ))
    }
}

impl<T> Structable for ItemList<T> {
    fn definition(&self) -> StructDef<'_> {
        StructDef::new_static("ItemList", Fields::Named(ITEM_LIST_FIELDS))
    }
}

impl<T> ItemList<T> {
    pub fn height(&self) -> u16 {
        self.item_size.height + 4
    }
}

impl<T> Deref for ItemList<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl<T> DerefMut for ItemList<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}

#[derive(Debug)]
pub enum ItemListAction<T> {
    SpecificInner(usize, T),
    CurrentInner(T),
    Left,
    Right,
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

impl<R: 'static, T: ItemWidget<R>> JellyhajWidget<R> for ItemList<T> {
    type Action = ItemListAction<<T as ItemWidget<R>>::IAction>;

    type ActionResult = <T as ItemWidget<R>>::IActionResult;

    const NAME: &str = "item-list";

    fn visit_children(&self, visitor: &mut impl WidgetTreeVisitor) {
        for item in &self.items {
            visitor.visit_item(item);
        }
    }

    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {
        for (index, item) in self.items.iter_mut().enumerate() {
            item.init(cx.wrap_with(ListWrapper { index }));
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

    #[instrument(skip(self, buf,cx), name = "render_list")]
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
        let cur = self.current;
        if let Some(i) = self.get_mut(cur)
            && i.item_accepts_text_input()
        {
            i.item_accept_char(text);
        }
    }

    fn accept_text(&mut self, text: String) {
        let cur = self.current;
        if let Some(i) = self.get_mut(cur)
            && i.item_accepts_text_input()
        {
            i.item_accept_text(text);
        }
    }
}
