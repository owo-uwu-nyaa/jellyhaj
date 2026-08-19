pub mod erased;
pub mod list;
pub mod shaded;
pub mod state;
pub mod suspended;

use std::sync::Arc;

pub use erased::*;

use color_eyre::Report;
use futures_intrusive::sync::ManualResetEvent;
use jellyhaj_widgets_core::TreeVisitor;
use tracing::debug;

use crate::{
    state::{Navigation, NextScreen},
    widgets::shaded::widget::Erased,
};

pub use jellyhaj_widgets_core::KeybindAction;

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
    Cont(Erased),
    Empty,
    Exit,
}

struct DropGuard {
    inner: Arc<ManualResetEvent>,
}

impl Drop for DropGuard {
    fn drop(&mut self) {
        debug!("returning suspended widget");
        self.inner.set();
    }
}

type Visitor = Box<dyn FnOnce(&dyn Fn(&mut dyn TreeVisitor)) + Send + Sync>;

pub type WidgetCreator = Arc<dyn Fn(NextScreen) -> Erased + Send + Sync>;
