use std::{
    borrow::Cow,
    cmp::min,
    fmt::Debug,
    ops::{Deref, DerefMut, Index, IndexMut},
};

pub use jellyhaj_item_list::{ItemList, ItemListAction, new_item_list};
use jellyhaj_widgets_core::{
    ItemWidget, JellyhajWidget, JellyhajWidgetExt, Result, WidgetContext, Wrapper,
    spawn::tracing::instrument,
    valuable::{Fields, NamedField, NamedValues, StructDef, Structable, Valuable, Value},
};
use ratatui::{
    layout::{Position, Rect, Size},
    widgets::{
        Block, Padding, Scrollbar, ScrollbarOrientation::HorizontalBottom, ScrollbarState,
        StatefulWidget, Widget,
    },
};

pub fn new_item_screen<R: 'static, W: ItemWidget<R>>(
    lists: impl IntoIterator<Item = ItemList<W>>,
    title: impl Into<Cow<'static, str>>,
    cx: &R,
) -> ItemScreen<W> {
    ItemScreen {
        lists: lists.into_iter().collect(),
        current: 0,
        title: title.into(),
        item_size: W::dimensions_static(cx),
        offset: 0,
    }
}

pub struct ItemScreen<T> {
    lists: Vec<ItemList<T>>,
    current: usize,
    title: Cow<'static, str>,
    item_size: Size,
    offset: usize,
}

impl<T> Deref for ItemScreen<T> {
    type Target = [ItemList<T>];

    fn deref(&self) -> &Self::Target {
        &self.lists
    }
}

impl<T> DerefMut for ItemScreen<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.lists
    }
}

static ITEM_SCREEN_FIELDS: &[NamedField] = &[NamedField::new("current"), NamedField::new("title")];

impl<T> Valuable for ItemScreen<T> {
    fn as_value(&self) -> Value<'_> {
        Value::Structable(self)
    }

    fn visit(&self, visit: &mut dyn jellyhaj_widgets_core::valuable::Visit) {
        visit.visit_named_fields(&NamedValues::new(
            ITEM_SCREEN_FIELDS,
            &[self.current.as_value(), self.title.deref().as_value()],
        ));
    }
}

impl<T> Structable for ItemScreen<T> {
    fn definition(&self) -> StructDef<'_> {
        StructDef::new_static("ItemScreen", Fields::Named(ITEM_SCREEN_FIELDS))
    }
}

impl<T> ItemScreen<T> {
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&ItemList<T>> {
        self.lists.get(index)
    }
    pub fn get_mut(&mut self, index: usize) -> Option<&mut ItemList<T>> {
        self.lists.get_mut(index)
    }
}

impl<T> Index<usize> for ItemScreen<T> {
    type Output = ItemList<T>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.lists[index]
    }
}

impl<T> IndexMut<usize> for ItemScreen<T> {
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

impl<R: 'static, T: ItemWidget<R>> JellyhajWidget<R> for ItemScreen<T> {
    type Action = ItemScreenAction<<T as ItemWidget<R>>::IAction>;
    type ActionResult = <T as ItemWidget<R>>::IActionResult;

    const NAME: &str = "item-screen";

    fn visit_children(&self, visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        for list in &self.lists {
            visitor.visit(list);
        }
    }

    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {
        for (index, list) in self.lists.iter_mut().enumerate() {
            list.init(cx.wrap_with(ScreenWrapper { index }));
        }
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        fn apply<R: 'static, T: ItemWidget<R>>(
            this: &mut ItemScreen<T>,
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

    #[instrument(skip(self, buf, cx), name = "render_screen")]
    fn render_fallible_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()> {
        let outer = Block::bordered()
            .title_top(&*self.title)
            .padding(Padding::uniform(1));
        let main = outer.inner(area);
        let visible = u16::try_from(min(
            self.lists.len(),
            ((main.height + 1) / (self.item_size.height + 5)).into(),
        ))
        .expect("bounded by height/entry height");
        self.offset = if (visible as usize) < self.lists.len()
            && let position_in_visible = visible / 2
            && self.current > position_in_visible as usize
        {
            min(
                self.current - position_in_visible as usize,
                self.lists.len() - visible as usize,
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
            list.render_fallible(area, buf, cx.wrap_with(ScreenWrapper { index: i }))?;
        }
        if (visible as usize) < self.lists.len() {
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
            .is_some_and(JellyhajWidget::accepts_text_input)
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
