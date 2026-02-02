use std::{
    cmp::{max, min},
    convert::Infallible,
    time::Duration,
};

use futures_util::stream::unfold;
use jellyhaj_widgets_core::{JellyhajWidget, Wrapper, async_task::TaskSubmitter};
use ratatui::widgets::{Block, Widget};
use tokio::time::interval;
use tracing::{info_span, instrument};

pub struct Loading<'s> {
    title: &'s str,
    timeout: u8,
    lines: Vec<u16>,
    max: u16,
    spawned: bool,
}

impl Loading<'_> {
    pub fn new<'s>(title: &'s str) -> Loading<'s> {
        Loading {
            title,
            timeout: 0,
            lines: Vec::new(),
            max: 0,
            spawned: false,
        }
    }
}

const TIMEOUT_CYCLE: u8 = 4;

pub struct AdvanceLoadingScreen;

fn draw_line_down(x: u16, y_start: u16, y_end: u16, buf: &mut ratatui::prelude::Buffer) {
    for y in y_start..=y_end {
        buf[(x, y)].set_symbol("│");
    }
}

fn draw_line_right(y: u16, x_start: u16, x_end: u16, buf: &mut ratatui::prelude::Buffer) {
    for x in x_start..=x_end {
        buf[(x, y)].set_symbol("─");
    }
}

impl<'s> JellyhajWidget for Loading<'s> {
    type State = &'s str;
    type Action = AdvanceLoadingScreen;
    type ActionResult = Infallible;

    fn min_width(&self) -> Option<u16> {
        Some(5)
    }
    fn min_height(&self) -> Option<u16> {
        Some(5)
    }

    fn into_state(self) -> Self::State {
        self.title
    }

    fn apply_action(
        &mut self,
        _: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        _: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        self.lines.retain_mut(|line| {
            *line += 1;
            self.max >= *line
        });
        if self.timeout == 0 {
            self.lines.push(0);
        }
        self.timeout = (self.timeout + 1) % TIMEOUT_CYCLE;
        Ok(None)
    }

    fn click(
        &mut self,
        _: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        _position: ratatui::prelude::Position,
        _size: ratatui::prelude::Size,
        _kind: ratatui::crossterm::event::MouseEventKind,
        _modifier: ratatui::crossterm::event::KeyModifiers,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        Ok(None)
    }

    #[instrument(name = "render_loading", skip_all)]
    fn render_fallible_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
    ) -> jellyhaj_widgets_core::Result<()> {
        if !self.spawned {
            self.spawned = true;
            let timer = unfold(interval(Duration::from_secs(1)), |mut interval| async {
                interval.tick().await;
                Some((Ok(AdvanceLoadingScreen), interval))
            });
            task.spawn_stream(timer, info_span!("update"))
        }
        let outer = Block::bordered().title(self.title);
        let main = outer.inner(area);
        self.max = max(main.width, main.height).div_ceil(2) - 1;
        let center_x = main.x + main.width / 2;
        let center_y = main.y + main.height / 2;
        if (main.width % 2) == 1 {
            if (main.height % 2) == 1 {
                for size in self.lines.iter().copied().filter(|s| s * 2 < self.max) {
                    if size == 0 {
                        buf[(center_x, center_y)].set_symbol("█");
                    } else {
                        if main.width > size * 2 {
                            let y_start = max(main.y, center_y - size + 1);
                            let y_end = min(main.y + main.height - 1, center_y + size - 1);
                            draw_line_down(center_x - size, y_start, y_end, buf);
                            draw_line_down(center_x + size, y_start, y_end, buf);
                        }
                        if main.height > size * 2 {
                            let x_start = max(main.x, center_x - size + 1);
                            let x_end = min(main.x + main.width - 1, center_x + size - 1);
                            draw_line_right(center_y - size, x_start, x_end, buf);
                            draw_line_right(center_y + size, x_start, x_end, buf);
                        }
                        if main.width > size * 2 || main.height > size * 2 {
                            buf[(center_x - size, center_y - size)].set_symbol("┌");
                            buf[(center_x - size, center_y + size)].set_symbol("└");
                            buf[(center_x + size, center_y - size)].set_symbol("┐");
                            buf[(center_x + size, center_y + size)].set_symbol("┘");
                        }
                    }
                }
            } else {
                for size in self.lines.iter().copied().filter(|s| s * 2 < self.max) {
                    if size == 0 {
                        buf[(center_x, center_y - 1)].set_symbol("▄");
                        buf[(center_x, center_y)].set_symbol("▀");
                    } else {
                        if main.width > size * 2 {
                            let y_start = max(main.y, center_y - size);
                            let y_end = min(main.y + main.height - 1, center_y + size - 1);
                            draw_line_down(center_x - size, y_start, y_end, buf);
                            draw_line_down(center_x + size, y_start, y_end, buf);
                        }
                        if main.height > size * 2 + 1 {
                            let x_start = max(main.x, center_x - size + 1);
                            let x_end = min(main.x + main.width - 1, center_x + size - 1);
                            draw_line_right(center_y - size, x_start, x_end, buf);
                            draw_line_right(center_y + size, x_start, x_end, buf);
                        }
                        if main.width > size * 2 || main.height > size * 2 + 1 {
                            buf[(center_x - size, center_y - size - 1)].set_symbol("┌");
                            buf[(center_x - size, center_y + size)].set_symbol("└");
                            buf[(center_x + size, center_y - size - 1)].set_symbol("┐");
                            buf[(center_x + size, center_y + size)].set_symbol("┘");
                        }
                    }
                }
            }
        } else if (main.height % 2) == 1 {
            for size in self.lines.iter().copied().filter(|s| s * 2 < self.max) {
                if size == 0 {
                    buf[(center_x - 1, center_y)].set_symbol("▐");
                    buf[(center_x, center_y)].set_symbol("▌");
                } else {
                    if main.width > size * 2 + 1 {
                        let y_start = max(main.y, center_y - size + 1);
                        let y_end = min(main.y + main.height - 1, center_y + size - 1);
                        draw_line_down(center_x - size, y_start, y_end, buf);
                        draw_line_down(center_x + size, y_start, y_end, buf);
                    }
                    if main.height > size * 2 {
                        let x_start = max(main.x, center_x - size);
                        let x_end = min(main.x + main.width - 1, center_x + size - 1);
                        draw_line_right(center_y - size, x_start, x_end, buf);
                        draw_line_right(center_y + size, x_start, x_end, buf);
                    }
                    if main.width > size * 2 + 1 || main.height > size * 2 {
                        buf[(center_x - size - 1, center_y - size)].set_symbol("┌");
                        buf[(center_x - size - 1, center_y + size)].set_symbol("└");
                        buf[(center_x + size, center_y - size)].set_symbol("┐");
                        buf[(center_x + size, center_y + size)].set_symbol("┘");
                    }
                }
            }
        } else {
            for size in self.lines.iter().copied().filter(|s| s * 2 < self.max) {
                if size == 0 {
                    buf[(center_x - 1, center_y - 1)].set_symbol("▗");
                    buf[(center_x, center_y - 1)].set_symbol("▖");
                    buf[(center_x - 1, center_y)].set_symbol("▝");
                    buf[(center_x, center_y)].set_symbol("▘");
                } else {
                    if main.width > size * 2 + 1 {
                        let y_start = max(main.y, center_y - size);
                        let y_end = min(main.y + main.height - 1, center_y + size - 1);
                        draw_line_down(center_x - size, y_start, y_end, buf);
                        draw_line_down(center_x + size, y_start, y_end, buf);
                    }
                    if main.height > size * 2 + 1 {
                        let x_start = max(main.x, center_x - size);
                        let x_end = min(main.x + main.width - 1, center_x + size - 1);
                        draw_line_right(center_y - size, x_start, x_end, buf);
                        draw_line_right(center_y + size, x_start, x_end, buf);
                    }
                    if main.width > size * 2 + 1 || main.height > size * 2 + 1 {
                        buf[(center_x - size - 1, center_y - size - 1)].set_symbol("┌");
                        buf[(center_x - size - 1, center_y + size)].set_symbol("└");
                        buf[(center_x + size, center_y - size - 1)].set_symbol("┐");
                        buf[(center_x + size, center_y + size)].set_symbol("┘");
                    }
                }
            }
        }
        outer.render(area, buf);
        Ok(())
    }

    fn accepts_text_input(&self) -> bool {
        false
    }

    fn accept_char(&mut self, _: char) {
        unimplemented!()
    }

    fn accept_text(&mut self, _: String) {
        unimplemented!()
    }
}
