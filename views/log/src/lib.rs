use std::ops::ControlFlow;

use jellyhaj_core::{
    CommandMapper,
    context::{DefaultTerminal, KeybindEvents, TuiContext},
    keybinds::LoggerCommand,
    render::{NavigationResult, render_widget},
    state::Navigation,
};
use jellyhaj_keybinds_widget::KeybindState;
use jellyhaj_log_widget::{LogWidget, TuiWidgetEvent};
use jellyhaj_widgets_core::outer::{Named, OuterState};

struct Mapper;

impl CommandMapper<LoggerCommand> for Mapper {
    type A = TuiWidgetEvent;

    fn map(&self, command: LoggerCommand) -> ControlFlow<Navigation, Self::A> {
        let event = match command {
            LoggerCommand::Space => TuiWidgetEvent::SpaceKey,
            LoggerCommand::TargetUp => TuiWidgetEvent::UpKey,
            LoggerCommand::TargetDown => TuiWidgetEvent::DownKey,
            LoggerCommand::Left => TuiWidgetEvent::LeftKey,
            LoggerCommand::Right => TuiWidgetEvent::RightKey,
            LoggerCommand::Plus => TuiWidgetEvent::PlusKey,
            LoggerCommand::Minus => TuiWidgetEvent::MinusKey,
            LoggerCommand::Hide => TuiWidgetEvent::HideKey,
            LoggerCommand::Focus => TuiWidgetEvent::FocusKey,
            LoggerCommand::MessagesUp => TuiWidgetEvent::PrevPageKey,
            LoggerCommand::MessagesDown => TuiWidgetEvent::NextPageKey,
            LoggerCommand::Escape => TuiWidgetEvent::EscapeKey,
            LoggerCommand::Quit => return ControlFlow::Break(Navigation::PopContext),
            LoggerCommand::Global(g) => return ControlFlow::Break(g.into()),
        };
        ControlFlow::Continue(event)
    }
}

struct Name;
impl Named for Name {
    const NAME: &str = "log-view";
}

pub fn render_log(
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    cx: TuiContext,
) -> impl Future<Output = NavigationResult> {
    let state = OuterState::<Name, _, _, _, _>::new(KeybindState::new(
        LogWidget::new(),
        cx.config.keybinds.logger.clone(),
        Mapper,
    ));
    render_widget(term, events, cx, state)
}
