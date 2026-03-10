use std::{convert::Infallible, ops::ControlFlow, pin::Pin};

use color_eyre::eyre::Context;
use jellyfin::{
    JellyfinClient,
    items::{RefreshItemQuery, RefreshMode},
};
use jellyhaj_core::{context::Spawner, keybinds::FormCommand, state::Navigation};
use jellyhaj_form_widget::{
    Selection,
    button::{ActionCreator, Button},
    form::{FormCommandMapper, FormData},
    form_widget,
};
use jellyhaj_keybinds_widget::{KeybindState, KeybindWidget};
use jellyhaj_widgets_core::{
    JellyhajWidget, JellyhajWidgetState, Result, TuiContext, WidgetContext, Wrapper,
    spawn::tracing::info_span,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Selection)]
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

#[derive(Default, Debug)]
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

type InnerState = KeybindState<FormCommand, RefreshItemState, FormCommandMapper>;
type InnerWidget = KeybindWidget<FormCommand, RefreshItemWidget, FormCommandMapper>;

#[derive(Debug)]
pub struct RefreshState {
    inner: InnerState,
    jellyfin: JellyfinClient,
    id: String,
}

impl RefreshState {
    pub fn new(id: String, cx: Pin<&mut TuiContext>) -> Self {
        Self {
            inner: KeybindState::new(
                RefreshItem::default().make_state_with(RefreshItemSelection::Action(None)),
                cx.config.help_prefixes.clone(),
                cx.config.keybinds.form.clone(),
                FormCommandMapper,
            ),
            jellyfin: cx.jellyfin.clone(),
            id,
        }
    }
}

pub struct RefreshWidget {
    inner: InnerWidget,
    jellyfin: JellyfinClient,
    id: String,
}

impl JellyhajWidgetState for RefreshState {
    type Action = <InnerState as JellyhajWidgetState>::Action;

    type ActionResult = Navigation;

    type Widget = RefreshWidget;

    const NAME: &str = "refresh-item";

    fn visit_children(visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        visitor.visit::<InnerState>();
    }

    fn into_widget(
        self,
        cx: std::pin::Pin<&mut jellyhaj_core::context::TuiContext>,
    ) -> Self::Widget {
        RefreshWidget {
            inner: self.inner.into_widget(cx),
            jellyfin: self.jellyfin,
            id: self.id,
        }
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        Ok(match self.inner.apply_action(cx, action)? {
            None => None,
            Some(ControlFlow::Break(b)) => Some(b),
            Some(ControlFlow::Continue(ControlFlow::Break(b))) => Some(b),
            Some(ControlFlow::Continue(ControlFlow::Continue(FormResult::Submit))) => {
                let jellyfin = self.jellyfin.clone();
                let id = self.id.clone();
                let query = self.inner.inner.data.to_query();
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
}

fn map(
    this: &RefreshWidget,
    res: Result<Option<ControlFlow<Navigation, ControlFlow<Navigation, FormResult>>>>,
    task: &Spawner,
) -> Result<Option<Navigation>> {
    Ok(match res? {
        None => None,
        Some(ControlFlow::Break(b)) => Some(b),
        Some(ControlFlow::Continue(ControlFlow::Break(b))) => Some(b),
        Some(ControlFlow::Continue(ControlFlow::Continue(FormResult::Submit))) => {
            let jellyfin = this.jellyfin.clone();
            let id = this.id.clone();
            let query = this.inner.inner.data.to_query();
            task.spawn_res(
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

impl JellyhajWidget for RefreshWidget {
    type Action = <InnerWidget as JellyhajWidget>::Action;

    type ActionResult = Navigation;

    type State = RefreshState;

    fn min_width(&self) -> Option<u16> {
        self.inner.min_width()
    }

    fn min_height(&self) -> Option<u16> {
        self.inner.min_height()
    }

    fn into_state(self) -> Self::State {
        RefreshState {
            inner: self.inner.into_state(),
            jellyfin: self.jellyfin,
            id: self.id,
        }
    }

    fn accepts_text_input(&self) -> bool {
        self.inner.accepts_text_input()
    }

    fn accept_char(&mut self, text: char) {
        self.inner.accept_char(text);
    }

    fn accept_text(&mut self, text: String) {
        self.inner.accept_text(text);
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        let res = self.inner.apply_action(cx, action);
        map(self, res, &cx.submitter)
    }

    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
        position: jellyhaj_widgets_core::Position,
        size: jellyhaj_widgets_core::Size,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        let res = self.inner.click(cx, position, size, kind, modifier);
        map(self, res, &cx.submitter)
    }

    fn render_fallible_inner(
        &mut self,
        area: jellyhaj_widgets_core::Rect,
        buf: &mut jellyhaj_widgets_core::Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>>,
    ) -> Result<()> {
        self.inner.render_fallible_inner(area, buf, cx)
    }
}
