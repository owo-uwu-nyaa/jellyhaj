use std::{cmp::min, convert::Infallible};

use jellyfin::{JellyfinClient, items::MediaItem};
use jellyhaj_core::{
    Config,
    context::{DB, JellyfinEventInterests, Spawner},
    state::{Navigation, NextScreen},
};
use jellyhaj_entry_widget::{Entry, EntryAction, ImageProtocolCache, Picker, Stats};
use jellyhaj_item_list::{ItemList, ItemListAction, new_item_list};
use jellyhaj_widgets_core::{
    ContextRef, GetFromContext, ItemWidget, ItemWidgetExt, JellyhajWidget, JellyhajWidgetExt, Rect,
    WidgetContext, WidgetTreeVisitor, Wrapper,
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
use valuable::Valuable;

#[derive(Debug)]
pub enum OverviewAction {
    Up,
    Down,
}

#[derive(Debug, Valuable)]
pub struct Overview {
    pub text: String,
    pub title: String,
    #[valuable(skip)]
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

impl<R: 'static> JellyhajWidget<R> for Overview {
    type Action = OverviewAction;

    type ActionResult = Infallible;

    const NAME: &str = "overview";

    fn visit_children(&self, _visitor: &mut impl WidgetTreeVisitor) {}

    fn init(&mut self, _cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {}

    fn min_width(&self) -> Option<u16> {
        Some(5)
    }

    fn min_height(&self) -> Option<u16> {
        Some(5)
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
        _: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
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
        _: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
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
        _: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
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

impl ItemDetails {
    pub fn new(
        item: Box<MediaItem>,
        cx: &(impl ContextRef<ImageProtocolCache> + ContextRef<Config> + ContextRef<Picker>),
    ) -> Self {
        let overview = item
            .overview
            .as_ref()
            .map(|o| Overview::new(o.clone(), "Overview".to_string()));
        Self {
            entry: Entry::new(*item, cx),
            overview,
        }
    }
}

#[derive(Valuable)]
pub struct ItemDetails {
    #[valuable(skip)]
    entry: Entry,
    #[valuable(skip)]
    overview: Option<Overview>,
}

impl<
    R: ContextRef<Spawner>
        + ContextRef<Config>
        + ContextRef<Picker>
        + ContextRef<Stats>
        + ContextRef<JellyfinClient>
        + ContextRef<JellyfinEventInterests>
        + ContextRef<DB>
        + 'static,
> JellyhajWidget<R> for ItemDetails
{
    type Action = DisplayAction;

    type ActionResult = Navigation;

    const NAME: &str = "item-details";

    fn visit_children(&self, visitor: &mut impl WidgetTreeVisitor) {
        visitor.visit_item::<R, _>(&self.entry);
        if let Some(o) = &self.overview {
            visitor.visit::<R, _>(o);
        }
    }

    fn min_width(&self) -> Option<u16> {
        Some(ItemWidget::<R>::dimensions(&self.entry).width + 4)
    }

    fn min_height(&self) -> Option<u16> {
        Some(ItemWidget::<R>::dimensions(&self.entry).height + 8)
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
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        match action {
            DisplayAction::Inner(action) => ItemWidget::<R>::item_apply_action(
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
            DisplayAction::Reload(id) => {
                Ok(Some(Navigation::Replace(NextScreen::FetchItemDetails(id))))
            }
            DisplayAction::Remove => Ok(Some(Navigation::PopContext)),
        }
    }

    fn click(
        &mut self,
        _: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        _: ratatui::prelude::Position,
        _: ratatui::prelude::Size,
        _: jellyhaj_widgets_core::MouseEventKind,
        _: jellyhaj_widgets_core::KeyModifiers,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        Ok(None)
    }

    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {
        let register = &self
            .entry
            .data()
            .item()
            .expect("should only be constructed with a media item")
            .id;

        JellyfinEventInterests::get_ref(cx.refs).with(|events| {
            events.register_item_updated(
                register.clone(),
                cx.submitter.wrap_with(DisplayAction::Reload),
            );
            events.register_item_removed(
                register.clone(),
                cx.submitter.wrap_with(|_| DisplayAction::Remove),
            );
        });
        self.entry.init(cx.wrap_with(DisplayAction::Inner));
        if let Some(overview) = &mut self.overview {
            overview.init(cx.wrap_with(|_| unreachable!()));
        }
    }

    fn render_fallible_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> jellyhaj_widgets_core::Result<()> {
        let entry_off = (area.width - ItemWidget::<R>::dimensions(&self.entry).width) / 2;
        self.entry.render_item(
            (
                (area.x + entry_off, area.y + 2).into(),
                ItemWidget::<R>::dimensions(&self.entry),
            )
                .into(),
            buf,
            cx.wrap_with(DisplayAction::Inner),
        )?;
        if let Some(overview) = self.overview.as_mut() {
            overview.render_fallible(
                (
                    (
                        area.x,
                        area.y + 4 + ItemWidget::<R>::dimensions(&self.entry).height,
                    )
                        .into(),
                    (
                        area.width,
                        area.height - 4 - ItemWidget::<R>::dimensions(&self.entry).height,
                    )
                        .into(),
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

#[derive(Valuable)]
pub struct ItemListDetails {
    #[valuable(skip)]
    children: ItemList<Entry>,
    item: MediaItem,
    #[valuable(skip)]
    overview: Option<Overview>,
}

impl ItemListDetails {
    pub fn new(
        children: Vec<MediaItem>,
        item: Box<MediaItem>,
        cx: &(
             impl ContextRef<ImageProtocolCache>
             + ContextRef<Spawner>
             + ContextRef<Config>
             + ContextRef<Picker>
             + ContextRef<Stats>
             + ContextRef<JellyfinClient>
             + ContextRef<JellyfinEventInterests>
             + ContextRef<DB>
             + 'static
         ),
    ) -> Self {
        let overview = item
            .overview
            .as_ref()
            .map(|o| Overview::new(o.clone(), "Overview".to_string()));
        let title = item.name.clone();
        let children = new_item_list(children.into_iter().map(|i| Entry::new(i, cx)), title, cx);
        Self {
            children,
            item: *item,
            overview,
        }
    }
}

impl<
    R: ContextRef<Spawner>
        + ContextRef<Config>
        + ContextRef<Picker>
        + ContextRef<Stats>
        + ContextRef<JellyfinClient>
        + ContextRef<JellyfinEventInterests>
        + ContextRef<DB>
        + 'static,
> JellyhajWidget<R> for ItemListDetails
{
    const NAME: &str = "item-list-details";

    type Action = DisplayListAction;

    type ActionResult = Navigation;

    fn visit_children(&self, visitor: &mut impl WidgetTreeVisitor) {
        visitor.visit::<R, _>(&self.children);
        if let Some(overview) = &self.overview {
            visitor.visit::<R, _>(overview);
        }
    }

    fn min_width(&self) -> Option<u16> {
        Some(9)
    }

    fn min_height(&self) -> Option<u16> {
        Some(12 + self.children.height())
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
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
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
            DisplayListAction::Reload => Ok(Some(Navigation::Replace(
                NextScreen::FetchItemListDetailsRef(self.item.id.clone()),
            ))),
            DisplayListAction::Remove => Ok(Some(Navigation::PopContext)),
        }
    }

    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
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

    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {
        let parent = &self.item.id;
        JellyfinEventInterests::get_ref(cx.refs).with(|events| {
            events.register_folder_modified(
                parent.clone(),
                cx.submitter.wrap_with(|_| DisplayListAction::Reload),
            );
            events.register_item_updated(
                parent.clone(),
                cx.submitter.wrap_with(|_| DisplayListAction::Reload),
            );
            events.register_item_removed(
                parent.clone(),
                cx.submitter.wrap_with(|_| DisplayListAction::Remove),
            );
            for child in self.children.iter().filter_map(|e| e.data().item()) {
                events.register_item_updated(
                    child.id.clone(),
                    cx.submitter.wrap_with(|_| DisplayListAction::Reload),
                );
            }
        });
        self.children.init(cx.wrap_with(DisplayListAction::Inner));
        if let Some(overview) = &mut self.overview {
            overview.init(cx.wrap_with(|_| unreachable!()));
        }
    }

    fn render_fallible_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> jellyhaj_widgets_core::Result<()> {
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
