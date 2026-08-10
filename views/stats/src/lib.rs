use std::ops::ControlFlow;

use jellyhaj_core::{
    CommandMapper, Config,
    keybinds::StatsCommand,
    state::Navigation,
    widgets::shaded::widget::{Erased, make_new_erased},
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_stats_widget::{StatsUpdate, StatsWidget};
use jellyhaj_widgets_core::{
    ContextRef, GetFromContext,
    outer::{Named, OuterWidget},
};
use spawn::Spawner;
use stats_data::StatsData;

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

pub fn render_stats(
    cx: impl ContextRef<Config> + ContextRef<Spawner> + ContextRef<StatsData> + Send + 'static,
) -> Erased {
    let top = Config::get_ref(&cx).keybinds.stats.clone();
    let widget = StatsWidget::new(cx.as_ref());
    let widget = OuterWidget::<Name, _>::new(KeybindWidget::new(widget, top, Mapper));
    make_new_erased(cx, widget)
}
