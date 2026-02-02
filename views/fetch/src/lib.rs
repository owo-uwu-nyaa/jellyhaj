use color_eyre::Result;
use jellyhaj_core::{keybinds::LoadingCommand, state::Navigation};
use jellyhaj_keybinds_widget::{CommandAction, KeybindWidget, MappedCommand};
use jellyhaj_loading_widget::Loading;
use jellyhaj_render_widgets::TermExt;
use keybinds::{BindingMap, KeybindEvents};
use ratatui::DefaultTerminal;
use spawn::Spawner;
use tokio::select;

struct QuitAction;

pub struct Quit {
    exit: bool,
}

pub async fn render_fetch(
    title: &str,
    events: &mut KeybindEvents,
    keybinds: BindingMap<LoadingCommand>,
    term: &mut DefaultTerminal,
    help_prefixes: &[String],
    spawner: Spawner,
) -> Result<Quit> {
    let mut widget = KeybindWidget::new(
        Loading::new(title),
        help_prefixes,
        keybinds,
        |LoadingCommand::Quit| MappedCommand::Up(QuitAction),
    );
    Ok(match term.render(&mut widget, events, spawner).await? {
        CommandAction::Up(QuitAction) => Quit { exit: false },
        CommandAction::Exit => Quit { exit: true },
    })
}

pub async fn render_fetch_future(
    title: &str,
    fetch: impl Future<Output = Result<Navigation>>,
    events: &mut KeybindEvents,
    keybinds: BindingMap<LoadingCommand>,
    term: &mut DefaultTerminal,
    help_prefixes: &[String],
    spawner: Spawner,
) -> Result<Navigation> {
    select! {
        v = fetch => v,
        v = render_fetch(title, events, keybinds, term, help_prefixes, spawner) => {
            Ok(if v?.exit {Navigation::Exit} else {Navigation::PopContext})}
    }
}
