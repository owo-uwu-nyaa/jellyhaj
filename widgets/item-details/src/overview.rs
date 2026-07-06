use std::{cmp::min, convert::Infallible};

use jellyhaj_widgets_core::{
    JellyhajWidget, JellyhajWidgetBase, Rect, WidgetContext, WidgetTreeVisitor, Wrapper,
};
use ratatui::{
    layout::Margin,
    symbols::merge::MergeStrategy,
    widgets::{
        Block, Padding, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget,
        WidgetRef,
    },
};
use valuable::Valuable;

#[derive(Debug)]
pub enum OverviewAction {
    Up,
    Down,
}

#[derive(Debug, Valuable)]
pub struct Overview<T: AsRef<str> + Valuable> {
    text: String,
    title: T,
    #[valuable(skip)]
    scroll: usize,
    computed: Option<(u16, Vec<String>)>,
}

impl<T: AsRef<str> + Valuable> Overview<T> {
    #[must_use]
    pub const fn new(text: String, title: T) -> Self {
        Self {
            text,
            title,
            scroll: 0,
            computed: None,
        }
    }
}

impl<T: AsRef<str> + Valuable + Send + 'static> JellyhajWidgetBase for Overview<T> {
    type Action = OverviewAction;

    type ActionResult = Infallible;

    const NAME: &str = "overview";

    fn visit_children(&self, _visitor: &mut impl WidgetTreeVisitor) {}
}

impl<R: 'static, T: AsRef<str> + Valuable + Send + 'static> JellyhajWidget<R> for Overview<T> {
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
            .merge_borders(MergeStrategy::Exact)
            .title(self.title.as_ref())
            .padding(Padding::uniform(1));
        let main = outer.inner(area);

        let lines: &[_] = if let Some((width, lines)) = self.computed.as_ref()
            && *width == main.width
        {
            lines
        } else {
            let lines = textwrap::wrap(&self.text, main.width as usize)
                .into_iter()
                .map(std::borrow::Cow::into_owned)
                .collect();
            &self.computed.insert((main.width, lines)).1
        };
        self.scroll = min(self.scroll, lines.len().saturating_sub(main.height as usize));
        for (i, y) in (main.y..(main.y + main.height)).into_iter().take(lines.len()).enumerate() {
            let i = i + self.scroll;
            lines[i].render_ref(
                Rect {
                    x: main.x,
                    y,
                    width: main.width,
                    height: 1,
                },
                buf,
            );
        }
        Scrollbar::new(ScrollbarOrientation::VerticalRight).render(
            area.inner(Margin {
                horizontal: 0,
                vertical: 2,
            }),
            buf,
            &mut ScrollbarState::new(lines.len()).position(self.scroll),
        );
        outer.render(area, buf);
        Ok(())
    }
}
