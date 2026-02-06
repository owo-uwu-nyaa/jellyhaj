use ratatui::widgets::Block;

pub mod offset;
pub mod size;
pub mod with_current;

pub fn outer_block(name: &'static str) -> Block<'static> {
    Block::bordered().title(name)
}

pub mod exports {
    pub use color_eyre::Result;
    pub use jellyhaj_widgets_core::{JellyhajWidget, Wrapper, async_task::TaskSubmitter};
    pub use ratatui::{
        buffer::Buffer,
        crossterm::event::{KeyModifiers, MouseEventKind},
        layout::{Position, Rect, Size},
        widgets::{StatefulWidget, Widget, Block},
    };
    pub use std::{
        cmp::min,
        default::Default,
        matches,
        option::Option::{self, None, Some},
        string::String,
        panic,
        convert::From,
        assert,
    };
    pub use tui_scrollview::{ScrollView, ScrollViewState, ScrollbarVisibility};
}
