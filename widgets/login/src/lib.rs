use std::time::Duration;
use std::{convert::Infallible, ops::ControlFlow};

use color_eyre::Report;
use jellyhaj_core::Config;
use jellyhaj_core::{keybinds::FormCommand, state::Navigation};
use jellyhaj_form_widget::button::Button;
use jellyhaj_form_widget::form::{FormData, FormResultMapper};
use jellyhaj_form_widget::form_widget;
use jellyhaj_form_widget::label::Label;
use jellyhaj_form_widget::{
    form::FormCommandMapper, label_block::LabelBlock, secret_field::SecretField,
    text_field::TextField,
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::JellyhajWidget;
use jellyhaj_widgets_core::flatten::FlattenWidget;
use jellyhaj_widgets_core::spawn::tracing::{info, info_span};
use jellyhaj_widgets_core::valuable::Valuable;
use jellyhaj_widgets_core::{
    ContextRef, KeyModifiers, MouseEventKind, Result, WidgetContext, Wrapper,
};
use ratatui::layout::Constraint;
use ratatui::prelude::{Buffer, Position, Rect, Size};
use ratatui::widgets::{Block, Padding, Widget};

#[derive(Debug, Clone, Copy)]
pub enum ButtonAction {
    Submit,
    QuickConnect,
}

impl From<Infallible> for ButtonAction {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

pub struct LoginResultMapper;

impl<R: 'static> FormResultMapper<R, LoginData> for LoginResultMapper {
    type Res = LoginType;

    fn map(
        state: &LoginData,
        form_result: <LoginData as jellyhaj_form_widget::form::FormDataTypes>::AR,
        _cx: WidgetContext<
            '_,
            <LoginData as jellyhaj_form_widget::form::FormDataTypes>::Action,
            impl Wrapper<<LoginData as jellyhaj_form_widget::form::FormDataTypes>::Action>,
            R,
        >,
    ) -> Result<Option<Self::Res>> {
        Ok(Some(match form_result {
            ButtonAction::Submit => LoginType::Password {
                server_url: state.server_url.text.clone(),
                username: state.username.text.clone(),
                password: state.password.secret.clone(),
            },
            ButtonAction::QuickConnect => LoginType::QuickConnect {
                server_url: state.server_url.text.clone(),
            },
        }))
    }
}

#[form_widget(
    "Enter Jellyfin Server / Login Information",
    ButtonAction,
    LoginResultMapper
)]
#[derive(Debug, Valuable)]
pub struct LoginData {
    #[descr("Jellyfin URL")]
    server_url: TextField,
    #[descr("Username")]
    username: TextField,
    #[descr("Password")]
    #[show_if(!self.password_cmd)]
    password: SecretField,
    #[descr("Password already set through command in login config")]
    #[show_if(self.password_cmd)]
    password_set: Label,
    #[skip]
    password_cmd: bool,
    #[descr("Login")]
    submit: Button<ButtonAction>,
    #[descr("Login with Quick Connect")]
    quick_connect: Button<ButtonAction>,
    #[descr("Error")]
    error: LabelBlock,
}

type InnerWidget =
    FlattenWidget<KeybindWidget<FormCommand, LoginDataWidget, FormCommandMapper<LoginDataAction>>>;

#[derive(Valuable)]
pub struct LoginWidget {
    #[valuable(skip)]
    inner: InnerWidget,
}

#[derive(Debug)]
pub enum LoginType {
    Password {
        server_url: String,
        username: String,
        password: String,
    },
    QuickConnect {
        server_url: String,
    },
}

impl LoginType {
    pub fn get_server_url(&self) -> &str {
        match self {
            LoginType::Password {
                server_url,
                username: _,
                password: _,
            } => server_url,
            LoginType::QuickConnect { server_url } => server_url,
        }
    }
}

impl<R: ContextRef<Config> + 'static> JellyhajWidget<R> for LoginWidget {
    type Action = <InnerWidget as JellyhajWidget<R>>::Action;

    type ActionResult = ControlFlow<Navigation, LoginType>;

    const NAME: &str = "login";

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
        self.inner.apply_action(cx, action)
    }

    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        position: Position,
        size: Size,
        kind: MouseEventKind,
        modifier: KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        self.inner.click(cx, position, size, kind, modifier)
    }

    fn render_fallible_inner(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()> {
        self.inner.render_fallible_inner(area, buf, cx)
    }
}

impl LoginWidget {
    pub fn new(
        server_url: String,
        username: String,
        password: String,
        password_cmd_set: bool,
        error: Report,
        c: &Config,
    ) -> Self {
        let selection = if server_url.is_empty() {
            LoginDataSelection::ServerUrl(())
        } else if username.is_empty() {
            LoginDataSelection::Username(())
        } else if !password_cmd_set && password.is_empty() {
            LoginDataSelection::Password(())
        } else {
            LoginDataSelection::Submit(())
        };
        LoginWidget {
            inner: FlattenWidget::new(KeybindWidget::new(
                LoginData {
                    server_url: TextField::new(server_url),
                    username: TextField::new(username),
                    password: SecretField::new(password),
                    password_set: Default::default(),
                    password_cmd: password_cmd_set,
                    submit: Button::new(ButtonAction::Submit),
                    quick_connect: Button::new(ButtonAction::QuickConnect),
                    error: LabelBlock::new(format!("{error:?}")),
                }
                .make_with(selection),
                c.keybinds.form.clone(),
                Default::default(),
            )),
        }
    }
}

#[derive(Valuable)]
pub struct QuickConnectWidget {
    code: String,
    position: u8,
}

impl QuickConnectWidget {
    pub fn new(code: String) -> Self {
        Self { code, position: 0 }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum QuickConectAction {
    Clock,
    Quit,
}

#[derive(Debug, Clone, Copy)]
pub struct Quit;

const TICK_INTERVAL: Duration = Duration::from_millis(200);
const CANCEL_STR: &str = "Cancel";

impl<R: 'static> JellyhajWidget<R> for QuickConnectWidget {
    type Action = QuickConectAction;

    type ActionResult = Quit;

    const NAME: &str = "quick-connect";

    fn visit_children(&self, _: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {}

    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {
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

    fn min_width(&self) -> Option<u16> {
        Some(33)
    }

    fn min_height(&self) -> Option<u16> {
        Some(9)
    }

    fn accepts_text_input(&self) -> bool {
        false
    }

    fn accept_char(&mut self, _text: char) {}

    fn accept_text(&mut self, _text: String) {}

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
            QuickConectAction::Quit => Ok(Some(Quit)),
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
                Constraint::Length(CANCEL_STR.len() as u16 + 2),
                Constraint::Length(5),
            );
            area.y += 2;
            area.height -= 2;
            area.contains(position)
        } {
            Ok(Some(Quit))
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
        let mut text_area = main.centered_horizontally(Constraint::Length(text.len() as u16));
        text_area.height = 1;
        info!("text_area: {text_area:?}");
        text.render(text_area, buf);
        main.y += 2;
        main.height -= 2;
        main = main.centered_horizontally(Constraint::Length(CANCEL_STR.len() as u16 + 2));
        let cancel_block = Block::bordered();
        CANCEL_STR.render(cancel_block.inner(main), buf);
        cancel_block.render(main, buf);
        block.render(area, buf);
        Ok(())
    }
}
