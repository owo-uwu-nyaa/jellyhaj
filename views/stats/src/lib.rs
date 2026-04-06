use std::ops::ControlFlow;

use jellyhaj_core::{
    CommandMapper,
    context::TuiContext,
    keybinds::StatsCommand,
    render::{Erased, make_new_erased},
    state::Navigation,
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_stats_widget::{StatsUpdate, StatsWidget};
use jellyhaj_widgets_core::outer::{Named, OuterWidget};

struct Mapper;
impl CommandMapper<StatsCommand> for Mapper {
    type A = StatsUpdate;

    fn map(&self, command: StatsCommand) -> std::ops::ControlFlow<Navigation, Self::A> {
        match command {
            StatsCommand::Quit => ControlFlow::Break(Navigation::PopContext),
            StatsCommand::Global(g) => ControlFlow::Break(g.into()),
        }
    }
}

struct Name;
impl Named for Name {
    const NAME: &str = "stats";
}

pub fn render_stats(cx: TuiContext) -> Erased {
    let widget = OuterWidget::<Name, _>::new(KeybindWidget::new(
        StatsWidget::new(&cx.stats),
        cx.config.keybinds.stats.clone(),
        Mapper,
    ));
    make_new_erased(cx, widget)
}
