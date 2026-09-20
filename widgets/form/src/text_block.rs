use std::{borrow::Cow, cmp::min, convert::Infallible, fmt::Debug, ops::ControlFlow, sync::Arc};

use jellyhaj_core::state::{Navigation, NextScreen};
use jellyhaj_widgets_core::{Position, Rect, RenderFlag, Result, WidgetContext, Wrapper};
use ratatui::{
    prelude::Buffer,
    widgets::{
        Block, BorderType, Padding, Scrollbar, ScrollbarOrientation, ScrollbarState,
        StatefulWidget, Widget, WidgetRef,
    },
};
use valuable::Valuable;

use crate::{FormAction, FormItem, FormItemBase};

#[must_use]
#[derive(Debug, Valuable)]
pub struct TextBlock {
    pub text: String,
    split: Vec<String>,
    width: u16,
    title: &'static str,
}

impl TextBlock {
    pub const fn new(text: String) -> Self {
        Self {
            text,
            split: vec![],
            width: 0,
            title: "",
        }
    }
}

impl<AR: From<Infallible> + Debug> FormItemBase<AR> for TextBlock {
    type SelectionInner = Option<usize>;

    type Ret = Infallible;

    type Action = String;

    fn height(&self) -> u16 {
        12
    }

    fn height_buf(&self) -> u16 {
        0
    }

    fn accepts_movement_action(&self, sel: &Self::SelectionInner) -> bool {
        sel.is_some()
    }

    fn popup_area(
        &self,
        sel: &Self::SelectionInner,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Size,
    ) -> ratatui::prelude::Rect {
        Rect::ZERO
    }
}

fn render_lines(main: Rect, height: u16, lines: &[String], buf: &mut Buffer) {
    for (y, line) in (0..height).into_iter().zip(lines) {
        let mut area = main;
        area.height = 1;
        area.y += y;
        line.render_ref(area, buf);
    }
}

impl<R: 'static, AR: From<Infallible> + Debug> FormItem<R, AR> for TextBlock {
    fn apply_movement(
        &mut self,
        sel: &mut Self::SelectionInner,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: FormAction<Infallible>,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<ControlFlow<Navigation, Self::Ret>>> {
        if let Some(pos) = sel {
            match action {
                FormAction::Quit => {
                    render_flag.set();
                    *sel = None;
                }
                FormAction::Up => {
                    render_flag.set();
                    *pos = pos.saturating_sub(1);
                }
                FormAction::Down => {
                    render_flag.set();
                    *pos = pos.saturating_add(1);
                }
                FormAction::Left | FormAction::Right | FormAction::Delete => {}
                FormAction::Enter => {
                    return Ok(Some(ControlFlow::Break(Navigation::Push(
                        NextScreen::Editor {
                            title: self.title.into(),
                            text: self.text.clone(),
                            res: Arc::new(cx.submitter.erased()),
                        },
                    ))));
                }
                FormAction::Inner(v) => match v {},
            }
        } else if matches!(action, FormAction::Enter) {
            *sel = Some(0);
            render_flag.set();
        }
        Ok(None)
    }

    fn apply_action(
        &mut self,
        _cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<ControlFlow<Navigation, Self::Ret>>> {
        self.text = action;
        render_flag.set();
        Ok(None)
    }

    fn apply_click_active(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        sel: &mut Self::SelectionInner,
        area: Rect,
        full_area: ratatui::prelude::Size,
        pos: Position,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<ControlFlow<Navigation, Self::Ret>>> {
        Ok(None)
    }

    fn apply_click_inactive(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        size: ratatui::prelude::Size,
        pos: Position,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
        render_flag: &mut RenderFlag,
    ) -> Result<(
        Option<Self::SelectionInner>,
        Option<ControlFlow<Navigation, Self::Ret>>,
    )> {
        if kind.is_down() {
            render_flag.set();
            Ok((Some(Some(0)), None))
        } else {
            Ok((None, None))
        }
    }

    fn render_pass_main(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        area: Rect,
        buf: &mut Buffer,
        active: bool,
        name: &'static str,
    ) -> Result<()> {
        let mut block = Block::bordered().padding(Padding::uniform(1));
        if active {
            block = block.border_type(BorderType::Double);
        }
        let main = block.inner(area);
        if self.width != main.width {
            self.width = main.width;
            self.split = textwrap::wrap(&self.text, usize::from(main.width))
                .into_iter()
                .map(Cow::into_owned)
                .collect();
        }

        if self.split.len() > main.height.into() {
            render_lines(main, main.height - 1, &self.split, buf);
            let mut area = main;
            area.height = 1;
            area.y += area.height - 1;
            "…".render(area, buf);
        } else {
            render_lines(main, main.height, &self.split, buf);
        }

        Ok(())
    }

    fn render_pass_popup(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        mut area: Rect,
        full_area: Rect,
        buf: &mut Buffer,
        name: &'static str,
        sel: &mut Self::SelectionInner,
        cursor: &mut Option<jellyhaj_widgets_core::Cursor>,
    ) -> Result<()> {
        if let Some(sel) = sel {
            let outer = area;
            area.height -= 4;
            area.width -= 4;
            area.x += 2;
            area.y += 2;
            let max = self.split.len().saturating_sub(area.height.into());
            *sel = min(*sel, max);
            render_lines(area, area.height, &self.split[*sel..], buf);
            if max > 1 {
                Scrollbar::new(ScrollbarOrientation::VerticalRight).render(
                    outer,
                    buf,
                    &mut ScrollbarState::new(max).position(*sel),
                );
            }
        }
        Ok(())
    }
}
