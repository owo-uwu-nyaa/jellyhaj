use std::convert::Infallible;

use jellyhaj_widgets_core::{
    JellyhajWidget, JellyhajWidgetState, Result, TuiContext, Wrapper, async_task::TaskSubmitter,
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

impl JellyhajWidgetState for LogWidget {
    type Action = TuiWidgetEvent;

    type ActionResult = Infallible;

    type Widget = Self;

    const NAME: &str = "log-view";

    fn visit_children(_: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {}

    fn into_widget(self, _: std::pin::Pin<&mut TuiContext>) -> Self::Widget {
        self
    }

    fn apply_action(
        &mut self,
        _: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        self.state.transition(action);
        Ok(None)
    }
}

impl JellyhajWidget for LogWidget {
    type State = Self;

    type Action = TuiWidgetEvent;

    type ActionResult = Infallible;

    fn min_width(&self) -> Option<u16> {
        Some(15)
    }

    fn min_height(&self) -> Option<u16> {
        Some(15)
    }

    fn into_state(self) -> Self::State {
        self
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
        _: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        self.state.transition(action);
        Ok(None)
    }

    fn click(
        &mut self,
        _: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
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
        _: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
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
