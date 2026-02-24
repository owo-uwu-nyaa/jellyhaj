use std::{ops::ControlFlow, pin::Pin};

use jellyhaj_core::{
    CommandMapper,
    context::TuiContext,
    keybinds::StatsCommand,
    render::{NavigationResult, render_widget},
    state::Navigation,
};
use jellyhaj_keybinds_widget::KeybindState;
use jellyhaj_stats_widget::{StatsState, StatsUpdate};
use jellyhaj_widgets_core::outer::{Named, OuterState};

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

pub fn render_stats(cx: Pin<&mut TuiContext>) -> impl Future<Output = NavigationResult> {
    let state = OuterState::<Name, _, _, _>::new(KeybindState::new(
        StatsState,
        cx.config.help_prefixes.clone(),
        cx.config.keybinds.stats.clone(),
        Mapper,
    ));
    render_widget(cx, state)
}
