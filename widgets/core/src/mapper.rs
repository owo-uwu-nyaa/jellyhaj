use std::{fmt::Debug, marker::PhantomData};

use color_eyre::eyre::Result;
use jellyhaj_async_task::Wrapper;
use valuable::{Fields, NamedValues, StructDef, Structable, Valuable, Value, Visit};

use crate::{JellyhajWidget, WidgetContext};

pub trait Named: 'static {
    const NAME: &str;
}

pub trait ResultMapper<I> {
    type R: Debug + 'static;
    fn map(res: I) -> Result<Option<Self::R>>;
}

pub struct MapperWidget<N: Named, W: 'static, M: 'static> {
    pub inner: W,
    named: PhantomData<fn(N) -> ()>,
    mapper: PhantomData<fn(M) -> ()>,
}

impl<N: Named, W, M> MapperWidget<N, W, M> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            named: PhantomData,
            mapper: PhantomData,
        }
    }
}

impl<N: Named, W, M> Valuable for MapperWidget<N, W, M> {
    fn as_value(&self) -> Value<'_> {
        Value::Structable(self)
    }

    fn visit(&self, visit: &mut dyn Visit) {
        visit.visit_named_fields(&NamedValues::new(&[], &[]));
    }
}

impl<N: Named, W, M> Structable for MapperWidget<N, W, M> {
    fn definition(&self) -> StructDef<'_> {
        StructDef::new_static("MapperState", Fields::Named(&[]))
    }
}

impl<R: 'static, N: Named, W: JellyhajWidget<R>, M: ResultMapper<W::ActionResult>> JellyhajWidget<R>
    for MapperWidget<N, W, M>
{
    type Action = W::Action;

    type ActionResult = M::R;

    const NAME: &str = N::NAME;

    fn visit_children(&self, visitor: &mut impl crate::WidgetTreeVisitor) {
        visitor.visit::<R, W>(&self.inner);
    }

    fn min_width(&self) -> Option<u16> {
        self.inner.min_width()
    }

    fn min_height(&self) -> Option<u16> {
        self.inner.min_height()
    }

    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {
        self.inner.init(cx);
    }

    fn accepts_text_input(&self) -> bool {
        self.inner.accepts_text_input()
    }

    fn accept_char(&mut self, text: char) {
        self.inner.accept_char(text);
    }

    fn accept_text(&mut self, text: String) {
        self.inner.accept_text(text);
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        match self.inner.apply_action(cx, action)? {
            None => Ok(None),
            Some(v) => M::map(v),
        }
    }

    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        position: ratatui::prelude::Position,
        size: ratatui::prelude::Size,
        kind: ratatui::crossterm::event::MouseEventKind,
        modifier: ratatui::crossterm::event::KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        match self.inner.click(cx, position, size, kind, modifier)? {
            None => Ok(None),
            Some(v) => M::map(v),
        }
    }

    fn render_fallible_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()> {
        self.inner.render_fallible_inner(area, buf, cx)
    }
}
