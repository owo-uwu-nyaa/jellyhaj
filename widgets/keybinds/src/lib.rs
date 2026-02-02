mod action;
mod click;
mod render;

use jellyhaj_widgets_core::JellyhajWidget;
use keybinds::{BindingMap, Command};
use ratatui::crossterm::event::KeyEvent;

pub enum MappedCommand<U: Send + 'static, D: Send + 'static> {
    Up(U),
    Down(D),
}

pub enum KeybindAction<A: Send + 'static> {
    Inner(A),
    Key(KeyEvent),
}

pub enum CommandAction<U: Send + 'static, A> {
    Action(A),
    Up(U),
    Exit
}

pub trait CommandMapper<T: Command> {
    type U: Send + 'static;
    type D: Send + 'static;
    fn map(&self, command: T) -> MappedCommand<Self::U, Self::D>;
}

impl<T: Command, U: Send + 'static, D: Send + 'static, F: Fn(T) -> MappedCommand<U, D>>
    CommandMapper<T> for F
{
    type U = U;
    type D = D;
    fn map(&self, command: T) -> MappedCommand<U, D> {
        self(command)
    }
}

pub struct KeybindWidget<'e, T: Command, W: JellyhajWidget, M: CommandMapper<T, D = W::Action>> {
    inner: W,
    help_prefixes: &'e [String],
    top: BindingMap<T>,
    next_maps: Vec<BindingMap<T>>,
    minor: Vec<BindingMap<T>>,
    mapper: M,
    current_view: usize,
}

impl<'e, T: Command, W: JellyhajWidget, M: CommandMapper<T, D = W::Action>>
    KeybindWidget<'e, T, W, M>
{
    pub fn new(
        inner: W,
        help_prefixes: &'e [String],
        top: BindingMap<T>,
        mapper: M,
    ) -> Self {
        Self {
            inner,
            help_prefixes,
            top,
            next_maps: Vec::new(),
            minor: Vec::new(),
            mapper,
            current_view: 0,
        }
    }
    pub fn new_with_minor(
        inner: W,
        help_prefixes: &'e [String],
        top: BindingMap<T>,
        minor: Vec<BindingMap<T>>,
        mapper: M,
    ) -> Self {
        Self {
            inner,
            help_prefixes,
            top,
            next_maps: Vec::new(),
            minor,
            mapper,
            current_view: 0,
        }
    }

}

impl<'e, T: Command, W: JellyhajWidget, M: CommandMapper<T, D = W::Action>> JellyhajWidget
    for KeybindWidget<'e, T, W, M>
{
    type State = W::State;

    type Action = KeybindAction<W::Action>;

    type ActionResult = CommandAction<M::U, W::ActionResult>;

    fn min_width(&self) -> Option<u16> {
        Some(24)
    }

    fn min_height(&self) -> Option<u16> {
        Some(7)
    }

    fn min_width_static(_: jellyhaj_widgets_core::DimensionsParameter<'_>) -> Option<u16> {
        Some(24)
    }

    fn min_height_static(_: jellyhaj_widgets_core::DimensionsParameter<'_>) -> Option<u16> {
        Some(7)
    }

    fn into_state(self) -> Self::State {
        self.inner.into_state()
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
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        action::apply_key_event(self, action)
    }

    fn click(
        &mut self,
        position: ratatui::prelude::Position,
        size: ratatui::prelude::Size,
        kind: ratatui::crossterm::event::MouseEventKind,
        modifier: ratatui::crossterm::event::KeyModifiers,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        click::apply_click(self, position, size, kind, modifier)
    }

    fn render_fallible_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        task: jellyhaj_widgets_core::async_task::TaskSubmitter<
            Self::Action,
            impl jellyhaj_widgets_core::Wrapper<Self::Action>,
        >,
    ) -> jellyhaj_widgets_core::Result<()> {
        render::render_keybinds(self, area, buf, task)
    }
}
