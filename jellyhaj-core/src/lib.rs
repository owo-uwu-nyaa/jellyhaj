pub mod render;
pub mod state;
pub mod term;

use std::fmt::Debug;
use std::ops::ControlFlow;

use ::keybinds::Command;
pub use config::Config;
pub use config::keybind_defs as keybinds;
pub use jellyhaj_context as context;

use crate::state::Navigation;

pub trait CommandMapper<T: Command>: Send + 'static {
    type A: Debug + Send + 'static;
    fn map(&self, command: T) -> ControlFlow<Navigation, Self::A>;
}
