use std::{convert::Infallible, fmt::Debug};

use jellyhaj_core::state::{Navigation, Next};
use jellyhaj_widgets_core::{JellyhajWidget, JellyhajWidgetState, Result};
use tracing::info_span;

#[derive(Debug)]
pub enum FetchAction<T: Debug> {
    Inner(T),
    FetchFinished(Next),
}

pub struct FetchState<
    S: JellyhajWidgetState<ActionResult = Infallible>,
    F: Future<Output = Result<Next>> + Send + 'static,
> {
    fut: Option<F>,
    inner: S,
}

impl<
    S: JellyhajWidgetState<ActionResult = Infallible>,
    F: Future<Output = Result<Next>> + Send + 'static,
> FetchState<S, F>
{
    pub fn new(fut: F, inner: S) -> Self {
        Self {
            fut: Some(fut),
            inner,
        }
    }
}

impl<
    S: JellyhajWidgetState<ActionResult = Infallible>,
    F: Future<Output = Result<Next>> + Send + 'static,
> std::fmt::Debug for FetchState<S, F>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FetchState")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<
    S: JellyhajWidgetState<ActionResult = Infallible>,
    F: Future<Output = Result<Next>> + Send + 'static,
> JellyhajWidgetState for FetchState<S, F>
{
    type Action = FetchAction<S::Action>;

    type ActionResult = Navigation;

    type Widget = FetchWidget<S::Widget, F>;

    const NAME: &str = "fetch";

    fn visit_children(visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        visitor.visit::<S>();
    }

    fn into_widget(
        self,
        cx: std::pin::Pin<&mut jellyhaj_core::context::TuiContext>,
    ) -> Self::Widget {
        FetchWidget {
            fut: self.fut,
            inner: self.inner.into_widget(cx),
        }
    }

    fn apply_action(
        &mut self,
        task: jellyhaj_widgets_core::async_task::TaskSubmitter<
            Self::Action,
            impl jellyhaj_widgets_core::Wrapper<Self::Action>,
        >,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        match action {
            FetchAction::Inner(a) => {
                let None = self
                    .inner
                    .apply_action(task.wrap_with(FetchAction::Inner), a)?;
                Ok(None)
            }
            FetchAction::FetchFinished(next_screen) => Ok(Some(Navigation::Replace(next_screen))),
        }
    }
}

pub struct FetchWidget<
    W: JellyhajWidget<ActionResult = Infallible>,
    F: Future<Output = Result<Next>> + Send + 'static,
> {
    fut: Option<F>,
    inner: W,
}

impl<
    W: JellyhajWidget<ActionResult = Infallible>,
    F: Future<Output = Result<Next>> + Send + 'static,
> JellyhajWidget for FetchWidget<W, F>
{
    type Action = FetchAction<W::Action>;

    type ActionResult = Navigation;

    type State = FetchState<W::State, F>;

    fn min_width(&self) -> Option<u16> {
        self.inner.min_width()
    }

    fn min_height(&self) -> Option<u16> {
        self.inner.min_height()
    }

    fn into_state(self) -> Self::State {
        FetchState {
            fut: self.fut,
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
        task: jellyhaj_widgets_core::async_task::TaskSubmitter<
            Self::Action,
            impl jellyhaj_widgets_core::Wrapper<Self::Action>,
        >,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        match action {
            FetchAction::Inner(action) => {
                let None = self
                    .inner
                    .apply_action(task.wrap_with(FetchAction::Inner), action)?;
                Ok(None)
            }
            FetchAction::FetchFinished(next_screen) => Ok(Some(Navigation::Replace(next_screen))),
        }
    }

    fn click(
        &mut self,
        task: jellyhaj_widgets_core::async_task::TaskSubmitter<
            Self::Action,
            impl jellyhaj_widgets_core::Wrapper<Self::Action>,
        >,
        position: jellyhaj_widgets_core::Position,
        size: jellyhaj_widgets_core::Size,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        let None = self.inner.click(
            task.wrap_with(FetchAction::Inner),
            position,
            size,
            kind,
            modifier,
        )?;
        Ok(None)
    }

    fn render_fallible_inner(
        &mut self,
        area: jellyhaj_widgets_core::Rect,
        buf: &mut jellyhaj_widgets_core::Buffer,
        task: jellyhaj_widgets_core::async_task::TaskSubmitter<
            Self::Action,
            impl jellyhaj_widgets_core::Wrapper<Self::Action>,
        >,
    ) -> Result<()> {
        if self.fut.is_some() {
            let fut = self.fut.take().expect("just checked");
            task.spawn_task(
                async move { Ok(FetchAction::FetchFinished(fut.await?)) },
                info_span!("do_fetch"),
            )
        }
        self.inner
            .render_fallible_inner(area, buf, task.wrap_with(FetchAction::Inner))
    }
}
