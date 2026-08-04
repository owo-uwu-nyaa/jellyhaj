use std::ops::ControlFlow;

use color_eyre::eyre::Report;
use jellyhaj_core::{
    CommandMapper, Config,
    keybinds::ErrorCommand,
    state::Navigation,
    widgets::shaded::widget::{Erased, make_new_erased},
};
use jellyhaj_error_widget::{ErrorAction, ErrorWidget};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::{
    ContextRef, GetFromContext,
    outer::{Named, OuterWidget},
};
use spawn::Spawner;

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
    cx: impl ContextRef<Config> + ContextRef<Spawner> + Send + 'static,
    e: &Report,
) -> Erased {
    tracing::error!("Error encountered: {e:?}");
    let top = Config::get_ref(&cx).keybinds.error.clone();
    let widget = OuterWidget::<Name, _>::new(KeybindWidget::new(
        ErrorWidget::new(format!("{e:?}")),
        top,
        Mapper,
    ));
    make_new_erased(cx, widget)
}
