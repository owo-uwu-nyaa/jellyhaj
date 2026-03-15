mod action;
mod click;
mod render;

use std::{fmt::Debug, marker::PhantomData, ops::ControlFlow};

use jellyhaj_core::{CommandMapper, Config, render::KeybindAction, state::Navigation};
use jellyhaj_widgets_core::{
    ContextRef, JellyhajWidget, JellyhajWidgetState, WidgetContext, Wrapper,
};
use keybinds::{BindingMap, Command};
use tracing::instrument;

pub struct KeybindWidget<
    R: ContextRef<Config> + 'static,
    T: Command,
    W: JellyhajWidget<R>,
    M: CommandMapper<T, A = W::Action>,
> {
    pub inner: W,
    top: BindingMap<T>,
    next_maps: Option<BindingMap<T>>,
    mapper: M,
    current_view: usize,
    _r: PhantomData<fn(R) -> ()>,
}

pub struct KeybindState<
    R: ContextRef<Config> + 'static,
    T: Command,
    S: JellyhajWidgetState<R>,
    M: CommandMapper<T, A = S::Action>,
> {
    pub inner: S,
    top: BindingMap<T>,
    mapper: M,
    _r: PhantomData<fn(R) -> ()>,
}

impl<
    R: ContextRef<Config> + 'static,
    T: Command + Debug,
    S: JellyhajWidgetState<R> + Debug,
    M: CommandMapper<T, A = S::Action>,
> Debug for KeybindState<R, T, S, M>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeybindState")
            .field("inner", &self.inner)
            .field("top", &self.top)
            .finish()
    }
}

impl<
    R: ContextRef<Config> + 'static,
    T: Command,
    S: JellyhajWidgetState<R>,
    M: CommandMapper<T, A = S::Action>,
> KeybindState<R, T, S, M>
{
    pub fn new(inner: S, top: BindingMap<T>, mapper: M) -> Self {
        Self {
            inner,
            top,
            mapper,
            _r: PhantomData,
        }
    }
}

impl<
    R: ContextRef<Config> + 'static,
    T: Command,
    W: JellyhajWidget<R>,
    M: CommandMapper<T, A = W::Action>,
> KeybindWidget<R, T, W, M>
{
    pub fn new(inner: W, top: BindingMap<T>, mapper: M) -> Self {
        Self {
            inner,
            top,
            next_maps: None,
            mapper,
            current_view: 0,
            _r: PhantomData,
        }
    }
}

impl<
    R: ContextRef<Config> + 'static,
    T: Command,
    S: JellyhajWidgetState<R>,
    M: CommandMapper<T, A = S::Action>,
> JellyhajWidgetState<R> for KeybindState<R, T, S, M>
{
    type Action = KeybindAction<S::Action>;

    type ActionResult = ControlFlow<Navigation, S::ActionResult>;

    type Widget = KeybindWidget<R, T, S::Widget, M>;

    const NAME: &str = "keybinds";

    fn visit_children(visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        visitor.visit::<R, S>();
    }

    fn into_widget(self, cx: &R) -> Self::Widget {
        KeybindWidget {
            inner: self.inner.into_widget(cx),
            top: self.top,
            next_maps: None,
            mapper: self.mapper,
            current_view: 0,
            _r: PhantomData,
        }
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        match action {
            KeybindAction::Inner(a) => {
                Ok(
                    S::apply_action(&mut self.inner, cx.wrap_with(KeybindWrapper), a)?
                        .map(ControlFlow::Continue),
                )
            }
            KeybindAction::Key(_) => Ok(None),
        }
    }
}

impl<
    R: ContextRef<Config> + 'static,
    T: Command,
    W: JellyhajWidget<R>,
    M: CommandMapper<T, A = W::Action>,
> JellyhajWidget<R> for KeybindWidget<R, T, W, M>
{
    type Action = KeybindAction<W::Action>;

    type ActionResult = ControlFlow<Navigation, W::ActionResult>;

    type State = KeybindState<R, T, W::State, M>;

    fn min_width(&self) -> Option<u16> {
        Some(24)
    }

    fn min_height(&self) -> Option<u16> {
        Some(7)
    }

    fn into_state(self) -> Self::State {
        KeybindState {
            inner: self.inner.into_state(),
            top: self.top,
            mapper: self.mapper,
            _r: PhantomData,
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

    #[instrument(skip_all, name = "action_keybinds")]
    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        action::apply_key_event(self, cx, action)
    }

    #[instrument(skip_all, name = "click_keybinds")]
    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        position: ratatui::prelude::Position,
        size: ratatui::prelude::Size,
        kind: ratatui::crossterm::event::MouseEventKind,
        modifier: ratatui::crossterm::event::KeyModifiers,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        click::apply_click(self, cx, position, size, kind, modifier)
    }

    #[instrument(skip_all, name = "render_keybind")]
    fn render_fallible_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> jellyhaj_widgets_core::Result<()> {
        render::render_keybinds(self, area, buf, cx)
    }
}

#[derive(Clone, Copy)]
struct KeybindWrapper;

impl<T: Debug + Send + 'static> Wrapper<T> for KeybindWrapper {
    type F = KeybindAction<T>;

    fn wrap(&self, val: T) -> Self::F {
        KeybindAction::Inner(val)
    }
}
