use std::{cmp::min, convert::Infallible};

use jellyfin::items::MediaItem;
use jellyhaj_core::state::Navigation;
use jellyhaj_entry_widget::{Entry, EntryAction, EntryState};
use jellyhaj_item_list::{ItemList, ItemListAction, ItemListState};
use jellyhaj_widgets_core::{
    ItemWidget, JellyhajWidget, JellyhajWidgetExt, JellyhajWidgetState, Rect, Wrapper,
    async_task::TaskSubmitter,
};
use ratatui::{
    layout::{HorizontalAlignment, Margin},
    symbols::merge::MergeStrategy,
    text::Line,
    widgets::{
        Block, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Widget,
    },
};

#[derive(Debug)]
pub enum OverviewAction {
    Up,
    Down,
}

#[derive(Debug)]
pub struct Overview {
    pub text: String,
    pub title: String,
    pub scroll: usize,
}

impl Overview {
    pub fn new(text: String, title: String) -> Self {
        Self {
            text,
            title,
            scroll: 0,
        }
    }
}

impl JellyhajWidgetState for Overview {
    type Action = OverviewAction;

    type ActionResult = Infallible;

    type Widget = Self;

    const NAME: &str = "overview";

    fn visit_children(_: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {}

    fn into_widget(
        self,
        _: std::pin::Pin<&mut jellyhaj_core::context::TuiContext>,
    ) -> Self::Widget {
        self
    }

    fn apply_action(
        &mut self,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        JellyhajWidget::apply_action(self, task, action)
    }
}

impl JellyhajWidget for Overview {
    type State = Self;

    type Action = OverviewAction;

    type ActionResult = Infallible;

    fn min_width(&self) -> Option<u16> {
        Some(5)
    }

    fn min_height(&self) -> Option<u16> {
        Some(5)
    }

    fn into_state(self) -> Self::State {
        self
    }

    fn accepts_text_input(&self) -> bool {
        false
    }

    fn accept_char(&mut self, _: char) {
        unimplemented!()
    }

    fn accept_text(&mut self, _: String) {
        unimplemented!()
    }

    fn apply_action(
        &mut self,
        _: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        match action {
            OverviewAction::Up => self.scroll = self.scroll.saturating_sub(1),
            OverviewAction::Down => self.scroll = self.scroll.saturating_add(1),
        }
        Ok(None)
    }

    fn click(
        &mut self,
        _task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        _position: jellyhaj_widgets_core::Position,
        _size: jellyhaj_widgets_core::Size,
        _kind: jellyhaj_widgets_core::MouseEventKind,
        _modifier: jellyhaj_widgets_core::KeyModifiers,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        Ok(None)
    }

    fn render_fallible_inner(
        &mut self,
        area: jellyhaj_widgets_core::Rect,
        buf: &mut jellyhaj_widgets_core::Buffer,
        _: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
    ) -> jellyhaj_widgets_core::Result<()> {
        let outer = Block::bordered()
            .title("Overview")
            .title_alignment(HorizontalAlignment::Center)
            .padding(Padding::uniform(1));
        let main = outer.inner(area);
        let lines: Vec<_> = self
            .text
            .lines()
            .flat_map(|line| textwrap::wrap(line, main.width as usize))
            .map(Line::from)
            .collect();
        let len = lines.len();
        self.scroll = min(self.scroll, lines.len() - 1);
        Paragraph::new(lines)
            .scroll((self.scroll as u16, 0))
            .render(main, buf);
        Scrollbar::new(ScrollbarOrientation::VerticalRight).render(
            area.inner(Margin {
                horizontal: 0,
                vertical: 2,
            }),
            buf,
            &mut ScrollbarState::new(len).position(self.scroll),
        );
        outer.render(area, buf);
        Ok(())
    }
}

#[derive(Debug)]
pub enum DisplayAction {
    Inner(EntryAction),
    Up,
    Down,
}

#[derive(Debug)]
pub struct ItemDisplayState {
    entry: EntryState,
    overview: Option<Overview>,
}

impl ItemDisplayState {
    pub fn new(entry: EntryState, overview: Option<Overview>) -> Self {
        Self { entry, overview }
    }
}

impl JellyhajWidgetState for ItemDisplayState {
    type Action = DisplayAction;

    type ActionResult = Navigation;

    type Widget = ItemDisplay;

    const NAME: &str = "item-display";

    fn visit_children(visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        visitor.visit::<EntryState>();
        visitor.visit::<Overview>();
    }

    fn into_widget(
        self,
        cx: std::pin::Pin<&mut jellyhaj_core::context::TuiContext>,
    ) -> Self::Widget {
        ItemDisplay {
            entry: self.entry.into_widget(cx),
            overview: self.overview,
        }
    }

    fn apply_action(
        &mut self,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        match action {
            DisplayAction::Inner(action) => JellyhajWidgetState::apply_action(
                &mut self.entry,
                task.wrap_with(DisplayAction::Inner),
                action,
            ),
            DisplayAction::Up => {
                if let Some(o) = self.overview.as_mut() {
                    JellyhajWidget::apply_action(
                        o,
                        task.wrap_with(|_| unreachable!()),
                        OverviewAction::Up,
                    )?;
                }
                Ok(None)
            }
            DisplayAction::Down => {
                if let Some(o) = self.overview.as_mut() {
                    JellyhajWidget::apply_action(
                        o,
                        task.wrap_with(|_| unreachable!()),
                        OverviewAction::Down,
                    )?;
                }
                Ok(None)
            }
        }
    }
}

pub struct ItemDisplay {
    entry: Entry,
    overview: Option<Overview>,
}

impl JellyhajWidget for ItemDisplay {
    type State = ItemDisplayState;

    type Action = DisplayAction;

    type ActionResult = Navigation;

    fn min_width(&self) -> Option<u16> {
        Some(self.entry.dimensions().width + 4)
    }

    fn min_height(&self) -> Option<u16> {
        Some(self.entry.dimensions().height + 8)
    }

    fn into_state(self) -> Self::State {
        ItemDisplayState {
            entry: ItemWidget::into_state(self.entry),
            overview: self.overview,
        }
    }

    fn accepts_text_input(&self) -> bool {
        false
    }

    fn accept_char(&mut self, _: char) {
        unimplemented!()
    }

    fn accept_text(&mut self, _: String) {
        unimplemented!()
    }

    fn apply_action(
        &mut self,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        match action {
            DisplayAction::Inner(action) => JellyhajWidget::apply_action(
                &mut self.entry,
                task.wrap_with(DisplayAction::Inner),
                action,
            ),
            DisplayAction::Up => {
                if let Some(o) = self.overview.as_mut() {
                    JellyhajWidget::apply_action(
                        o,
                        task.wrap_with(|_| unreachable!()),
                        OverviewAction::Up,
                    )?;
                }
                Ok(None)
            }
            DisplayAction::Down => {
                if let Some(o) = self.overview.as_mut() {
                    JellyhajWidget::apply_action(
                        o,
                        task.wrap_with(|_| unreachable!()),
                        OverviewAction::Down,
                    )?;
                }
                Ok(None)
            }
        }
    }

    fn click(
        &mut self,
        _: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        _: ratatui::prelude::Position,
        _: ratatui::prelude::Size,
        _: jellyhaj_widgets_core::MouseEventKind,
        _: jellyhaj_widgets_core::KeyModifiers,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        Ok(None)
    }

    fn render_fallible_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
    ) -> jellyhaj_widgets_core::Result<()> {
        let entry_off = (area.width - self.entry.dimensions().width) / 2;
        self.entry.render_fallible(
            (
                (area.x + entry_off, area.y + 2).into(),
                self.entry.dimensions(),
            )
                .into(),
            buf,
            task.clone().wrap_with(DisplayAction::Inner),
        )?;
        if let Some(overview) = self.overview.as_mut() {
            overview.render_fallible(
                (
                    (area.x, area.y + 4 + self.entry.dimensions().height).into(),
                    (area.width, area.height - 4 - self.entry.dimensions().height).into(),
                )
                    .into(),
                buf,
                task.wrap_with(|_| unreachable!()),
            )?
        }
        Block::bordered()
            .title(
                self.entry
                    .data()
                    .item()
                    .expect("initialized with item")
                    .name
                    .as_str(),
            )
            .merge_borders(MergeStrategy::Fuzzy)
            .render(area, buf);

        Ok(())
    }
}

#[derive(Debug)]
pub enum DisplayListAction {
    Inner(ItemListAction<EntryAction>),
    Up,
    Down,
    Left,
    Right,
}

pub struct ItemListDisplay {
    children: ItemList<Entry>,
    item: MediaItem,
    overview: Option<Overview>,
}

impl ItemListDisplay {
    pub fn new(item: MediaItem, mut children: ItemList<Entry>) -> Self {
        let overview = item
            .overview
            .as_ref()
            .map(|o| Overview::new(o.clone(), "Overview".to_string()));
        children.active = true;
        Self {
            children,
            item,
            overview,
        }
    }
}

#[derive(Debug)]
pub struct ItemListDisplayState {
    pub children: ItemListState<Entry>,
    pub item: MediaItem,
    pub overview: Option<Overview>,
}

impl JellyhajWidgetState for ItemListDisplayState {
    type Action = DisplayListAction;

    type ActionResult = Navigation;

    type Widget = ItemListDisplay;

    const NAME: &str = "item-list-details";

    fn visit_children(visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        visitor.visit::<ItemListState<Entry>>();
        visitor.visit::<Overview>();
    }

    fn into_widget(
        self,
        cx: std::pin::Pin<&mut jellyhaj_core::context::TuiContext>,
    ) -> Self::Widget {
        ItemListDisplay {
            children: self.children.into_widget(cx),
            item: self.item,
            overview: self.overview,
        }
    }

    fn apply_action(
        &mut self,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        match action {
            DisplayListAction::Inner(action) => self
                .children
                .apply_action(task.wrap_with(DisplayListAction::Inner), action),
            DisplayListAction::Up => {
                if let Some(o) = self.overview.as_mut() {
                    JellyhajWidget::apply_action(
                        o,
                        task.wrap_with(|_| unimplemented!()),
                        OverviewAction::Up,
                    )?;
                }
                Ok(None)
            }
            DisplayListAction::Down => {
                if let Some(o) = self.overview.as_mut() {
                    JellyhajWidget::apply_action(
                        o,
                        task.wrap_with(|_| unimplemented!()),
                        OverviewAction::Down,
                    )?;
                }
                Ok(None)
            }
            DisplayListAction::Left => self.children.apply_action(
                task.wrap_with(DisplayListAction::Inner),
                ItemListAction::Left,
            ),
            DisplayListAction::Right => self.children.apply_action(
                task.wrap_with(DisplayListAction::Inner),
                ItemListAction::Right,
            ),
        }
    }
}

impl JellyhajWidget for ItemListDisplay {
    type State = ItemListDisplayState;

    type Action = DisplayListAction;

    type ActionResult = Navigation;

    fn min_width(&self) -> Option<u16> {
        Some(9)
    }

    fn min_height(&self) -> Option<u16> {
        Some(12 + self.children.height())
    }

    fn into_state(self) -> Self::State {
        ItemListDisplayState {
            item: self.item,
            overview: self.overview,
            children: self.children.into_state(),
        }
    }

    fn accepts_text_input(&self) -> bool {
        false
    }

    fn accept_char(&mut self, _: char) {
        unimplemented!()
    }

    fn accept_text(&mut self, _: String) {
        unimplemented!()
    }

    fn apply_action(
        &mut self,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        match action {
            DisplayListAction::Inner(action) => self
                .children
                .apply_action(task.wrap_with(DisplayListAction::Inner), action),
            DisplayListAction::Up => {
                if let Some(o) = self.overview.as_mut() {
                    JellyhajWidget::apply_action(
                        o,
                        task.wrap_with(|_| unimplemented!()),
                        OverviewAction::Up,
                    )?;
                }
                Ok(None)
            }
            DisplayListAction::Down => {
                if let Some(o) = self.overview.as_mut() {
                    JellyhajWidget::apply_action(
                        o,
                        task.wrap_with(|_| unimplemented!()),
                        OverviewAction::Down,
                    )?;
                }
                Ok(None)
            }
            DisplayListAction::Left => self.children.apply_action(
                task.wrap_with(DisplayListAction::Inner),
                ItemListAction::Left,
            ),
            DisplayListAction::Right => self.children.apply_action(
                task.wrap_with(DisplayListAction::Inner),
                ItemListAction::Right,
            ),
        }
    }

    fn click(
        &mut self,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        mut pos: ratatui::prelude::Position,
        size: ratatui::prelude::Size,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        let area = Rect::from((
            (2, 2).into(),
            (size.width - 4, self.children.height()).into(),
        ));
        if area.contains(pos) {
            pos.x -= 2;
            pos.y -= 2;
            self.children.click(
                task.wrap_with(DisplayListAction::Inner),
                pos,
                (size.width - 4, self.children.height()).into(),
                kind,
                modifier,
            )
        } else {
            Ok(None)
        }
    }

    fn render_fallible_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
    ) -> jellyhaj_widgets_core::Result<()> {
        self.children.render_fallible(
            (
                (area.x + 2, area.y + 2).into(),
                (area.width - 4, self.children.height()).into(),
            )
                .into(),
            buf,
            task.clone().wrap_with(DisplayListAction::Inner),
        )?;
        if let Some(overview) = self.overview.as_mut() {
            overview.render_fallible(
                (
                    (area.x, area.y + 4 + self.children.height()).into(),
                    (area.width, area.height - 4 - self.children.height()).into(),
                )
                    .into(),
                buf,
                task.wrap_with(|_| unreachable!()),
            )?
        }
        Block::bordered()
            .title(self.item.name.as_str())
            .merge_borders(MergeStrategy::Fuzzy)
            .render(area, buf);

        Ok(())
    }
}
