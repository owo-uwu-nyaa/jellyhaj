use std::{borrow::Cow, cmp::min};

use jellyhaj_core::state::Navigation;
use jellyhaj_widgets_core::{
    Cursor, JellyhajWidget, JellyhajWidgetBase, Position,
    async_task::ErasedSubmitter,
    ratatui::{
        crossterm::cursor::SetCursorStyle,
        widgets::{Block, Padding, Widget, WidgetRef},
    },
};
use tracing::{instrument, trace};
use valuable::{Fields, NamedField, NamedValues, StructDef, Structable, Valuable, Value};

#[derive(Debug)]
pub struct Editor {
    title: Cow<'static, str>,
    lines: Vec<String>,
    split: Vec<Vec<String>>,
    line: u16,
    col: u16,
    width: u16,
    res: Box<dyn ErasedSubmitter<String>>,
}
const _: () = {
    static FIELDS: &[NamedField] = &[
        NamedField::new("title"),
        NamedField::new("lines"),
        NamedField::new("split"),
        NamedField::new("line"),
        NamedField::new("col"),
        NamedField::new("width"),
    ];
    impl Valuable for Editor {
        fn as_value(&self) -> Value<'_> {
            Value::Structable(self)
        }

        fn visit(&self, visit: &mut dyn valuable::Visit) {
            visit.visit_named_fields(&NamedValues::new(
                FIELDS,
                &[
                    self.title.as_ref().as_value(),
                    self.lines.as_value(),
                    self.split.as_value(),
                    self.line.as_value(),
                    self.col.as_value(),
                    self.width.as_value(),
                ],
            ));
        }
    }
    impl Structable for Editor {
        fn definition(&self) -> StructDef<'_> {
            StructDef::new_static("Editor", Fields::Named(FIELDS))
        }
    }
};

impl Editor {
    pub fn new(
        title: Cow<'static, str>,
        text: &str,
        res: Box<dyn ErasedSubmitter<String>>,
    ) -> Self {
        let lines: Vec<String> = text.split('\n').map(ToOwned::to_owned).collect();
        let split = vec![vec![]; lines.len()];
        Self {
            title,
            lines,
            split,
            line: 0,
            col: 0,
            width: 0,
            res,
        }
    }
    fn rewrap(&mut self) {
        let index: usize = self.line.into();
        let width: usize = self.width.into();
        self.split[index] = textwrap::wrap(&self.lines[index], width)
            .into_iter()
            .map(Cow::into_owned)
            .collect();
    }
    fn rewrap_all(&mut self) {
        let width: usize = self.width.into();
        self.lines.iter().enumerate().for_each(|(index, line)| {
            self.split[index] = textwrap::wrap(line, width)
                .into_iter()
                .map(Cow::into_owned)
                .collect();
        });
    }
    fn send(&self) {
        let text = self.lines.join("\n");
        self.res.spawn_value_infallible(text);
    }
}

/// Index of `index`'th char in string.
/// If `index` is out of bounds it is adjusted to `chars.len()` and `val.str()` is returned
#[instrument(level = "trace", ret)]
fn char_index(val: &str, index: &mut u16) -> usize {
    let mut total = 0u16;
    val.char_indices()
        .inspect(|_| total = total.strict_add(1))
        .nth((*index).into())
        .map_or_else(
            || {
                // clamp max value
                *index = total;
                val.len()
            },
            |(v, _)| v,
        )
}

#[must_use]
fn chars(val: &str) -> u16 {
    val.chars().map(|_| 1u16).sum()
}

#[cfg(test)]
mod tests {
    use crate::char_index;

    #[test]
    fn char_indices() {
        let mut pos = 0u16;
        assert_eq!(0, char_index("", &mut pos));
        assert_eq!(0, pos);
        assert_eq!(0, char_index("a", &mut pos));
        assert_eq!(0, pos);
        pos = 1;
        assert_eq!(1, char_index("a", &mut pos));
        assert_eq!(1, pos);
        assert_eq!(1, char_index("ab", &mut pos));
        assert_eq!(1, pos);
        pos = 2;
        assert_eq!(1, char_index("a", &mut pos));
        assert_eq!(1, pos);
        pos = 2;
        assert_eq!(2, char_index("ab", &mut pos));
        assert_eq!(2, pos);
        pos = 3;
        assert_eq!(2, char_index("ab", &mut pos));
        assert_eq!(2, pos);
    }
}

#[derive(Debug)]
pub enum EditorAction {
    Finish,
    NewLine,
    Delete,
    Up,
    Down,
    Left,
    Right,
    Quit,
}

impl JellyhajWidgetBase for Editor {
    type Action = EditorAction;

    type ActionResult = Navigation;

    const NAME: &str = "editor";

    fn visit_children(&self, _visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {}

    fn min_width(&self) -> Option<u16> {
        Some(5)
    }

    fn min_height(&self) -> Option<u16> {
        Some(5)
    }

    fn accepts_text_input(&self) -> bool {
        true
    }

    fn accept_char(&mut self, text: char, render_flag: &mut jellyhaj_widgets_core::RenderFlag) {
        render_flag.set();
        let val = &mut self.lines[usize::from(self.line)];
        let index = char_index(val, &mut self.col);
        val.insert(index, text);
        self.col = self.col.strict_add(1);
        self.rewrap();
    }

    fn accept_text(&mut self, text: String, render_flag: &mut jellyhaj_widgets_core::RenderFlag) {
        let chars = match text
            .chars()
            .enumerate()
            .last()
            .map(|(v, _)| u16::try_from(v + 1))
        {
            Some(Ok(v)) => v.strict_add(1),
            Some(Err(_)) => {
                // string is definitely to long
                return;
            }
            None => 0,
        };
        render_flag.set();
        let val = &mut self.lines[usize::from(self.line)];
        let index = char_index(val, &mut self.col);
        val.insert_str(index, &text);
        self.col = self.col.strict_add(chars);
        self.rewrap();
    }
}

impl<R: 'static> JellyhajWidget<R> for Editor {
    fn init(
        &mut self,
        _cx: jellyhaj_widgets_core::WidgetContext<
            '_,
            Self::Action,
            impl jellyhaj_widgets_core::Wrapper<Self::Action>,
            R,
        >,
    ) {
    }

    #[instrument(level = "trace", ret, err, skip(_cx))]
    fn apply_action(
        &mut self,
        _cx: jellyhaj_widgets_core::WidgetContext<
            '_,
            Self::Action,
            impl jellyhaj_widgets_core::Wrapper<Self::Action>,
            R,
        >,
        action: Self::Action,
        render_flag: &mut jellyhaj_widgets_core::RenderFlag,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        match action {
            EditorAction::Finish => {
                self.send();
                Ok(Some(Navigation::PopContext))
            }
            EditorAction::NewLine => {
                render_flag.set();
                let line: usize = self.line.into();
                let index = char_index(&self.lines[line], &mut self.col);
                let new = self.lines[line].split_off(index);
                self.rewrap();
                self.lines.insert(line + 1, new);
                self.split.insert(line + 1, vec![]);
                self.line = self.line.strict_add(1);
                self.col = 0;
                self.rewrap();
                Ok(None)
            }
            EditorAction::Delete => {
                let (n, o) = self.col.overflowing_sub(1);
                self.col = n;
                let line: usize = self.line.into();
                'fin: {
                    if o {
                        let (n, o) = self.line.overflowing_sub(1);
                        self.line = n;
                        if o {
                            self.col = 0;
                            self.line = 0;
                            break 'fin;
                        }
                        let len = chars(&self.lines[line - 1]);
                        let cur = self.lines.remove(line);
                        let _ = self.split.remove(line);
                        self.col = len;
                        self.lines[line - 1].push_str(&cur);
                        trace!(
                            col = self.col,
                            line_len = chars(&self.lines[line - 1]),
                            "lines combined"
                        );
                    } else {
                        let line = &mut self.lines[line];
                        let index = char_index(line, &mut self.col);
                        line.remove(index);
                    }
                    self.rewrap();
                    render_flag.set();
                }
                Ok(None)
            }
            EditorAction::Up => {
                self.line = self.line.saturating_sub(1);
                Ok(None)
            }
            EditorAction::Down => {
                let l = self.line.saturating_add(1);
                let m = u16::try_from(self.lines.len().strict_sub(1)).expect("to large");
                self.line = min(l, m);
                Ok(None)
            }
            EditorAction::Left => {
                let (n, o) = self.col.overflowing_sub(1);
                self.col = n;
                if o {
                    let (n, o) = self.line.overflowing_sub(1);
                    self.line = n;
                    if o {
                        self.line = 0;
                        self.col = 0;
                    } else {
                        self.col = chars(&self.lines[usize::from(self.line)]);
                        render_flag.set();
                    }
                } else {
                    render_flag.set();
                }
                Ok(None)
            }
            EditorAction::Right => {
                let old = self.col;
                let old_line = self.line;
                let line: usize = self.line.into();
                self.col = self.col.saturating_add(1);
                if chars(&self.lines[line]) == self.col {
                    self.line = self.line.saturating_add(1);
                    if self.lines.len() == usize::from(self.line) {
                        self.col = old;
                        self.line = old_line;
                    } else {
                        render_flag.set();
                    }
                } else {
                    render_flag.set();
                }
                Ok(None)
            }
            EditorAction::Quit => Ok(Some(Navigation::PopContext)),
        }
    }

    fn click(
        &mut self,
        _cx: jellyhaj_widgets_core::WidgetContext<
            '_,
            Self::Action,
            impl jellyhaj_widgets_core::Wrapper<Self::Action>,
            R,
        >,
        _position: jellyhaj_widgets_core::ratatui::prelude::Position,
        _size: jellyhaj_widgets_core::ratatui::prelude::Size,
        _kind: jellyhaj_widgets_core::MouseEventKind,
        _modifier: jellyhaj_widgets_core::KeyModifiers,
        _render_flag: &mut jellyhaj_widgets_core::RenderFlag,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        Ok(None)
    }

    fn render_fallible_inner(
        &mut self,
        area: jellyhaj_widgets_core::ratatui::prelude::Rect,
        buf: &mut jellyhaj_widgets_core::ratatui::prelude::Buffer,
        _cx: jellyhaj_widgets_core::WidgetContext<
            '_,
            Self::Action,
            impl jellyhaj_widgets_core::Wrapper<Self::Action>,
            R,
        >,
        cursor: &mut Option<Cursor>,
    ) -> jellyhaj_widgets_core::Result<()> {
        if self.width != area.width {
            self.width = area.width;
            self.rewrap_all();
        }
        let outer = Block::bordered()
            .padding(Padding::uniform(1))
            .title(self.title.as_ref());
        let main = outer.inner(area);
        let line: usize = self.line.into();
        let start_off = (main.height - 1).div_ceil(2);
        let mut col = self.col;
        let split_index = self.split[line]
            .iter()
            .map(|s| chars(s))
            .enumerate()
            .find(|(_, chars)| {
                trace!(chars, col, "finding line");
                if *chars >= col {
                    true
                } else {
                    col -= *chars;
                    false
                }
            })
            .expect("col out of bounds")
            .0;
        let start_off = min(
            start_off,
            u16::try_from(split_index + self.split[0..line].iter().map(|l| l.len()).sum::<usize>())
                .unwrap_or(u16::MAX),
        );
        let end_off = main.height - 1 - start_off;
        {
            let mut area = main;
            area.height = 1;
            area.y += start_off;
            self.split[line][split_index].render_ref(area, buf);
            *cursor = Some(Cursor {
                position: Position {
                    x: area.x + self.col,
                    y: area.y,
                },
                kind: SetCursorStyle::DefaultUserShape,
            });
        }
        for (y, line) in (0..start_off).rev().zip(
            self.split[line][0..split_index].iter().rev().chain(
                self.split[0..line]
                    .iter()
                    .rev()
                    .flat_map(|l| l.iter().rev()),
            ),
        ) {
            let mut area = main;
            area.height = 1;
            area.y += y;
            line.render_ref(area, buf);
        }
        for (y, line) in (0..end_off).map(|v| v + 1 + start_off).zip(
            self.split[line][split_index + 1..]
                .iter()
                .chain(self.split[line + 1..].iter().flat_map(|l| l.iter())),
        ) {
            let mut area = main;
            area.height = 1;
            area.y += y;
            line.render_ref(area, buf);
        }
        outer.render(area, buf);
        Ok(())
    }
}
