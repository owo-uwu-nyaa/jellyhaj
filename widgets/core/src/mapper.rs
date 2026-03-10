use std::{fmt::Debug, marker::PhantomData, pin::Pin};

use color_eyre::eyre::Result;
use jellyhaj_async_task::Wrapper;
use jellyhaj_context::TuiContext;

use crate::{JellyhajWidget, JellyhajWidgetState, WidgetContext};

pub trait Named: 'static {
    const NAME: &str;
}

pub trait ResultMapper<S: JellyhajWidgetState>: Default + Send + 'static {
    type R: Debug + 'static;
    fn map_widget(this: &mut S::Widget, res: S::ActionResult) -> Result<Option<Self::R>>;
    fn map_state(this: &mut S, res: S::ActionResult) -> Result<Option<Self::R>>;
}

pub struct MapperWidget<N: Named, W: JellyhajWidget, M: ResultMapper<W::State>> {
    pub inner: W,
    named: PhantomData<fn(N) -> N>,
    mapper: M,
}

impl<N: Named, W: JellyhajWidget, M: ResultMapper<W::State>> MapperWidget<N, W, M> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            named: PhantomData,
            mapper: Default::default(),
        }
    }
}

pub struct MapperState<N: Named, S: JellyhajWidgetState, M: ResultMapper<S>> {
    pub inner: S,
    named: PhantomData<fn(N) -> N>,
    mapper: M,
}

impl<N: Named, S: JellyhajWidgetState, M: ResultMapper<S>> MapperState<N, S, M> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            named: PhantomData,
            mapper: Default::default(),
        }
    }
}

impl<N: Named, S: JellyhajWidgetState, M: ResultMapper<S>> Debug for MapperState<N, S, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OuterState")
            .field("inner", &self.inner)
            .field("named", &N::NAME)
            .finish()
    }
}

impl<N: Named, S: JellyhajWidgetState, M: ResultMapper<S>> JellyhajWidgetState
    for MapperState<N, S, M>
{
    type Action = S::Action;

    type ActionResult = M::R;

    type Widget = MapperWidget<N, S::Widget, M>;

    const NAME: &str = N::NAME;

    fn into_widget(self, cx: Pin<&mut TuiContext>) -> Self::Widget {
        MapperWidget {
            inner: self.inner.into_widget(cx),
            named: self.named,
            mapper: self.mapper,
        }
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        match self.inner.apply_action(cx, action)? {
            None => Ok(None),
            Some(v) => M::map_state(&mut self.inner, v),
        }
    }

    fn visit_children(visitor: &mut impl crate::WidgetTreeVisitor) {
        visitor.visit::<S>();
    }
}

impl<N: Named, W: JellyhajWidget, M: ResultMapper<W::State>> JellyhajWidget
    for MapperWidget<N, W, M>
{
    type Action = W::Action;

    type ActionResult = M::R;

    type State = MapperState<N, W::State, M>;

    fn min_width(&self) -> Option<u16> {
        self.inner.min_width()
    }

    fn min_height(&self) -> Option<u16> {
        self.inner.min_height()
    }

    fn into_state(self) -> Self::State {
        MapperState {
            inner: self.inner.into_state(),
            named: self.named,
            mapper: self.mapper,
        }
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
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        match self.inner.apply_action(cx, action)? {
            None => Ok(None),
            Some(v) => M::map_widget(&mut self.inner, v),
        }
    }

    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
        position: ratatui::prelude::Position,
        size: ratatui::prelude::Size,
        kind: ratatui::crossterm::event::MouseEventKind,
        modifier: ratatui::crossterm::event::KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        match self.inner.click(cx, position, size, kind, modifier)? {
            None => Ok(None),
            Some(v) => M::map_widget(&mut self.inner, v),
        }
    }

    fn render_fallible_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
    ) -> Result<()> {
        self.inner.render_fallible_inner(area, buf, cx)
    }
}
