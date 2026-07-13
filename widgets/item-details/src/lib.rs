pub mod children;
pub mod item_list_details;
pub mod overview;

use jellyfin::{JellyfinClient, items::MediaItem};
use jellyhaj_core::{
    Config,
    context::{DB, JellyfinEventInterests, Spawner},
    state::{Navigation, NextScreen},
};
use jellyhaj_entry_widget::{Entry, EntryAction, ImageCache, Picker, Stats};
use jellyhaj_widgets_core::{
    ContextRef, GetFromContext, ItemWidget, ItemWidgetBase, ItemWidgetExt, JellyhajWidget,
    JellyhajWidgetBase, JellyhajWidgetExt, WidgetContext, WidgetTreeVisitor, Wrapper,
};
use ratatui::{
    symbols::merge::MergeStrategy,
    widgets::{Block, Widget},
};
use valuable::Valuable;

use crate::overview::{Overview, OverviewAction};

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
        cx: &(impl ContextRef<ImageCache> + ContextRef<Config> + ContextRef<Picker>),
    ) -> Self {
        let overview = item.overview.as_ref().map(|o| Overview::new(o.clone(), ""));
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
    overview: Option<Overview<&'static str>>,
}

impl JellyhajWidgetBase for ItemDetails {
    type Action = DisplayAction;

    type ActionResult = Navigation;

    const NAME: &str = "item-details";

    fn visit_children(&self, visitor: &mut impl WidgetTreeVisitor) {
        visitor.visit(&self.entry);
        if let Some(o) = &self.overview {
            visitor.visit(o);
        }
    }

    fn min_width(&self) -> Option<u16> {
        Some(self.entry.dimensions().width + 4)
    }

    fn min_height(&self) -> Option<u16> {
        Some(self.entry.dimensions().height + 8)
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
        + ContextRef<ImageCache>
        + 'static,
> JellyhajWidget<R> for ItemDetails
{
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
        let entry_off = (area.width - self.entry.dimensions().width) / 2;
        self.entry.render_item(
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
            )?;
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
