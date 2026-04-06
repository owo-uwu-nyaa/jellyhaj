use std::convert::Infallible;

use jellyhaj_widgets_core::{
    JellyhajWidget, Result, WidgetContext, WidgetTreeVisitor, Wrapper,
    valuable::{Fields, NamedField, NamedValues, StructDef, Structable, Valuable, Value},
};
use ratatui::{
    style::{Color, Style},
    widgets::{Block, Padding, Widget},
};
pub use tui_logger::TuiWidgetEvent;
use tui_logger::{TuiLoggerLevelOutput, TuiWidgetState};

#[derive(Default)]
pub struct LogWidget {
    state: TuiWidgetState,
}

static LOG_WIDGET_FIELDS: &[NamedField] = &[NamedField::new("state")];

impl Valuable for LogWidget {
    fn as_value(&self) -> Value<'_> {
        Value::Structable(self)
    }

    fn visit(&self, visit: &mut dyn jellyhaj_widgets_core::valuable::Visit) {
        visit.visit_named_fields(&NamedValues::new(
            LOG_WIDGET_FIELDS,
            &["state not inspectable".as_value()],
        ));
    }
}

impl Structable for LogWidget {
    fn definition(&self) -> StructDef<'_> {
        StructDef::new_static("LogWidget", Fields::Named(LOG_WIDGET_FIELDS))
    }
}

impl std::fmt::Debug for LogWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogWidget").finish()
    }
}

impl LogWidget {
    pub fn new() -> Self {
        Self {
            state: TuiWidgetState::new(),
        }
    }
}

impl<R: 'static> JellyhajWidget<R> for LogWidget {
    const NAME: &str = "log-view";

    type Action = TuiWidgetEvent;

    type ActionResult = Infallible;

    fn visit_children(&self, _visitor: &mut impl WidgetTreeVisitor) {}

    fn init(&mut self, _cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {}

    fn min_width(&self) -> Option<u16> {
        Some(15)
    }

    fn min_height(&self) -> Option<u16> {
        Some(15)
    }

    fn accepts_text_input(&self) -> bool {
        false
    }

    fn accept_char(&mut self, _: char) {
        unimplemented!()
    }

    fn accept_text(&mut self, _: String) {
        unimplemented!()
    }

    fn apply_action(
        &mut self,
        _: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        self.state.transition(action);
        Ok(None)
    }

    fn click(
        &mut self,
        _: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        _: jellyhaj_widgets_core::Position,
        _: jellyhaj_widgets_core::Size,
        _: jellyhaj_widgets_core::MouseEventKind,
        _: jellyhaj_widgets_core::KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        Ok(None)
    }

    fn render_fallible_inner(
        &mut self,
        area: jellyhaj_widgets_core::Rect,
        buf: &mut jellyhaj_widgets_core::Buffer,
        _: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()> {
        let block = Block::bordered()
            .title("Log Messages")
            .padding(Padding::uniform(1));
        tui_logger::TuiLoggerSmartWidget::default()
            .style_error(Style::default().fg(Color::Red))
            .style_debug(Style::default().fg(Color::Green))
            .style_warn(Style::default().fg(Color::Yellow))
            .style_trace(Style::default().fg(Color::Magenta))
            .style_info(Style::default().fg(Color::Cyan))
            .output_separator(':')
            .output_timestamp(Some("%H:%M:%S".to_string()))
            .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
            .output_target(true)
            .output_file(false)
            .output_line(false)
            .state(&self.state)
            .render(block.inner(area), buf);
        block.render(area, buf);
        Ok(())
    }
}
