use std::ops::ControlFlow;

use color_eyre::eyre::Report;
use jellyhaj_core::{
    CommandMapper,
    context::{DefaultTerminal, KeybindEvents, TuiContext},
    keybinds::ErrorCommand,
    render::render_widget,
    state::Navigation,
};
use jellyhaj_error_widget::{ErrorAction, ErrorWidget};
use jellyhaj_keybinds_widget::KeybindState;
use jellyhaj_widgets_core::outer::{Named, OuterState};

struct Mapper;

impl CommandMapper<ErrorCommand> for Mapper {
    type A = ErrorAction;

    fn map(&self, command: ErrorCommand) -> ControlFlow<Navigation, Self::A> {
        match command {
            ErrorCommand::Quit => ControlFlow::Break(Navigation::PopContext),
            ErrorCommand::Kill => ControlFlow::Break(Navigation::Exit),
            ErrorCommand::Up => ControlFlow::Continue(ErrorAction::Up),
            ErrorCommand::Down => ControlFlow::Continue(ErrorAction::Down),
            ErrorCommand::Left => ControlFlow::Continue(ErrorAction::Left),
            ErrorCommand::Right => ControlFlow::Continue(ErrorAction::Right),
            ErrorCommand::Global(g) => ControlFlow::Break(g.into()),
        }
    }
}

struct Name;
impl Named for Name {
    const NAME: &str = "error";
}

pub fn render_error(
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    cx: TuiContext,
    e: Report,
) -> impl Future<Output = jellyhaj_core::render::NavigationResult> {
    tracing::error!("Error encountered: {e:?}");
    let state = OuterState::<Name, _, _, _, _>::new(KeybindState::new(
        ErrorWidget::new(format!("{e:?}")),
        cx.config.keybinds.error.clone(),
        Mapper,
    ));
    render_widget(term, events, cx, state)
}
