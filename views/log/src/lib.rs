use std::pin::Pin;

use color_eyre::Result;
use jellyhaj_core::{context::TuiContext, keybinds::LoggerCommand, state::Navigation};
use jellyhaj_keybinds_widget::{CommandAction, KeybindWidget, MappedCommand};
use jellyhaj_log_widget::{LogWidget, TuiWidgetEvent};
use jellyhaj_render_widgets::TermExt;

struct Quit;

pub async fn render_log(cx: Pin<&mut TuiContext>) -> Result<Navigation> {
    let cx = cx.project();
    let mut widget = KeybindWidget::new(
        LogWidget::new(),
        &cx.config.help_prefixes,
        cx.config.keybinds.logger.clone(),
        |c| match c {
            LoggerCommand::Space => MappedCommand::Down(TuiWidgetEvent::SpaceKey),
            LoggerCommand::TargetUp => MappedCommand::Down(TuiWidgetEvent::UpKey),
            LoggerCommand::TargetDown => MappedCommand::Down(TuiWidgetEvent::DownKey),
            LoggerCommand::Left => MappedCommand::Down(TuiWidgetEvent::LeftKey),
            LoggerCommand::Right => MappedCommand::Down(TuiWidgetEvent::RightKey),
            LoggerCommand::Plus => MappedCommand::Down(TuiWidgetEvent::PlusKey),
            LoggerCommand::Minus => MappedCommand::Down(TuiWidgetEvent::MinusKey),
            LoggerCommand::Hide => MappedCommand::Down(TuiWidgetEvent::HideKey),
            LoggerCommand::Focus => MappedCommand::Down(TuiWidgetEvent::FocusKey),
            LoggerCommand::MessagesUp => MappedCommand::Down(TuiWidgetEvent::PrevPageKey),
            LoggerCommand::MessagesDown => MappedCommand::Down(TuiWidgetEvent::NextPageKey),
            LoggerCommand::Escape => MappedCommand::Down(TuiWidgetEvent::EscapeKey),
            LoggerCommand::Quit => MappedCommand::Up(Quit),
        },
    );
    Ok(
        match cx
            .term
            .render(&mut widget, cx.events, cx.spawn.clone())
            .await?
        {
            CommandAction::Up(Quit) => Navigation::PopContext,
            CommandAction::Exit => Navigation::Exit,
        },
    )
}
