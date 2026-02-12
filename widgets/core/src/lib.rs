pub mod async_task;
mod item;
mod jellyhaj;
pub use color_eyre::Result;
pub use config::Config;
pub use item::ItemWidget;
pub use jellyhaj::{DimensionsParameter, JellyhajWidget, JellyhajWidgetExt, Wrapper};
pub use ratatui::{
    self,
    buffer::Buffer,
    crossterm::event::{KeyModifiers, MouseEventKind},
    layout::{Position, Rect, Size},
};
pub use ratatui_image::FontSize;
pub use spawn;

pub trait RectExt {
    fn contains(self, pos: Position) -> bool;
}

impl RectExt for Rect {
    fn contains(self, pos: Position) -> bool {
        self.x <= pos.x
            && self.y <= pos.y
            && self.x + self.width > pos.x
            && self.y + self.height > pos.y
    }
}
