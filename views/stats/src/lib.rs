use std::{ops::ControlFlow, pin::Pin};

use color_eyre::Result;
use jellyhaj_core::{CommandMapper, context::TuiContext, keybinds::StatsCommand, state::Navigation};
use jellyhaj_keybinds_widget::KeybindState;
use jellyhaj_stats_widget::{StatsState, StatsUpdate, StatsWidget};
use jellyhaj_widgets_core::outer::{Named, OuterState};

struct Mapper;
impl CommandMapper<StatsCommand> for Mapper{
    type A = StatsUpdate;

    fn map(&self, command: StatsCommand) -> std::ops::ControlFlow<Navigation, Self::A> {
        match command {
            StatsCommand::Quit => ControlFlow::Break(Navigation::PopContext),
        }
    }
}

struct Name;
impl Named for Name{
    const NAME: &str = "stats";
}

pub async fn render_stats(cx: Pin<&mut TuiContext>) -> Result<Navigation> {
    let cx = cx.project();

    let mut state = OuterState::<Name,_,_>::new( KeybindState::new(
        StatsState,
        cx.config.help_prefixes.clone(),
        cx.config.keybinds.stats.clone(),
        Mapper
    ));

    }
