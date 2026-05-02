use std::ops::ControlFlow;

use jellyhaj_core::{
    CommandMapper,
    context::TuiContext,
    keybinds::InspectCommand,
    render::{Erased, make_new_erased},
    state::Navigation,
};
use jellyhaj_inspect_widget::{InspectAction, InspectWidget};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::outer::{Named, OuterWidget};

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

#[must_use]
pub fn render_inspect(cx: TuiContext) -> Erased {
    let widget = OuterWidget::<Name, _>::new(KeybindWidget::new(
        InspectWidget::default(),
        cx.config.keybinds.inspect.clone(),
        Mapper,
    ));
    make_new_erased(cx, widget)
}
