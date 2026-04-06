use jellyhaj_core::{
    context::TuiContext,
    render::{Erased, make_new_erased},
};
use jellyhaj_refresh_item_widget::RefreshWidget;

pub fn render_refresh_item_form(cx: TuiContext, id: String) -> Erased {
    let widget = RefreshWidget::new(id, &cx);
    make_new_erased(cx, widget)
}
