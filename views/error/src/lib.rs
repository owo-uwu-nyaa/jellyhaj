use color_eyre::eyre::{Report, Result};
use jellyhaj_core::{
    context::{DefaultTerminal, KeybindEvents, Spawner},
    keybinds::{ErrorCommand, Keybinds},
    state::{Navigation, NextScreen},
};
use jellyhaj_error_widget::{ErrorAction, ErrorWidget};
use jellyhaj_keybinds_widget::{CommandAction, KeybindWidget, MappedCommand};
use jellyhaj_render_widgets::TermExt;

enum Pass {
    Quit,
    Kill,
    Logs,
}

pub trait ResultDisplayExt<T> {
    fn render_error(
        self,
        term: &mut DefaultTerminal,
        events: &mut KeybindEvents,
        keybinds: &Keybinds,
        help_prefixes: &[String],
        spawn: Spawner,
    ) -> impl Future<Output = Option<T>>;
}

impl<T> ResultDisplayExt<T> for Result<T> {
    async fn render_error(
        self,
        term: &mut DefaultTerminal,
        events: &mut KeybindEvents,
        keybinds: &Keybinds,
        help_prefixes: &[String],
        spawn: Spawner,
    ) -> Option<T> {
        match self {
            Err(e) => {
                if let Some(e) = render_error(term, events, keybinds, help_prefixes, spawn, e)
                    .await
                    .err()
                {
                    tracing::error!("Error displaying error: {e:?}");
                }
                None
            }
            Ok(v) => Some(v),
        }
    }
}

pub async fn render_error(
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    keybinds: &Keybinds,
    help_prefixes: &[String],
    spawn: Spawner,
    e: Report,
) -> Result<Navigation> {
    tracing::error!("Error encountered: {e:?}");
    let mut widget = KeybindWidget::new(
        ErrorWidget::new(format!("{e:?}")),
        help_prefixes,
        keybinds.error.clone(),
        |command| match command {
            ErrorCommand::Quit => MappedCommand::Up(Pass::Quit),
            ErrorCommand::Kill => MappedCommand::Up(Pass::Kill),
            ErrorCommand::Up => MappedCommand::Down(ErrorAction::Up),
            ErrorCommand::Down => MappedCommand::Down(ErrorAction::Down),
            ErrorCommand::Left => MappedCommand::Down(ErrorAction::Left),
            ErrorCommand::Right => MappedCommand::Down(ErrorAction::Right),
            ErrorCommand::ShowLogs => MappedCommand::Up(Pass::Logs),
        },
    );
    Ok(match term.render(&mut widget, events, spawn).await? {
        CommandAction::Up(Pass::Quit) => Navigation::PopContext,
        CommandAction::Up(Pass::Kill) => Navigation::Exit,
        CommandAction::Up(Pass::Logs) => Navigation::Push {
            current: NextScreen::Error(e),
            next: NextScreen::Logs,
        },
        CommandAction::Exit => Navigation::Exit,
    })
}
