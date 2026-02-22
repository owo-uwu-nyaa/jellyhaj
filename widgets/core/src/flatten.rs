use std::{fmt::Debug, ops::ControlFlow};

use crate::{JellyhajWidget, JellyhajWidgetState};

fn map<A, B>(v: ControlFlow<A, ControlFlow<A, B>>) -> ControlFlow<A, B> {
    v?
}

pub struct FlattenState<
    A,
    B,
    S: JellyhajWidgetState<ActionResult = ControlFlow<A, ControlFlow<A, B>>>,
> {
    pub inner: S,
}

impl<A, B, S: JellyhajWidgetState<ActionResult = ControlFlow<A, ControlFlow<A, B>>> + Debug> Debug
    for FlattenState<A, B, S>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlattenState")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<A, B, S: JellyhajWidgetState<ActionResult = ControlFlow<A, ControlFlow<A, B>>>>
    FlattenState<A, B, S>
{
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}
impl<
    A: Debug + 'static,
    B: Debug + 'static,
    S: JellyhajWidgetState<ActionResult = ControlFlow<A, ControlFlow<A, B>>>,
> JellyhajWidgetState for FlattenState<A, B, S>
{
    type Action = S::Action;

    type ActionResult = ControlFlow<A, B>;

    type Widget = Flatten<A, B, S::Widget>;

    const NAME: &str = "flatten";

    fn visit_children(visitor: &mut impl crate::WidgetTreeVisitor) {
        visitor.visit::<S>();
    }

    fn into_widget(self, cx: std::pin::Pin<&mut jellyhaj_context::TuiContext>) -> Self::Widget {
        Flatten {
            inner: self.inner.into_widget(cx),
        }
    }

    fn apply_action(
        &mut self,
        task: crate::async_task::TaskSubmitter<Self::Action, impl crate::Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> color_eyre::eyre::Result<Option<Self::ActionResult>> {
        S::apply_action(&mut self.inner, task, action).map(|v| v.map(map))
    }
}

pub struct Flatten<A, B, W: JellyhajWidget<ActionResult = ControlFlow<A, ControlFlow<A, B>>>> {
    pub inner: W,
}

impl<A, B, W: JellyhajWidget<ActionResult = ControlFlow<A, ControlFlow<A, B>>>> Flatten<A, B, W> {
    pub fn new(inner: W) -> Self {
        Self { inner }
    }
}

impl<
    A: Debug + 'static,
    B: Debug + 'static,
    W: JellyhajWidget<ActionResult = ControlFlow<A, ControlFlow<A, B>>>,
> JellyhajWidget for Flatten<A, B, W>
{
    type Action = W::Action;

    type ActionResult = ControlFlow<A, B>;

    type State = FlattenState<A, B, W::State>;

    fn min_width(&self) -> Option<u16> {
        self.inner.min_width()
    }

    fn min_height(&self) -> Option<u16> {
        self.inner.min_height()
    }

    fn into_state(self) -> Self::State {
        FlattenState {
            inner: self.inner.into_state(),
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
    ) -> color_eyre::eyre::Result<Option<Self::ActionResult>> {
        self.inner.apply_action(task, action).map(|v| v.map(map))
    }

    fn click(
        &mut self,
        task: crate::async_task::TaskSubmitter<Self::Action, impl crate::Wrapper<Self::Action>>,
        position: ratatui::prelude::Position,
        size: ratatui::prelude::Size,
        kind: ratatui::crossterm::event::MouseEventKind,
        modifier: ratatui::crossterm::event::KeyModifiers,
    ) -> color_eyre::eyre::Result<Option<Self::ActionResult>> {
        self.inner
            .click(task, position, size, kind, modifier)
            .map(|v| v.map(map))
    }

    fn render_fallible_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        task: crate::async_task::TaskSubmitter<Self::Action, impl crate::Wrapper<Self::Action>>,
    ) -> color_eyre::eyre::Result<()> {
        self.inner.render_fallible_inner(area, buf, task)
    }
}
