pub mod async_task;
pub mod flatten;
mod item;
mod jellyhaj;
pub mod mapper;
pub mod outer;
pub use color_eyre::Result;
pub use config::Config;
pub use item::ItemWidget;
pub use jellyhaj::{
    DimensionsParameter, JellyhajWidget, JellyhajWidgetExt, JellyhajWidgetState, TreeVisitor,
    WidgetTreeVisitor, Wrapper,
};
pub use jellyhaj_context::TuiContext;
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
