use std::{cmp::max, convert::Infallible, sync::atomic::Ordering::Relaxed, time::Duration};

use futures_util::stream::unfold;
use jellyhaj_widgets_core::{ContextRef, GetFromContext, JellyhajWidget, WidgetContext, Wrapper};
use ratatui::{
    layout::Constraint,
    symbols::merge::MergeStrategy,
    widgets::{Block, Padding, Widget},
};
use stats_data::StatsData;
use tokio::time::interval;
use tracing::{info_span, instrument};
use valuable::Valuable;

struct BorderedTable<'r> {
    rows: &'r [&'r [&'r str]],
    col_widths: &'r [u16],
}

const IMAGE_FETCHES: &str = "Image fetches";
const DB_IMAGE_CACHE_HITS: &str = "DB image cache hits";
const MEMORY_IMAGE_CACHE_HITS: &str = "In memory image cache hits";

const fn max_c(v1: usize, v2: usize) -> usize {
    let res = if v1 > v2 { v1 } else { v2 };
    assert!(res <= u16::MAX as usize);
    res
}

#[allow(clippy::cast_possible_truncation)]
const LABEL_MAX_LEN: u16 = max_c(
    IMAGE_FETCHES.len(),
    max_c(DB_IMAGE_CACHE_HITS.len(), MEMORY_IMAGE_CACHE_HITS.len()),
) as u16;

impl<'r> BorderedTable<'r> {
    const fn new(rows: &'r [&'r [&'r str]], col_widths: &'r [u16]) -> Self {
        Self { rows, col_widths }
    }
}

impl Widget for &BorderedTable<'_> {
    fn render(self, mut area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let block = Block::bordered()
            .padding(Padding::horizontal(1))
            .merge_borders(MergeStrategy::Exact);
        for row in self.rows {
            let mut row_area = area;
            row_area.height = 3;
            area.y = area.y.strict_add(2);
            area.height = area.height.strict_sub(2);
            assert_eq!(
                row.len(),
                self.col_widths.len(),
                "mismatch in the number of colums"
            );
            for (cell, width) in row.iter().zip(self.col_widths) {
                let width = width.strict_add(4);
                let mut cell_area = row_area;
                cell_area.width = width;
                row_area.x = row_area.x.strict_add(width - 1);
                row_area.width = row_area.width.strict_sub(width - 1);
                cell.render(block.inner(cell_area), buf);
                (&block).render(cell_area, buf);
            }
        }
    }
}

#[derive(Valuable)]
pub struct StatsWidget {
    image_fetches: String,
    db_image_cache_hits: String,
    memory_image_cache_hits: String,
}

impl StatsWidget {
    pub fn new(stats: &StatsData) -> Self {
        Self {
            image_fetches: stats.image_fetches.load(Relaxed).to_string(),
            db_image_cache_hits: stats.db_image_cache_hits.load(Relaxed).to_string(),
            memory_image_cache_hits: stats.memory_image_cache_hits.load(Relaxed).to_string(),
        }
    }
}

#[derive(Debug)]
pub struct StatsUpdate;

impl<R: 'static + ContextRef<StatsData>> JellyhajWidget<R> for StatsWidget {
    type Action = StatsUpdate;

    type ActionResult = Infallible;

    const NAME: &str = "stats";

    fn visit_children(&self, _visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {}

    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {
        cx.submitter.spawn_stream(
            unfold(interval(Duration::from_secs(5)), |mut i| async move {
                i.tick().await;
                Some((Ok(StatsUpdate), i))
            }),
            info_span!("update_stats_tick"),
            "update_stats_tick",
        );
    }

    fn min_width(&self) -> Option<u16> {
        Some(
            LABEL_MAX_LEN
                .strict_add(
                    max(
                        self.image_fetches.len(),
                        max(
                            self.db_image_cache_hits.len(),
                            self.memory_image_cache_hits.len(),
                        ),
                    )
                    .try_into()
                    .expect("overflow"),
                )
                .strict_add(11),
        )
    }

    fn min_height(&self) -> Option<u16> {
        Some(11)
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

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        _: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        let stats = StatsData::get_ref(cx.refs);
        self.image_fetches = stats.image_fetches.load(Relaxed).to_string();
        self.db_image_cache_hits = stats.db_image_cache_hits.load(Relaxed).to_string();
        self.memory_image_cache_hits = stats.memory_image_cache_hits.load(Relaxed).to_string();
        Ok(None)
    }

    fn click(
        &mut self,
        _: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        _: ratatui::prelude::Position,
        _: ratatui::prelude::Size,
        _: ratatui::crossterm::event::MouseEventKind,
        _: ratatui::crossterm::event::KeyModifiers,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        Ok(None)
    }

    #[instrument(name = "render_stats", skip_all)]
    fn render_fallible_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        _cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> jellyhaj_widgets_core::Result<()> {
        let block = Block::bordered().title("Program stats");
        let memory_image_cache_hits = [MEMORY_IMAGE_CACHE_HITS, &self.memory_image_cache_hits];
        let db_image_cache_hits = [DB_IMAGE_CACHE_HITS, &self.db_image_cache_hits];
        let image_fetches = [IMAGE_FETCHES, &self.image_fetches];
        let rows: [&[_]; _] = [
            &memory_image_cache_hits,
            &db_image_cache_hits,
            &image_fetches,
        ];
        let col = max(
            self.image_fetches.len(),
            max(
                self.db_image_cache_hits.len(),
                self.memory_image_cache_hits.len(),
            ),
        );
        let cols = [LABEL_MAX_LEN, col.try_into()?];
        let table = BorderedTable::new(&rows, &cols);
        let table_area = block.inner(area).centered(
            Constraint::Length(<Self as JellyhajWidget<R>>::min_width(self).unwrap()),
            Constraint::Length(<Self as JellyhajWidget<R>>::min_height(self).unwrap()),
        );
        table.render(table_area, buf);
        block.render(area, buf);
        Ok(())
    }
}
