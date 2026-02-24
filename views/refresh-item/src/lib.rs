use std::pin::Pin;

use jellyhaj_core::render::render_widget;
use jellyhaj_refresh_item_widget::RefreshState;
use jellyhaj_widgets_core::TuiContext;

pub fn render_refresh_item_form(
    mut cx: Pin<&mut TuiContext>,
    id: String,
) -> impl Future<Output = jellyhaj_core::render::NavigationResult> {
    let state = RefreshState::new(id, cx.as_mut());
    render_widget(cx, state)
}
