mod action;
mod click;
mod render;

use std::{fmt::Debug, ops::ControlFlow, sync::Arc};

use jellyhaj_core::{CommandMapper, render::KeybindAction, state::Navigation};
use jellyhaj_widgets_core::{
    JellyhajWidget, JellyhajWidgetState, Wrapper, async_task::TaskSubmitter,
};
use keybinds::{BindingMap, Command};
use tracing::instrument;

pub struct KeybindWidget<T: Command, W: JellyhajWidget, M: CommandMapper<T, A = W::Action>> {
    pub inner: W,
    help_prefixes: Arc<[String]>,
    top: BindingMap<T>,
    next_maps: Option<BindingMap<T>>,
    mapper: M,
    current_view: usize,
}

pub struct KeybindState<T: Command, S: JellyhajWidgetState, M: CommandMapper<T, A = S::Action>> {
    pub inner: S,
    help_prefixes: Arc<[String]>,
    top: BindingMap<T>,
    mapper: M,
}

impl<T: Command + Debug, S: JellyhajWidgetState + Debug, M: CommandMapper<T, A = S::Action>> Debug
    for KeybindState<T, S, M>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeybindState")
            .field("inner", &self.inner)
            .field("help_prefixes", &self.help_prefixes)
            .field("top", &self.top)
            .finish()
    }
}

impl<T: Command, S: JellyhajWidgetState, M: CommandMapper<T, A = S::Action>> KeybindState<T, S, M> {
    pub fn new(inner: S, help_prefixes: Arc<[String]>, top: BindingMap<T>, mapper: M) -> Self {
        Self {
            inner,
            help_prefixes,
            top,
            mapper,
        }
    }
}

impl<T: Command, W: JellyhajWidget, M: CommandMapper<T, A = W::Action>> KeybindWidget<T, W, M> {
    pub fn new(inner: W, help_prefixes: Arc<[String]>, top: BindingMap<T>, mapper: M) -> Self {
        Self {
            inner,
            help_prefixes,
            top,
            next_maps: None,
            mapper,
            current_view: 0,
        }
    }
}

impl<T: Command, S: JellyhajWidgetState, M: CommandMapper<T, A = S::Action>> JellyhajWidgetState
    for KeybindState<T, S, M>
{
    type Action = KeybindAction<S::Action>;

    type ActionResult = ControlFlow<Navigation, S::ActionResult>;

    type Widget = KeybindWidget<T, S::Widget, M>;

    const NAME: &str = "keybinds";

    fn visit_children(visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        visitor.visit::<S>();
    }

    fn into_widget(
        self,
        cx: std::pin::Pin<&mut jellyhaj_core::context::TuiContext>,
    ) -> Self::Widget {
        KeybindWidget {
            inner: self.inner.into_widget(cx),
            help_prefixes: self.help_prefixes,
            top: self.top,
            next_maps: None,
            mapper: self.mapper,
            current_view: 0,
        }
    }

    fn apply_action(
        &mut self,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        match action {
            KeybindAction::Inner(a) => {
                Ok(
                    S::apply_action(&mut self.inner, task.wrap_with(KeybindWrapper), a)?
                        .map(ControlFlow::Continue),
                )
            }
            KeybindAction::Key(_) => Ok(None),
        }
    }
}

impl<T: Command, W: JellyhajWidget, M: CommandMapper<T, A = W::Action>> JellyhajWidget
    for KeybindWidget<T, W, M>
{
    type Action = KeybindAction<W::Action>;

    type ActionResult = ControlFlow<Navigation, W::ActionResult>;

    type State = KeybindState<T, W::State, M>;

    fn min_width(&self) -> Option<u16> {
        Some(24)
    }

    fn min_height(&self) -> Option<u16> {
        Some(7)
    }

    fn into_state(self) -> Self::State {
        KeybindState {
            inner: self.inner.into_state(),
            help_prefixes: self.help_prefixes,
            top: self.top,
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

    #[instrument(skip_all, name = "action_keybinds")]
    fn apply_action(
        &mut self,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        action::apply_key_event(self, task, action)
    }

    #[instrument(skip_all, name = "click_keybinds")]
    fn click(
        &mut self,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        position: ratatui::prelude::Position,
        size: ratatui::prelude::Size,
        kind: ratatui::crossterm::event::MouseEventKind,
        modifier: ratatui::crossterm::event::KeyModifiers,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        click::apply_click(self, task, position, size, kind, modifier)
    }

    #[instrument(skip_all, name = "render_keybind")]
    fn render_fallible_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
    ) -> jellyhaj_widgets_core::Result<()> {
        render::render_keybinds(self, area, buf, task)
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
