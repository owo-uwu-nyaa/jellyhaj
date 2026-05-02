use color_eyre::eyre::Context;
use jellyfin::items::RefreshItemQuery;
use jellyhaj_core::{
    context::TuiContext,
    render::{Erased, make_new_erased},
    state::Navigation,
};
use jellyhaj_form_widget::form::{FormCommandMapper, FormDataDefaultExt};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_refresh_item_widget::RefreshItem;
use jellyhaj_widgets_core::outer::{Named, OuterWidget, UnwrapWidget};

struct Name;
impl Named for Name {
    const NAME: &str = "refresh-item";
}

#[must_use]
pub fn render_refresh_item_form(cx: TuiContext, id: String) -> Erased {
    let widget = OuterWidget::<Name, _>::new(KeybindWidget::new(
        UnwrapWidget::new(RefreshItem::new(id).make_with_default()),
        cx.config.keybinds.form.clone(),
        FormCommandMapper::default(),
    ));
    make_new_erased(cx, widget)
}

#[must_use]
pub fn render_do_refresh_item(cx: TuiContext, id: String, query: RefreshItemQuery) -> Erased {
    let jellyfin = cx.jellyfin.clone();
    let fut = async move {
        jellyfin
            .refresh_item(&id, &query)
            .await
            .context("refreshing item")?;
        Ok(Navigation::PopContext)
    };
    jellyhaj_fetch_view::make_nav_fetch(cx, "Refreshing Item", fut)
}
