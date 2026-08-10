use std::ops::ControlFlow;

use jellyhaj_core::{
    CommandMapper, Config,
    keybinds::InspectCommand,
    state::Navigation,
    widgets::{
        shaded::widget::{Erased, make_new_erased},
        state::StateStack,
    },
};
use jellyhaj_inspect_widget::{InspectAction, InspectWidget};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::{
    ContextRef, GetFromContext,
    outer::{Named, OuterWidget},
};
use spawn::Spawner;

struct Mapper;
impl CommandMapper<InspectCommand> for Mapper {
    type A = InspectAction;

    fn map(&self, command: InspectCommand) -> ControlFlow<Navigation, Self::A> {
        let action = match command {
            InspectCommand::Toggle => InspectAction::Toggle,
            InspectCommand::Open => InspectAction::Open,
            InspectCommand::CloseMoveParent => InspectAction::CloseMoveParent,
            InspectCommand::Close => InspectAction::Close,
            InspectCommand::Up => InspectAction::Up,
            InspectCommand::Down => InspectAction::Down,
            InspectCommand::Copy => InspectAction::Copy,
            InspectCommand::Quit => return ControlFlow::Break(Navigation::PopContext),
            InspectCommand::Global(g) => return ControlFlow::Break(g.into()),
        };
        ControlFlow::Continue(action)
    }
}

struct Name;
impl Named for Name {
    const NAME: &str = "inspect";
}

pub fn render_inspect(
    cx: impl ContextRef<Config> + ContextRef<Spawner> + ContextRef<StateStack> + Send + 'static,
) -> Erased {
    let top = Config::get_ref(&cx).keybinds.inspect.clone();
    let widget = OuterWidget::<Name, _>::new(KeybindWidget::new(
        InspectWidget::widget_state(),
        top,
        Mapper,
    ));
    make_new_erased(cx, widget)
}
pub fn render_inspect_value(
    cx: impl ContextRef<Config> + ContextRef<Spawner> + ContextRef<StateStack> + Send + 'static,
    value: &serde_json::Value,
) -> Erased {
    let top = Config::get_ref(&cx).keybinds.inspect.clone();
    let widget = OuterWidget::<Name, _>::new(KeybindWidget::new(
        InspectWidget::json_value(value),
        top,
        Mapper,
    ));
    make_new_erased(cx, widget)
}
