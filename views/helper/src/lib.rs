use jellyhaj_core::{
    Config,
    state::Navigation,
    widgets::shaded::widget::{Erased, make_new_erased},
};
use jellyhaj_helper_widget::{ExitWidget, ReadyWidget};
use jellyhaj_widgets_core::{ContextRef, spawn::Spawner};

pub fn make_exit<R: ContextRef<Spawner> + ContextRef<Config> + 'static>(cx: R) -> Erased {
    make_new_erased(cx, ExitWidget)
}
pub fn make_ready<R: ContextRef<Spawner> + ContextRef<Config> + 'static>(
    cx: R,
    nav: Box<Navigation>,
) -> Erased {
    make_new_erased(cx, ReadyWidget::new(nav))
}
