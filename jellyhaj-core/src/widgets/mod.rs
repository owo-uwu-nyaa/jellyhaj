pub mod erased;
pub mod list;
pub mod shaded;
pub mod state;
pub mod suspended;

use std::{fmt::Debug, sync::Arc};

pub use erased::*;

use color_eyre::Report;
use futures_intrusive::sync::ManualResetEvent;
use jellyhaj_widgets_core::TreeVisitor;
use ratatui::crossterm::event::KeyEvent;

use crate::{
    state::{Navigation, NextScreen},
    widgets::shaded::widget::ShadedWidget,
};

#[derive(Debug)]
pub enum KeybindAction<A: Debug + Send + 'static> {
    Inner(A),
    Key(KeyEvent),
}

pub enum WidgetResult<T> {
    Ok(T),
    Err(Report),
    Pop,
    Exit,
}

impl From<WidgetResult<Self>> for Navigation {
    fn from(value: WidgetResult<Self>) -> Self {
        match value {
            WidgetResult::Ok(v) => v,
            WidgetResult::Err(report) => Self::Replace(NextScreen::Error(report)),
            WidgetResult::Pop => Self::PopContext,
            WidgetResult::Exit => Self::Exit,
        }
    }
}

pub enum RunResult {
    Cont(ShadedErased),
    Empty,
    Exit,
}

struct DropGuard {
    inner: Arc<ManualResetEvent>,
}

impl Drop for DropGuard {
    fn drop(&mut self) {
        self.inner.set();
    }
}

type Visitor = Box<dyn FnOnce(&dyn Fn(&mut dyn TreeVisitor)) + Send + Sync>;

pub type ShadedErased = Box<ShadedWidget<Navigation>>;

pub type WidgetCreator = Arc<dyn Fn(NextScreen) -> ShadedErased + Send + Sync>;
