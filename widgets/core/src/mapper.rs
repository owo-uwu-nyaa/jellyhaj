use std::{fmt::Debug, marker::PhantomData};

use color_eyre::eyre::Result;
use jellyhaj_async_task::Wrapper;

use crate::{JellyhajWidget, JellyhajWidgetState, WidgetContext};

pub trait Named: 'static {
    const NAME: &str;
}

pub trait ResultMapper<CX: 'static, S: JellyhajWidgetState<CX>>: Default + Send + 'static {
    type R: Debug + 'static;
    fn map_widget(this: &mut S::Widget, res: S::ActionResult) -> Result<Option<Self::R>>;
    fn map_state(this: &mut S, res: S::ActionResult) -> Result<Option<Self::R>>;
}

pub struct MapperWidget<CX: 'static, N: Named, W: JellyhajWidget<CX>, M: ResultMapper<CX, W::State>>
{
    pub inner: W,
    named: PhantomData<fn(N) -> N>,
    cx: PhantomData<fn(CX) -> ()>,
    mapper: M,
}

impl<R, N: Named, W: JellyhajWidget<R>, M: ResultMapper<R, W::State>> MapperWidget<R, N, W, M> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            named: PhantomData,
            mapper: Default::default(),
            cx: PhantomData,
        }
    }
}

pub struct MapperState<R: 'static, N: Named, S: JellyhajWidgetState<R>, M: ResultMapper<R, S>> {
    pub inner: S,
    named: PhantomData<fn(N) -> N>,
    r: PhantomData<fn(R) -> ()>,
    mapper: M,
}

impl<R: 'static, N: Named, S: JellyhajWidgetState<R>, M: ResultMapper<R, S>>
    MapperState<R, N, S, M>
{
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            named: PhantomData,
            mapper: Default::default(),
            r: PhantomData,
        }
    }
}

impl<R, N: Named, S: JellyhajWidgetState<R>, M: ResultMapper<R, S>> Debug
    for MapperState<R, N, S, M>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OuterState")
            .field("inner", &self.inner)
            .field("named", &N::NAME)
            .finish()
    }
}

impl<R, N: Named, S: JellyhajWidgetState<R>, M: ResultMapper<R, S>> JellyhajWidgetState<R>
    for MapperState<R, N, S, M>
{
    type Action = S::Action;

    type ActionResult = M::R;

    type Widget = MapperWidget<R, N, S::Widget, M>;

    const NAME: &str = N::NAME;

    fn into_widget(self, cx: &R) -> Self::Widget {
        MapperWidget {
            inner: self.inner.into_widget(cx),
            named: self.named,
            mapper: self.mapper,
            cx: PhantomData,
        }
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        match self.inner.apply_action(cx, action)? {
            None => Ok(None),
            Some(v) => M::map_state(&mut self.inner, v),
        }
    }

    fn visit_children(visitor: &mut impl crate::WidgetTreeVisitor) {
        visitor.visit::<R, S>();
    }
}

impl<R, N: Named, W: JellyhajWidget<R>, M: ResultMapper<R, W::State>> JellyhajWidget<R>
    for MapperWidget<R, N, W, M>
{
    type Action = W::Action;

    type ActionResult = M::R;

    type State = MapperState<R, N, W::State, M>;

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
            r: PhantomData,
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
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        match self.inner.apply_action(cx, action)? {
            None => Ok(None),
            Some(v) => M::map_widget(&mut self.inner, v),
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
            Some(v) => M::map_widget(&mut self.inner, v),
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
