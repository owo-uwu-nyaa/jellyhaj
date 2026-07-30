use std::time::Duration;

use color_eyre::eyre::Context;
use jellyfin::JellyfinClient;
use jellyhaj_core::state::{ClientOut, LoginState, Navigation, NextScreen};
use jellyhaj_widgets_core::{
    Buffer, JellyhajWidget, JellyhajWidgetBase, KeyModifiers, MouseEventKind, Position, Rect,
    Result, Size, WidgetContext, Wrapper,
    spawn::tracing::{info, info_span},
};
use ratatui::{
    layout::Constraint,
    widgets::{Block, Padding, Widget},
};
use valuable::Valuable;

#[derive(Valuable)]
pub struct QuickConnectWidget<F: Future<Output = Result<QuickConectAction>>> {
    code: String,
    #[valuable(skip)]
    fut: Option<F>,
    state: LoginState,
    #[valuable(skip)]
    out: ClientOut,
    server_id: String,
    position: u8,
}

impl<F: Future<Output = Result<QuickConectAction>>> QuickConnectWidget<F> {
    #[must_use]
    pub const fn new(
        code: String,
        fut: F,
        state: LoginState,
        out: ClientOut,
        server_id: String,
    ) -> Self {
        Self {
            code,
            position: 0,
            fut: Some(fut),
            state,
            out,
            server_id,
        }
    }
}

#[derive(Debug)]
pub enum QuickConectAction {
    Clock,
    Finished { client: JellyfinClient },
    Quit,
}

const TICK_INTERVAL: Duration = Duration::from_millis(200);
const CANCEL_STR: &str = "Cancel";

impl<F: Future<Output = Result<QuickConectAction>> + Send + 'static> JellyhajWidgetBase
    for QuickConnectWidget<F>
{
    type Action = QuickConectAction;

    type ActionResult = Navigation;

    const NAME: &str = "quick-connect";

    fn visit_children(&self, _: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {}

    fn min_width(&self) -> Option<u16> {
        Some(33)
    }
    fn min_height(&self) -> Option<u16> {
        Some(9)
    }
}

impl<F: Future<Output = Result<QuickConectAction>> + Send + 'static, R: 'static> JellyhajWidget<R>
    for QuickConnectWidget<F>
{
    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {
        cx.submitter.spawn_task(
            self.fut.take().expect("was init called twice?"),
            info_span!("quick-connect-poll"),
            "quick-connect-poll",
        );
        let interval = tokio::time::interval(TICK_INTERVAL);
        cx.submitter.spawn_stream(
            futures_util::stream::unfold(interval, |mut interval| async move {
                interval.tick().await;
                Some((Ok(QuickConectAction::Clock), interval))
            }),
            info_span!("quick-connect-clock"),
            "quick-connect-clock",
        );
    }

    fn apply_action(
        &mut self,
        _cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        match action {
            QuickConectAction::Clock => {
                self.position = (self.position + 1) % 4;
                Ok(None)
            }
            QuickConectAction::Quit => Ok(Some(Navigation::PopContext)),
            QuickConectAction::Finished { client } => {
                Ok(Some(Navigation::Replace(NextScreen::AuthFinished {
                    state: self.state.clone(),
                    out: self.out.clone(),
                    client,
                    server_id: self.server_id.clone(),
                })))
            }
        }
    }

    fn click(
        &mut self,
        _cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        position: Position,
        size: Size,
        kind: MouseEventKind,
        _modifier: KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        if kind.is_down() && {
            let mut area = Rect::from((Position::ORIGIN, size)).centered(
                Constraint::Length(u16::try_from(CANCEL_STR.len()).expect("known length") + 2),
                Constraint::Length(5),
            );
            area.y += 2;
            area.height -= 2;
            area.contains(position)
        } {
            Ok(Some(Navigation::PopContext))
        } else {
            Ok(None)
        }
    }

    fn render_fallible_inner(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        _cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()> {
        info!("area: {area:?}");
        info!("rendering quick connect");
        let block = Block::bordered()
            .title("Quick Connect ")
            .padding(Padding::uniform(1));
        let mut main = block.inner(area).centered_vertically(Constraint::Length(5));
        info!("main: {main:?}");
        let spin = ['|', '/', '-', '\\'];
        let text = format!(
            "Enter code {} to login {}",
            self.code, spin[self.position as usize]
        );
        let mut text_area = main.centered_horizontally(Constraint::Length(
            text.len()
                .try_into()
                .context("text lenght conversion overflowed")?,
        ));
        text_area.height = 1;
        info!("text_area: {text_area:?}");
        text.render(text_area, buf);
        main.y += 2;
        main.height -= 2;
        main = main.centered_horizontally(Constraint::Length(
            u16::try_from(CANCEL_STR.len()).expect("known length") + 2,
        ));
        let cancel_block = Block::bordered();
        CANCEL_STR.render(cancel_block.inner(main), buf);
        cancel_block.render(main, buf);
        block.render(area, buf);
        Ok(())
    }
}
