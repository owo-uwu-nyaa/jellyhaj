use std::ops::ControlFlow;

use jellyhaj_core::{
    CommandMapper, Config,
    keybinds::LoggerCommand,
    state::Navigation,
    widgets::shaded::widget::{Erased, make_new_erased},
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_log_widget::{LogWidget, TuiWidgetEvent};
use jellyhaj_widgets_core::{
    ContextRef, GetFromContext,
    outer::{Named, OuterWidget},
};
use spawn::Spawner;

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

pub fn render_log(cx: impl ContextRef<Config> + ContextRef<Spawner> + 'static) -> Erased {
    let top = Config::get_ref(&cx).keybinds.logger.clone();
    let widget = OuterWidget::<Name, _>::new(KeybindWidget::new(LogWidget::new(), top, Mapper));
    make_new_erased(cx, widget)
}
