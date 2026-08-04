use std::{
    io::stdout,
    pin::Pin,
    task::{Context, Poll},
};

use color_eyre::Result;
use futures_util::future::BoxFuture;
use ratatui::{
    DefaultTerminal, Terminal,
    crossterm::{
        event::{
            DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
            KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
        },
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    prelude::CrosstermBackend,
};

struct TermGuard;

impl Drop for TermGuard {
    fn drop(&mut self) {
        disable_unconditional();
    }
}

pub fn disable_unconditional() {
    let _ = execute!(stdout(), DisableBracketedPaste);
    let _ = execute!(stdout(), DisableMouseCapture);
    let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
    let _ = execute!(stdout(), LeaveAlternateScreen);
    let _ = disable_raw_mode();
}

pub fn run_with(f: impl FnOnce(DefaultTerminal) -> Result<()>) -> Result<()> {
    let g = TermGuard;
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    execute!(
        stdout(),
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    )?;
    execute!(stdout(), EnableMouseCapture)?;
    execute!(stdout(), EnableBracketedPaste)?;
    let backend = CrosstermBackend::new(stdout());
    f(Terminal::new(backend)?)?;
    drop(g);
    Ok(())
}

struct DisableTermGuard;

impl Drop for DisableTermGuard {
    fn drop(&mut self) {
        let _ = enable_raw_mode();
        let _ = execute!(stdout(), EnterAlternateScreen);
        let _ = execute!(
            stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        );
        let _ = execute!(stdout(), EnableMouseCapture);
        let _ = execute!(stdout(), EnableBracketedPaste);
    }
}

pub struct RunWithout {
    guard: Option<DisableTermGuard>,
    f: BoxFuture<'static, Result<()>>,
}

impl Future for RunWithout {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.guard.is_none() {
            let g = DisableTermGuard;
            execute!(stdout(), DisableBracketedPaste)?;
            execute!(stdout(), DisableMouseCapture)?;
            execute!(stdout(), PopKeyboardEnhancementFlags)?;
            execute!(stdout(), LeaveAlternateScreen)?;
            disable_raw_mode()?;
            this.guard = Some(g);
        }
        this.f.as_mut().poll(cx)
    }
}

#[must_use]
pub fn run_without(f: BoxFuture<'static, Result<()>>) -> RunWithout {
    RunWithout { guard: None, f }
}
