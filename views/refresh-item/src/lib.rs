use jellyhaj_core::{
    context::{DefaultTerminal, KeybindEvents, TuiContext},
    render::render_widget,
};
use jellyhaj_refresh_item_widget::RefreshState;

pub fn render_refresh_item_form(
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    cx: TuiContext,
    id: String,
) -> impl Future<Output = jellyhaj_core::render::NavigationResult> {
    let state = RefreshState::new(id, &cx);
    render_widget(term, events, cx, state)
}
