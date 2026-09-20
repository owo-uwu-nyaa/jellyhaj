use jellyhaj_core::state::Navigation;
use jellyhaj_widgets_core::{
    Buffer, Cursor, JellyhajWidget, JellyhajWidgetBase, KeybindAction, MouseEventKind, Position,
    Rect, RenderFlag, Result, Size, WidgetContext, Wrapper,
    ratatui::widgets::{Paragraph, Widget},
};
use valuable::Valuable;

#[derive(Debug)]
pub enum ExitAction {
    Quit,
}
#[derive(Valuable)]
pub struct ExitWidget;

impl JellyhajWidgetBase for ExitWidget {
    type Action = KeybindAction<ExitAction>;

    type ActionResult = Navigation;

    const NAME: &str = "quit";

    fn visit_children(&self, _: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {}

    fn min_width(&self) -> Option<u16> {
        None
    }

    fn min_height(&self) -> Option<u16> {
        None
    }
}

impl<R: 'static> JellyhajWidget<R> for ExitWidget {
    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {
        cx.submitter
            .spawn_value_infallible(KeybindAction::Inner(ExitAction::Quit));
    }

    fn apply_action(
        &mut self,
        _cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
        _render_flag: &mut RenderFlag,
    ) -> Result<Option<Self::ActionResult>> {
        if matches!(action, KeybindAction::Inner(ExitAction::Quit)) {
            Ok(Some(Navigation::Exit))
        } else {
            Ok(None)
        }
    }

    fn click(
        &mut self,
        _cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        _position: Position,
        _size: Size,
        _kind: MouseEventKind,
        _modifier: jellyhaj_widgets_core::KeyModifiers,
        _render_flag: &mut RenderFlag,
    ) -> Result<Option<Self::ActionResult>> {
        Ok(None)
    }

    fn render_fallible_inner(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        _cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        _cursor: &mut Option<Cursor>,
    ) -> Result<()> {
        Paragraph::new("stopping").centered().render(area, buf);
        Ok(())
    }
}

#[derive(Valuable)]
pub struct ReadyWidget {
    #[valuable(skip)]
    val: Option<Box<Navigation>>,
}

impl ReadyWidget {
    #[must_use]
    pub const fn new(val: Box<Navigation>) -> Self {
        Self { val: Some(val) }
    }
}

impl JellyhajWidgetBase for ReadyWidget {
    type Action = KeybindAction<Option<Box<Navigation>>>;

    type ActionResult = Navigation;

    const NAME: &str = "ready";

    fn visit_children(&self, _visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {}

    fn min_width(&self) -> Option<u16> {
        None
    }

    fn min_height(&self) -> Option<u16> {
        None
    }
}

impl<R: 'static> JellyhajWidget<R> for ReadyWidget {
    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {
        cx.submitter
            .spawn_value_infallible(KeybindAction::Inner(self.val.take()));
        cx.submitter
            .spawn_value_infallible(KeybindAction::Inner(None));
    }

    fn apply_action(
        &mut self,
        _cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
        _render_flag: &mut RenderFlag,
    ) -> Result<Option<Self::ActionResult>> {
        match action {
            KeybindAction::Inner(Some(nav)) => Ok(Some(*nav)),
            KeybindAction::Inner(None) => Ok(Some(Navigation::PopContext)),
            KeybindAction::Key(_) => Ok(None),
        }
    }

    fn click(
        &mut self,
        _cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        _position: Position,
        _size: Size,
        _kind: MouseEventKind,
        _modifier: jellyhaj_widgets_core::KeyModifiers,
        _render_flag: &mut RenderFlag,
    ) -> Result<Option<Self::ActionResult>> {
        Ok(None)
    }

    fn render_fallible_inner(
        &mut self,
        _area: Rect,
        _buf: &mut Buffer,
        _cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        _cursor: &mut Option<Cursor>,
    ) -> Result<()> {
        Ok(())
    }
}
