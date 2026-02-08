use std::pin::Pin;

use color_eyre::Result;
use jellyhaj_core::{context::TuiContext, keybinds::StatsCommand, state::Navigation};
use jellyhaj_keybinds_widget::{CommandAction, KeybindWidget, MappedCommand};
use jellyhaj_render_widgets::TermExt;
use jellyhaj_stats_widget::StatsWidget;

#[derive(Debug)]
struct QuitAction;

pub async fn render_stats(cx: Pin<&mut TuiContext>) -> Result<Navigation> {
    let cx = cx.project();

    let mut widget = KeybindWidget::new(
        StatsWidget::new(cx.stats.clone()),
        &cx.config.help_prefixes,
        cx.config.keybinds.stats.clone(),
        |StatsCommand::Quit| MappedCommand::Up(QuitAction),
    );

    Ok(
        match cx
            .term
            .render(&mut widget, cx.events, cx.spawn.clone())
            .await?
        {
            CommandAction::Up(QuitAction) => Navigation::PopContext,
            CommandAction::Exit => Navigation::Exit,
        },
    )
}
