use std::cmp::min;

use jellyfin::JellyfinClient;
use jellyhaj_core::{
    Config,
    context::{DB, JellyfinEventInterests, Spawner},
    state::{Navigation, NextScreen},
};
use jellyhaj_entry_widget::{Entry, EntryAction, ImageCache};
use jellyhaj_image::{Picker, Stats};
use jellyhaj_widgets_core::{
    ContextRef, GetFromContext, ItemWidget, ItemWidgetExt, JellyhajWidget, JellyhajWidgetBase,
    JellyhajWidgetExt, KeyModifiers, MouseEventKind, Result, WidgetContext, Wrapper,
};
use ratatui::{
    prelude::{Buffer, Position, Rect, Size},
    symbols::merge::MergeStrategy,
    widgets::{Block, Padding, Widget},
};
use valuable::Valuable;

use crate::overview::{Overview, OverviewAction};

#[derive(Valuable)]
pub struct Child {
    #[valuable(skip)]
    pub entry: Entry,
    #[valuable(skip)]
    pub overview: Option<Overview<&'static str>>,
}

#[derive(Valuable)]
pub struct ItemChilds {
    pub id: String,
    pub current: usize,
    pub offset: usize,
    pub items: Vec<Child>,
}

#[derive(Debug)]
pub enum ChildAction {
    Up,
    Down,
    ScrollUp,
    ScrollDown,
    CurrentEntry(EntryAction),
    Entry {
        index: usize,
        action: EntryAction,
    },
    Overview {
        index: usize,
        action: OverviewAction,
    },
    Reload,
    Remove,
}

impl JellyhajWidgetBase for ItemChilds {
    type Action = ChildAction;

    type ActionResult = Navigation;

    const NAME: &str = "item-childs";

    fn visit_children(&self, visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        for child in &self.items {
            visitor.visit(&child.entry);
            if let Some(overview) = &child.overview {
                visitor.visit(overview);
            }
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
        + ContextRef<ImageCache>
        + 'static,
> JellyhajWidget<R> for ItemChilds
{
    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {
        let parent = &self.id;
        JellyfinEventInterests::get_ref(cx.refs).with(|events| {
            events.register_folder_modified(
                parent.clone(),
                cx.submitter.wrap_with(|_| ChildAction::Reload),
            );
            events.register_item_updated(
                parent.clone(),
                cx.submitter.wrap_with(|_| ChildAction::Reload),
            );
            events.register_item_removed(
                parent.clone(),
                cx.submitter.wrap_with(|_| ChildAction::Remove),
            );
            for child in self.items.iter().filter_map(|e| e.entry.data().item()) {
                events.register_item_updated(
                    child.id.clone(),
                    cx.submitter.wrap_with(|_| ChildAction::Reload),
                );
            }
        });
        for (index, child) in self.items.iter_mut().enumerate() {
            child
                .entry
                .init(cx.wrap_with(move |action| ChildAction::Entry { index, action }));
            if let Some(overview) = child.overview.as_mut() {
                overview.init(cx.wrap_with(move |action| ChildAction::Overview { index, action }));
            }
        }
    }

    fn accepts_text_input(&self) -> bool {
        false
    }
    fn accept_char(&mut self, _text: char) {
        unimplemented!()
    }
    fn accept_text(&mut self, _text: String) {
        unimplemented!()
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        match action {
            ChildAction::Up => {
                self.current = min(self.current + 1, self.items.len().saturating_sub(1));
                Ok(None)
            }
            ChildAction::Down => {
                self.current = self.current.saturating_sub(1);
                Ok(None)
            }
            ChildAction::ScrollUp => {
                let current = self.current;
                if let Some(overview) = self
                    .items
                    .get_mut(current)
                    .and_then(|c| c.overview.as_mut())
                {
                    overview.apply_action(
                        cx.wrap_with(move |action| ChildAction::Overview {
                            index: current,
                            action,
                        }),
                        OverviewAction::Up,
                    )?;
                }
                Ok(None)
            }
            ChildAction::ScrollDown => {
                let current = self.current;
                if let Some(overview) = self
                    .items
                    .get_mut(current)
                    .and_then(|c| c.overview.as_mut())
                {
                    overview.apply_action(
                        cx.wrap_with(move |action| ChildAction::Overview {
                            index: current,
                            action,
                        }),
                        OverviewAction::Down,
                    )?;
                }
                Ok(None)
            }
            ChildAction::Entry { index, action } => {
                if let Some(child) = self.items.get_mut(index) {
                    child.entry.item_apply_action(
                        cx.wrap_with(move |action| ChildAction::Entry { index, action }),
                        action,
                    )
                } else {
                    Ok(None)
                }
            }
            ChildAction::CurrentEntry(action) => {
                let index = self.current;
                if let Some(child) = self.items.get_mut(index) {
                    child.entry.item_apply_action(
                        cx.wrap_with(move |action| ChildAction::Entry { index, action }),
                        action,
                    )
                } else {
                    Ok(None)
                }
            }
            ChildAction::Overview { index, action } => {
                if let Some(overview) = self.items.get_mut(index).and_then(|v| v.overview.as_mut())
                {
                    overview.apply_action(
                        cx.wrap_with(move |action| ChildAction::Overview { index, action }),
                        action,
                    )?;
                }
                Ok(None)
            }
            ChildAction::Reload => Ok(Some(Navigation::Replace(
                NextScreen::FetchItemListDetailsRef(self.id.clone()),
            ))),
            ChildAction::Remove => Ok(Some(Navigation::PopContext)),
        }
    }

    fn min_width(&self) -> Option<u16> {
        self.items
            .first()
            .map(|first| ItemWidget::<R>::dimensions(&first.entry).width + 4 + 5 + 4)
    }
    fn min_height(&self) -> Option<u16> {
        self.items
            .first()
            .map(|first| ItemWidget::<R>::dimensions(&first.entry).width + 4 + 4)
    }

    fn render_fallible_inner(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()> {
        let outer = Block::bordered().padding(Padding::uniform(1));
        if let Some(dim) = self
            .items
            .first()
            .map(|c| ItemWidget::<R>::dimensions(&c.entry))
        {
            let height = dim.height + 4;
            let main = outer.inner(area);
            let visible = u16::try_from(min(
                self.items.len(),
                main.height
                    .strict_sub(1)
                    .strict_div(height.strict_sub(1))
                    .into(),
            ))
            .expect("bounded by area height");
            self.offset = if (visible as usize) < self.items.len()
                && let position_in_visible = visible / 2
                && self.current > position_in_visible as usize
            {
                min(
                    self.current - position_in_visible as usize,
                    self.items.len() - visible as usize,
                )
            } else {
                0
            };
            for ((i, child), y) in self
                .items
                .iter_mut()
                .enumerate()
                .skip(self.offset)
                .zip((0..visible as u16).map(|i| main.y + i * (height.strict_sub(1))))
            {
                ItemWidget::<R>::set_active(&mut child.entry, i == self.current);
                let area = Rect {
                    x: main.x,
                    y,
                    width: main.width,
                    height,
                };
                let outer = Block::bordered()
                    .merge_borders(MergeStrategy::Exact)
                    .padding(Padding::uniform(1));
                let mut main = outer.inner(area);
                main.width = dim.width;
                child.entry.render_item(
                    main,
                    buf,
                    cx.wrap_with(move |action| ChildAction::Entry { index: i, action }),
                )?;
                let offset = dim.width + 3;
                let mut descr_area = area;
                descr_area.x += offset;
                descr_area.width -= offset;
                if let Some(overview) = &mut child.overview {
                    overview.render_fallible(
                        descr_area,
                        buf,
                        cx.wrap_with(move |action| ChildAction::Overview { index: i, action }),
                    )?;
                } else {
                    Block::bordered()
                        .merge_borders(MergeStrategy::Exact)
                        .render(descr_area, buf);
                }
                outer.render(area, buf);
            }
        }
        Ok(())
    }
    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        mut position: Position,
        size: Size,
        kind: MouseEventKind,
        modifier: KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        if let Some(dim) = self
            .items
            .first()
            .map(|c| ItemWidget::<R>::dimensions(&c.entry))
            && position.x > 1
            && position.y > 1
            && position.x + 2 < size.width
            && position.y + 2 < size.height
        {
            position.x -= 2;
            position.y -= 2;
            let index = position.y / (dim.height + 3);
            position.y %= dim.height + 3;
            if let Some((index, child)) = self
                .items
                .iter_mut()
                .enumerate()
                .nth(self.offset + (index as usize))
                && position.y > 0
            {
                if let MouseEventKind::Down(_) = kind {
                    self.current = index;
                }
                if position.x > 1
                    && position.y > 1
                    && position.x < dim.width + 2
                    && position.y < dim.height + 2
                {
                    position.x -= 2;
                    position.y -= 2;
                    return child.entry.item_click(
                        cx.wrap_with(move |action| ChildAction::Entry { index, action }),
                        position,
                        dim,
                        kind,
                        modifier,
                    );
                }
            }
        }

        Ok(None)
    }
}
