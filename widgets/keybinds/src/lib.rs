mod action;
mod click;
mod render;

use std::{fmt::Debug, ops::ControlFlow};

use jellyhaj_core::{CommandMapper, Config, render::KeybindAction, state::Navigation};
use jellyhaj_widgets_core::{
    ContextRef, JellyhajWidget, WidgetContext, Wrapper,
    valuable::{Fields, NamedField, NamedValues, StructDef, Structable, Valuable, Value},
};
use keybinds::{BindingMap, Command};
use tracing::instrument;

pub struct KeybindWidget<T: Command, W, M> {
    pub inner: W,
    top_map: BindingMap<T>,
    current_map: Option<BindingMap<T>>,
    mapper: M,
    current_view: usize,
}

static KEYBIND_WIDGET_FIELDS: &[NamedField] = &[
    NamedField::new("top_map"),
    NamedField::new("current_map"),
    NamedField::new("current_view"),
];

impl<T: Command, W, M> Valuable for KeybindWidget<T, W, M> {
    fn as_value(&self) -> Value<'_> {
        Value::Structable(self)
    }

    fn visit(&self, visit: &mut dyn jellyhaj_widgets_core::valuable::Visit) {
        visit.visit_named_fields(&NamedValues::new(
            KEYBIND_WIDGET_FIELDS,
            &[
                self.top_map.as_value(),
                self.current_map.as_value(),
                self.current_view.as_value(),
            ],
        ));
    }
}
impl<T: Command, W, M> Structable for KeybindWidget<T, W, M> {
    fn definition(&self) -> StructDef<'_> {
        StructDef::new_static("KeybindWidget", Fields::Named(KEYBIND_WIDGET_FIELDS))
    }
}

impl<T: Command, W, M> KeybindWidget<T, W, M> {
    pub fn new(inner: W, top: BindingMap<T>, mapper: M) -> Self {
        Self {
            inner,
            top_map: top,
            current_map: None,
            mapper,
            current_view: 0,
        }
    }
}

impl<
    R: ContextRef<Config> + 'static,
    T: Command,
    W: JellyhajWidget<R>,
    M: CommandMapper<T, A = W::Action>,
> JellyhajWidget<R> for KeybindWidget<T, W, M>
{
    type Action = KeybindAction<W::Action>;

    type ActionResult = ControlFlow<Navigation, W::ActionResult>;

    const NAME: &str = "keybinds";

    fn visit_children(&self, visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        visitor.visit(&self.inner);
    }

    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {
        self.inner.init(cx.wrap_with(KeybindWrapper));
    }

    fn min_width(&self) -> Option<u16> {
        Some(24)
    }

    fn min_height(&self) -> Option<u16> {
        Some(7)
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
