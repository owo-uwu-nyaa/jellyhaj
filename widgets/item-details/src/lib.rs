use std::{cmp::min, convert::Infallible, sync::Arc};

use jellyfin::{JellyfinClient, items::MediaItem};
use jellyhaj_core::{Config, context::DB};
use jellyhaj_entry_widget::{
    Entry, EntryAction, EntryData, EntryResult, ImageProtocolCache, Picker, Stats,
};
use jellyhaj_item_list::{ItemList, ItemListAction, ItemListData};
use jellyhaj_widgets_core::{
    ItemWidget, JellyhajWidget, JellyhajWidgetExt, Rect, Wrapper, async_task::TaskSubmitter,
};
use ratatui::{
    layout::Margin,
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
        Ok(())
    }
}

#[derive(Debug)]
pub enum DisplayAction {
    Inner(EntryAction),
    Up,
    Down,
}

pub struct ItemDisplay {
    entry: Entry,
    overview: Option<Overview>,
}

impl ItemDisplay {
    pub fn new(
        item: MediaItem,
        jellyfin: &JellyfinClient,
        db: &DB,
        cache: &ImageProtocolCache,
        picker: &Arc<Picker>,
        stats: &Stats,
        config: &Config,
    ) -> Self {
        let overview = item
            .overview
            .as_ref()
            .map(|o| Overview::new(o.clone(), "Overview".to_string()));
        Self {
            entry: Entry::new(
                EntryData::Item(item),
                jellyfin,
                db,
                cache,
                picker,
                stats,
                config,
            ),
            overview,
        }
    }
}

impl JellyhajWidget for ItemDisplay {
    type State = MediaItem;

    type Action = DisplayAction;

    type ActionResult = EntryResult;

    fn min_width(&self) -> Option<u16> {
        Some(self.entry.dimensions().width + 4)
    }

    fn min_height(&self) -> Option<u16> {
        Some(self.entry.dimensions().height + 8)
    }

    fn into_state(self) -> Self::State {
        JellyhajWidget::into_state(self.entry)
            .into_item()
            .expect("entry has been initialized with an item")
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
                    o.apply_action(task.wrap_with(|_| unreachable!()), OverviewAction::Up)?;
                }
                Ok(None)
            }
            DisplayAction::Down => {
                if let Some(o) = self.overview.as_mut() {
                    o.apply_action(task.wrap_with(|_| unreachable!()), OverviewAction::Down)?;
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

pub struct ItemListState {
    pub children: ItemListData<EntryData>,
    pub item: MediaItem,
}

impl JellyhajWidget for ItemListDisplay {
    type State = ItemListState;

    type Action = DisplayListAction;

    type ActionResult = EntryResult;

    fn min_width(&self) -> Option<u16> {
        Some(9)
    }

    fn min_height(&self) -> Option<u16> {
        Some(12 + self.children.height())
    }

    fn into_state(self) -> Self::State {
        ItemListState {
            children: self.children.into_state(),
            item: self.item,
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
                    o.apply_action(task.wrap_with(|_| unimplemented!()), OverviewAction::Up)?;
                }
                Ok(None)
            }
            DisplayListAction::Down => {
                if let Some(o) = self.overview.as_mut() {
                    o.apply_action(task.wrap_with(|_| unimplemented!()), OverviewAction::Down)?;
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
            (size.width - 2, self.children.height()).into(),
        ));
        if area.contains(pos) {
            pos.x -= 2;
            pos.y -= 2;
            self.children.click(
                task.wrap_with(DisplayListAction::Inner),
                pos,
                (size.width - 2, self.children.height()).into(),
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
                (area.width - 2, self.children.height()).into(),
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
