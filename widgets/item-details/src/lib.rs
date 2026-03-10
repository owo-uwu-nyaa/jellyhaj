use std::{cmp::min, convert::Infallible, pin::Pin};

use jellyfin::items::MediaItem;
use jellyhaj_core::state::{Navigation, NextScreen};
use jellyhaj_entry_widget::{Entry, EntryAction, EntryState};
use jellyhaj_item_list::{ItemList, ItemListAction, ItemListState};
use jellyhaj_widgets_core::{
    ItemWidget, JellyhajWidget, JellyhajWidgetExt, JellyhajWidgetState, Rect, TuiContext,
    WidgetContext, Wrapper,
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

    fn into_widget(self, _: Pin<&mut jellyhaj_core::context::TuiContext>) -> Self::Widget {
        self
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        JellyhajWidget::apply_action(self, cx, action)
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
        _: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
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
        _: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
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
        _: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
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
    Reload(String),
    Remove,
}

#[derive(Debug)]
pub struct ItemDisplayState {
    entry: EntryState,
    overview: Option<Overview>,
    register: Option<String>,
}

impl ItemDisplayState {
    pub fn new(item: MediaItem, cx: Pin<&mut TuiContext>) -> Self {
        let overview = item
            .overview
            .as_ref()
            .map(|o| Overview::new(o.clone(), "Overview".to_string()));
        let register = Some(item.id.clone());
        Self {
            entry: EntryState::new(item, cx),
            overview,
            register,
        }
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

    fn into_widget(self, cx: Pin<&mut jellyhaj_core::context::TuiContext>) -> Self::Widget {
        ItemDisplay {
            entry: self.entry.into_widget(cx),
            overview: self.overview,
            register: self.register,
        }
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        match action {
            DisplayAction::Inner(action) => JellyhajWidgetState::apply_action(
                &mut self.entry,
                cx.wrap_with(DisplayAction::Inner),
                action,
            ),
            DisplayAction::Up => {
                if let Some(o) = self.overview.as_mut() {
                    JellyhajWidget::apply_action(
                        o,
                        cx.wrap_with(|_| unreachable!()),
                        OverviewAction::Up,
                    )?;
                }
                Ok(None)
            }
            DisplayAction::Down => {
                if let Some(o) = self.overview.as_mut() {
                    JellyhajWidget::apply_action(
                        o,
                        cx.wrap_with(|_| unreachable!()),
                        OverviewAction::Down,
                    )?;
                }
                Ok(None)
            }
            DisplayAction::Reload(id) => Ok(Some(Navigation::Replace(Box::new(
                NextScreen::FetchItemDetails(id),
            )))),
            DisplayAction::Remove => Ok(Some(Navigation::PopContext)),
        }
    }
}

pub struct ItemDisplay {
    entry: Entry,
    overview: Option<Overview>,
    register: Option<String>,
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
            entry: self.entry.item_into_state(),
            overview: self.overview,
            register: self.register,
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
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        match action {
            DisplayAction::Inner(action) => JellyhajWidget::apply_action(
                &mut self.entry,
                cx.wrap_with(DisplayAction::Inner),
                action,
            ),
            DisplayAction::Up => {
                if let Some(o) = self.overview.as_mut() {
                    JellyhajWidget::apply_action(
                        o,
                        cx.wrap_with(|_| unreachable!()),
                        OverviewAction::Up,
                    )?;
                }
                Ok(None)
            }
            DisplayAction::Down => {
                if let Some(o) = self.overview.as_mut() {
                    JellyhajWidget::apply_action(
                        o,
                        cx.wrap_with(|_| unreachable!()),
                        OverviewAction::Down,
                    )?;
                }
                Ok(None)
            }
            DisplayAction::Reload(id) => Ok(Some(Navigation::Replace(Box::new(
                NextScreen::FetchItemDetails(id),
            )))),
            DisplayAction::Remove => Ok(Some(Navigation::PopContext)),
        }
    }

    fn click(
        &mut self,
        _: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
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
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
    ) -> jellyhaj_widgets_core::Result<()> {
        if let Some(register) = self.register.take(){
            let mut events = cx.jellyfin_events.get();
            events.register_item_updated(register.clone(), cx.submitter.wrap_with(DisplayAction::Reload));
            events.register_item_removed(register, cx.submitter.wrap_with(|_|DisplayAction::Remove));
        }
        let entry_off = (area.width - self.entry.dimensions().width) / 2;
        self.entry.render_fallible(
            (
                (area.x + entry_off, area.y + 2).into(),
                self.entry.dimensions(),
            )
                .into(),
            buf,
            cx.wrap_with(DisplayAction::Inner),
        )?;
        if let Some(overview) = self.overview.as_mut() {
            overview.render_fallible(
                (
                    (area.x, area.y + 4 + self.entry.dimensions().height).into(),
                    (area.width, area.height - 4 - self.entry.dimensions().height).into(),
                )
                    .into(),
                buf,
                cx.wrap_with(|_| unreachable!()),
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
    Reload,
    Remove,
}

pub struct ItemListDisplay {
    children: ItemList<Entry>,
    item: MediaItem,
    overview: Option<Overview>,
    register: Option<(String, Vec<String>)>
}

#[derive(Debug)]
pub struct ItemListDisplayState {
    pub children: ItemListState<EntryState>,
    pub item: MediaItem,
    pub overview: Option<Overview>,
    register: Option<(String, Vec<String>)>
}

impl ItemListDisplayState {
    pub fn new(children: Vec<MediaItem>, item: MediaItem, mut cx: Pin<&mut TuiContext>) -> Self {
        let overview = item
            .overview
            .as_ref()
            .map(|o| Overview::new(o.clone(), "Overview".to_string()));
        let title = item.name.clone();
        let register = Some((item.id.clone(), children.iter().map(|i|i.id.clone()).collect()));
        let children = ItemListState::new(
            children
                .into_iter()
                .map(|i| EntryState::new(i, cx.as_mut())),
            title,
        );
        Self {
            children,
            item,
            overview,
            register,
        }
    }
}

impl JellyhajWidgetState for ItemListDisplayState {
    type Action = DisplayListAction;

    type ActionResult = Navigation;

    type Widget = ItemListDisplay;

    const NAME: &str = "item-list-details";

    fn visit_children(visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        visitor.visit::<ItemListState<EntryState>>();
        visitor.visit::<Overview>();
    }

    fn into_widget(self, cx: Pin<&mut jellyhaj_core::context::TuiContext>) -> Self::Widget {
        ItemListDisplay {
            children: self.children.into_widget(cx),
            item: self.item,
            overview: self.overview,
            register: self.register,
        }
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        match action {
            DisplayListAction::Inner(action) => self
                .children
                .apply_action(cx.wrap_with(DisplayListAction::Inner), action),
            DisplayListAction::Up => {
                if let Some(o) = self.overview.as_mut() {
                    JellyhajWidget::apply_action(
                        o,
                        cx.wrap_with(|_| unimplemented!()),
                        OverviewAction::Up,
                    )?;
                }
                Ok(None)
            }
            DisplayListAction::Down => {
                if let Some(o) = self.overview.as_mut() {
                    JellyhajWidget::apply_action(
                        o,
                        cx.wrap_with(|_| unimplemented!()),
                        OverviewAction::Down,
                    )?;
                }
                Ok(None)
            }
            DisplayListAction::Left => self
                .children
                .apply_action(cx.wrap_with(DisplayListAction::Inner), ItemListAction::Left),
            DisplayListAction::Right => self.children.apply_action(
                cx.wrap_with(DisplayListAction::Inner),
                ItemListAction::Right,
            ),
            DisplayListAction::Reload => Ok(Some(Navigation::Replace(Box::new(NextScreen::FetchItemListDetailsRef(self.item.id.clone()))))),
            DisplayListAction::Remove => Ok(Some(Navigation::PopContext))
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
            register: self.register
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
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        match action {
            DisplayListAction::Inner(action) => self
                .children
                .apply_action(cx.wrap_with(DisplayListAction::Inner), action),
            DisplayListAction::Up => {
                if let Some(o) = self.overview.as_mut() {
                    JellyhajWidget::apply_action(
                        o,
                        cx.wrap_with(|_| unimplemented!()),
                        OverviewAction::Up,
                    )?;
                }
                Ok(None)
            }
            DisplayListAction::Down => {
                if let Some(o) = self.overview.as_mut() {
                    JellyhajWidget::apply_action(
                        o,
                        cx.wrap_with(|_| unimplemented!()),
                        OverviewAction::Down,
                    )?;
                }
                Ok(None)
            }
            DisplayListAction::Left => self
                .children
                .apply_action(cx.wrap_with(DisplayListAction::Inner), ItemListAction::Left),
            DisplayListAction::Right => self.children.apply_action(
                cx.wrap_with(DisplayListAction::Inner),
                ItemListAction::Right,
            ),
            DisplayListAction::Reload => Ok(Some(Navigation::Replace(Box::new(NextScreen::FetchItemListDetailsRef(self.item.id.clone()))))),
            DisplayListAction::Remove => Ok(Some(Navigation::PopContext))
        }
    }

    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
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
                cx.wrap_with(DisplayListAction::Inner),
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
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
    ) -> jellyhaj_widgets_core::Result<()> {
        if let Some((parent, children)) = self.register.take(){
            let mut events = cx.jellyfin_events.get();
            for child in children{
                events.register_item_updated(child, cx.submitter.wrap_with(|_|DisplayListAction::Reload));
            }
            events.register_folder_modified(parent.clone(), cx.submitter.wrap_with(|_|DisplayListAction::Reload));
            events.register_item_updated(parent.clone(), cx.submitter.wrap_with(|_|DisplayListAction::Reload));
            events.register_item_removed(parent, cx.submitter.wrap_with(|_|DisplayListAction::Remove));
        }
        self.children.active = true;
        self.children.render_fallible(
            (
                (area.x + 2, area.y + 2).into(),
                (area.width - 4, self.children.height()).into(),
            )
                .into(),
            buf,
            cx.wrap_with(DisplayListAction::Inner),
        )?;
        if let Some(overview) = self.overview.as_mut() {
            overview.render_fallible(
                (
                    (area.x, area.y + 4 + self.children.height()).into(),
                    (area.width, area.height - 4 - self.children.height()).into(),
                )
                    .into(),
                buf,
                cx.wrap_with(|_| unreachable!()),
            )?
        }
        Block::bordered()
            .title(self.item.name.as_str())
            .merge_borders(MergeStrategy::Fuzzy)
            .render(area, buf);

        Ok(())
    }
}
