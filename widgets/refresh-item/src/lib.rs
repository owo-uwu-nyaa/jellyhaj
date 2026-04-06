use std::{convert::Infallible, fmt::Debug, ops::ControlFlow};

use color_eyre::eyre::Context;
use jellyfin::{
    JellyfinClient,
    items::{RefreshItemQuery, RefreshMode},
};
use jellyhaj_core::{Config, keybinds::FormCommand, state::Navigation};
use jellyhaj_form_widget::{
    Selection,
    button::{ActionCreator, Button},
    form::{FormCommandMapper, FormData},
    form_widget,
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::{
    ContextRef, GetFromContext, JellyhajWidget, Result, WidgetContext, Wrapper,
    spawn::tracing::info_span,
};
use valuable::Valuable;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Selection, Valuable)]
enum Action {
    #[default]
    #[descr("Scan for new and updated files")]
    NewUpdated,
    #[descr("Search for missing metadata")]
    MissingMetadata,
    #[descr("Replace all metadata")]
    ReplaceMetadata,
}

#[derive(Debug)]
pub enum FormResult {
    Submit,
}

impl From<Infallible> for FormResult {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

#[derive(Default, Debug)]
struct AC;
impl ActionCreator for AC {
    type T = FormResult;

    fn make_action(&self) -> Self::T {
        FormResult::Submit
    }
}

#[derive(Default, Debug, Valuable)]
#[form_widget("Refresh Metadata", FormResult)]
pub struct RefreshItem {
    #[descr("Refresh mode")]
    action: Action,
    #[descr("Replace existing images")]
    #[show_if(self.action != Action::NewUpdated)]
    replace_images: bool,
    #[descr("Replace existing trickplay images")]
    #[show_if(self.action != Action::NewUpdated)]
    replace_trickplay: bool,
    #[descr("Refresh Now!")]
    refresh: Button<AC>,
}

impl RefreshItem {
    fn to_query(&self) -> RefreshItemQuery {
        match self.action {
            Action::NewUpdated => RefreshItemQuery {
                recursive: true,
                metadata_refresh_mode: RefreshMode::Default,
                image_refresh_mode: RefreshMode::Default,
                replace_all_metadata: false,
                replace_all_images: false,
                regenerate_trickplay: false,
            },
            Action::MissingMetadata => RefreshItemQuery {
                recursive: true,
                metadata_refresh_mode: RefreshMode::FullRefresh,
                image_refresh_mode: RefreshMode::FullRefresh,
                replace_all_metadata: false,
                replace_all_images: self.replace_images,
                regenerate_trickplay: self.replace_trickplay,
            },
            Action::ReplaceMetadata => RefreshItemQuery {
                recursive: true,
                metadata_refresh_mode: RefreshMode::FullRefresh,
                image_refresh_mode: RefreshMode::FullRefresh,
                replace_all_metadata: true,
                replace_all_images: self.replace_images,
                regenerate_trickplay: self.replace_trickplay,
            },
        }
    }
}

type InnerWidget =
    KeybindWidget<FormCommand, RefreshItemWidget, FormCommandMapper<RefreshItemAction>>;

#[derive(Valuable)]
pub struct RefreshWidget {
    #[valuable(skip)]
    inner: InnerWidget,
    id: String,
}

impl RefreshWidget {
    pub fn new(id: String, cx: &impl ContextRef<Config>) -> Self {
        Self {
            inner: KeybindWidget::new(
                RefreshItem::default().make_with(RefreshItemSelection::Action(None)),
                Config::get_ref(cx).keybinds.form.clone(),
                Default::default(),
            ),
            id,
        }
    }
}

fn map<R: ContextRef<Config> + ContextRef<JellyfinClient> + 'static, A>(
    this: &RefreshWidget,
    res: Result<Option<ControlFlow<Navigation, ControlFlow<Navigation, FormResult>>>>,
    cx: WidgetContext<'_, A, impl Wrapper<A>, R>,
) -> Result<Option<Navigation>> {
    Ok(match res? {
        None => None,
        Some(ControlFlow::Break(b)) => Some(b),
        Some(ControlFlow::Continue(ControlFlow::Break(b))) => Some(b),
        Some(ControlFlow::Continue(ControlFlow::Continue(FormResult::Submit))) => {
            let jellyfin = JellyfinClient::get_ref(cx.refs).clone();
            let id = this.id.clone();
            let query = this.inner.inner.data.to_query();
            cx.submitter.spawn_res(
                async move {
                    jellyfin
                        .refresh_item(&id, &query)
                        .await
                        .context("refreshing jellyfin item")
                },
                info_span!("send_refresh_item"),
                "send_refresh_item",
            );
            Some(Navigation::PopContext)
        }
    })
}

impl<R: ContextRef<Config> + ContextRef<JellyfinClient> + 'static> JellyhajWidget<R>
    for RefreshWidget
{
    type Action = <InnerWidget as JellyhajWidget<R>>::Action;

    type ActionResult = Navigation;

    const NAME: &str = "refresh-item";

    fn visit_children(&self, visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        visitor.visit::<R, InnerWidget>(&self.inner);
    }

    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {
        self.inner.init(cx);
    }

    fn min_width(&self) -> Option<u16> {
        JellyhajWidget::<R>::min_width(&self.inner)
    }

    fn min_height(&self) -> Option<u16> {
        JellyhajWidget::<R>::min_height(&self.inner)
    }

    fn accepts_text_input(&self) -> bool {
        JellyhajWidget::<R>::accepts_text_input(&self.inner)
    }

    fn accept_char(&mut self, text: char) {
        JellyhajWidget::<R>::accept_char(&mut self.inner, text);
    }

    fn accept_text(&mut self, text: String) {
        JellyhajWidget::<R>::accept_text(&mut self.inner, text);
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        let res = self.inner.apply_action(cx, action);
        map(self, res, cx)
    }

    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        position: jellyhaj_widgets_core::Position,
        size: jellyhaj_widgets_core::Size,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        let res = self.inner.click(cx, position, size, kind, modifier);
        map(self, res, cx)
    }

    fn render_fallible_inner(
        &mut self,
        area: jellyhaj_widgets_core::Rect,
        buf: &mut jellyhaj_widgets_core::Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()> {
        self.inner.render_fallible_inner(area, buf, cx)
    }
}
