use std::{cmp::min, convert::Infallible, time::Duration};

use futures_util::stream::unfold;
use jellyhaj_widgets_core::{JellyhajWidget, Rect, Wrapper, async_task::TaskSubmitter};
use ratatui::widgets::{Block, BorderType, Widget};
use tokio::time::interval;
use tracing::{info_span, instrument};

pub struct Loading<'s> {
    title: &'s str,
    timeout: u8,
    lines: Vec<u16>,
    spawned: bool,
}

impl Loading<'_> {
    pub fn new<'s>(title: &'s str) -> Loading<'s> {
        Loading {
            title,
            timeout: 0,
            lines: Vec::new(),
            spawned: false,
        }
    }
}

const TIMEOUT_CYCLE: u8 = 2;
const BORDERS: ratatui::widgets::BorderType = BorderType::Thick;
const TICK_INTERVAL: Duration = Duration::from_millis(200);

#[derive(Debug)]
pub struct AdvanceLoadingScreen;

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
        for v in &mut self.lines {
            *v += 1;
        }
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
            let timer = unfold(interval(TICK_INTERVAL), |mut interval| async {
                interval.tick().await;
                Some((Ok(AdvanceLoadingScreen), interval))
            });
            task.spawn_stream(timer, info_span!("update"))
        }
        let outer = Block::bordered().title(self.title);
        let main = outer.inner(area);
        let max_size = (min(main.width, main.height) - 1) / 2;
        let width_rem = main.width - (max_size * 2);
        let height_rem = main.height - (max_size * 2);
        self.lines.retain(|s| *s <= max_size);
        for size in self.lines.iter().copied() {
            if size == 0 {
                if width_rem == 1 && height_rem == 1 {
                    buf[(main.x + max_size, main.y + max_size)].set_char('█');
                } else if width_rem == 1 {
                    buf[(main.x + max_size, main.y + max_size)].set_symbol("╻");
                    buf[(main.x + max_size, main.y + max_size + height_rem - 1)].set_symbol("╹");
                    for p in (Rect {
                        x: main.x + max_size,
                        y: main.y + max_size + 1,
                        width: 1,
                        height: height_rem - 2,
                    }
                    .positions())
                    {
                        buf[p].set_symbol("┃");
                    }
                } else if height_rem == 1 {
                    buf[(main.x + max_size, main.y + max_size)].set_symbol("╺");
                    buf[(main.x + max_size + width_rem - 1, main.y + max_size)].set_symbol("╸");
                    for p in (Rect {
                        x: main.x + max_size + 1,
                        y: main.y + max_size,
                        width: width_rem - 2,
                        height: 1,
                    }
                    .positions())
                    {
                        buf[p].set_symbol("━");
                    }
                } else {
                    let a = Rect {
                        x: main.x + max_size,
                        y: main.y + max_size,
                        width: width_rem,
                        height: height_rem,
                    };
                    Block::bordered().border_type(BORDERS).render(a, buf);
                }
            } else {
                let off = max_size - size;
                let a = Rect {
                    x: main.x + off,
                    y: main.y + off,
                    width: size * 2 + width_rem,
                    height: size * 2 + height_rem,
                };
                Block::bordered().border_type(BORDERS).render(a, buf);
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
