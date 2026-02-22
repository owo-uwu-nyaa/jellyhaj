use std::{fmt::Debug, marker::PhantomData, ops::ControlFlow};

use color_eyre::eyre::Result;

use crate::{JellyhajWidget, JellyhajWidgetState};

pub trait Named: 'static {
    const NAME: &str;
}

pub struct Outer<N: Named, T: Debug + 'static, W: JellyhajWidget<ActionResult = ControlFlow<T, T>>>
{
    pub inner: W,
    named: PhantomData<fn(N) -> N>,
}

impl<N: Named, T: Debug + 'static, W: JellyhajWidget<ActionResult = ControlFlow<T, T>>>
    Outer<N, T, W>
{
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            named: PhantomData,
        }
    }
}

pub struct OuterState<
    N: Named,
    T: Debug + 'static,
    S: JellyhajWidgetState<ActionResult = ControlFlow<T, T>>,
> {
    pub inner: S,
    named: PhantomData<fn(N) -> N>,
}

impl<N: Named, T: Debug + 'static, S: JellyhajWidgetState<ActionResult = ControlFlow<T, T>>>
    OuterState<N, T, S>
{
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            named: PhantomData,
        }
    }
}

fn map_cf<T>(cf: Result<Option<ControlFlow<T, T>>>) -> Result<Option<T>> {
    match cf {
        Err(e) => Err(e),
        Ok(None) => Ok(None),
        Ok(Some(ControlFlow::Continue(v))) => Ok(Some(v)),
        Ok(Some(ControlFlow::Break(v))) => Ok(Some(v)),
    }
}

impl<N: Named, T: Debug + 'static, S: JellyhajWidgetState<ActionResult = ControlFlow<T, T>> + Debug>
    Debug for OuterState<N, T, S>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OuterState")
            .field("inner", &self.inner)
            .field("named", &N::NAME)
            .finish()
    }
}

impl<N: Named, T: Debug + 'static, S: JellyhajWidgetState<ActionResult = ControlFlow<T, T>>>
    JellyhajWidgetState for OuterState<N, T, S>
{
    type Action = S::Action;

    type ActionResult = T;

    type Widget = Outer<N, T, S::Widget>;

    const NAME: &str = N::NAME;

    fn into_widget(self, cx: std::pin::Pin<&mut jellyhaj_context::TuiContext>) -> Self::Widget {
        Outer {
            inner: self.inner.into_widget(cx),
            named: self.named,
        }
    }

    fn apply_action(
        &mut self,
        task: crate::async_task::TaskSubmitter<Self::Action, impl crate::Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        map_cf(self.inner.apply_action(task, action))
    }

    fn visit_children(visitor: &mut impl crate::WidgetTreeVisitor) {
        visitor.visit::<S>();
    }
}

impl<N: Named, T: Debug + 'static, W: JellyhajWidget<ActionResult = ControlFlow<T, T>>>
    JellyhajWidget for Outer<N, T, W>
{
    type Action = W::Action;

    type ActionResult = T;

    type State = OuterState<N, T, W::State>;

    fn min_width(&self) -> Option<u16> {
        self.inner.min_width()
    }

    fn min_height(&self) -> Option<u16> {
        self.inner.min_height()
    }

    fn into_state(self) -> Self::State {
        OuterState {
            inner: self.inner.into_state(),
            named: self.named,
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
        task: crate::async_task::TaskSubmitter<Self::Action, impl crate::Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        map_cf(self.inner.apply_action(task, action))
    }

    fn click(
        &mut self,
        task: crate::async_task::TaskSubmitter<Self::Action, impl crate::Wrapper<Self::Action>>,
        position: ratatui::prelude::Position,
        size: ratatui::prelude::Size,
        kind: ratatui::crossterm::event::MouseEventKind,
        modifier: ratatui::crossterm::event::KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        map_cf(self.inner.click(task, position, size, kind, modifier))
    }

    fn render_fallible_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        task: crate::async_task::TaskSubmitter<Self::Action, impl crate::Wrapper<Self::Action>>,
    ) -> Result<()> {
        self.inner.render_fallible_inner(area, buf, task)
    }
}
