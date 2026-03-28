use std::{borrow::Cow, fmt::Debug, time::Duration};

use jellyhaj_core::{
    Config,
    state::{Navigation, NextScreen},
};
use jellyhaj_loading_widget::{AdvanceLoadingScreen, Loading, LoadingState};
use jellyhaj_widgets_core::{
    ContextRef, GetFromContext, JellyhajWidget, JellyhajWidgetState, Result, WidgetContext, Wrapper,
};
use tracing::info_span;

#[derive(Debug)]
pub enum FetchAction {
    Inner(AdvanceLoadingScreen),
    FetchFinished(NextScreen),
    FetchTimeout,
}

pub struct FetchState<F: Future<Output = Result<NextScreen>> + Send + 'static> {
    fut: Option<F>,
    inner: LoadingState,
}

impl<F: Future<Output = Result<NextScreen>> + Send + 'static> FetchState<F> {
    pub fn new(fut: F, title: Cow<'static, str>) -> Self {
        Self {
            fut: Some(fut),
            inner: LoadingState::new(title),
        }
    }
}

impl<F: Future<Output = Result<NextScreen>> + Send + 'static> Debug for FetchState<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FetchState")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<R: ContextRef<Config> + 'static, F: Future<Output = Result<NextScreen>> + Send + 'static>
    JellyhajWidgetState<R> for FetchState<F>
{
    type Action = FetchAction;

    type ActionResult = Navigation;

    type Widget = FetchWidget<F>;

    const NAME: &str = "fetch";

    fn visit_children(visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        visitor.visit::<R, LoadingState>();
    }

    fn into_widget(self, cx: &R) -> Self::Widget {
        FetchWidget {
            fut: self.fut,
            inner: self.inner.into_widget(cx),
        }
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        match action {
            FetchAction::Inner(a) => {
                let None = self
                    .inner
                    .apply_action(cx.wrap_with(FetchAction::Inner), a)?;
                Ok(None)
            }
            FetchAction::FetchFinished(next_screen) => Ok(Some(Navigation::Replace(next_screen))),
            FetchAction::FetchTimeout => Ok(Some(Navigation::Replace(NextScreen::Error(
                color_eyre::eyre::eyre!("fetch timeout reached.\n\n(this can be configured)"),
            )))),
        }
    }
}

pub struct FetchWidget<F: Future<Output = Result<NextScreen>> + Send + 'static> {
    fut: Option<F>,
    inner: Loading,
}

impl<R: ContextRef<Config> + 'static, F: Future<Output = Result<NextScreen>> + Send + 'static>
    JellyhajWidget<R> for FetchWidget<F>
{
    type Action = FetchAction;

    type ActionResult = Navigation;

    type State = FetchState<F>;

    fn min_width(&self) -> Option<u16> {
        JellyhajWidget::<R>::min_width(&self.inner)
    }

    fn min_height(&self) -> Option<u16> {
        JellyhajWidget::<R>::min_height(&self.inner)
    }

    fn into_state(self) -> Self::State {
        FetchState {
            fut: self.fut,
            inner: JellyhajWidget::<R>::into_state(self.inner),
        }
    }

    fn accepts_text_input(&self) -> bool {
        JellyhajWidget::<R>::accepts_text_input(&self.inner)
    }

    fn accept_char(&mut self, text: char) {
        JellyhajWidget::<R>::accept_char(&mut self.inner, text);
    }

    fn accept_text(&mut self, text: String) {
        JellyhajWidget::<R>::accept_text(&mut self.inner, text);
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        match action {
            FetchAction::Inner(action) => {
                let None = self
                    .inner
                    .apply_action(cx.wrap_with(FetchAction::Inner), action)?;
                Ok(None)
            }
            FetchAction::FetchFinished(next_screen) => Ok(Some(Navigation::Replace(next_screen))),
            FetchAction::FetchTimeout => Ok(Some(Navigation::Replace(NextScreen::Error(
                color_eyre::eyre::eyre!("fetch timeout reached.\n\n(this can be configured)"),
            )))),
        }
    }

    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        position: jellyhaj_widgets_core::Position,
        size: jellyhaj_widgets_core::Size,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        let None = self.inner.click(
            cx.wrap_with(FetchAction::Inner),
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
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()> {
        if self.fut.is_some() {
            let fut = self.fut.take().expect("just checked");
            cx.submitter.spawn_task(
                async move { Ok(FetchAction::FetchFinished(fut.await?)) },
                info_span!("do_fetch"),
                "do_fetch",
            );
            cx.submitter
                .wrap_with(|_| FetchAction::FetchTimeout)
                .spawn_task_infallible(
                    tokio::time::sleep(Duration::from_secs(
                        Config::get_ref(cx.refs).fetch_timeout.into(),
                    )),
                    info_span!("fetch_timeout"),
                    "fetch_timeout",
                );
        }
        self.inner
            .render_fallible_inner(area, buf, cx.wrap_with(FetchAction::Inner))
    }
}
