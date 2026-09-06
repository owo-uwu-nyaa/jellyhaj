use std::fmt::Debug;
use std::{convert::Infallible, ops::ControlFlow};

use jellyhaj_core::state::Navigation;
use jellyhaj_widgets_core::{Cursor, Rect, RenderFlag, Result, WidgetContext, Wrapper};
use ratatui::buffer::CellWidth;
use ratatui::style::Color;
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Widget};
use valuable::Valuable;

use crate::{FormAction, FormItem, FormItemBase};
pub mod support {
    use std::convert::Infallible;

    use crossterm::cursor::SetCursorStyle;
    use jellyhaj_widgets_core::{Cursor, RenderFlag};
    use ratatui::buffer::CellWidth;
    use tracing::{instrument, trace};

    use crate::FormAction;

    /// Index of `index`'th char in string.
    /// If `index` is out of bounds it is adjusted to `chars.len()` and `val.str()` is returned
    #[instrument(level = "trace", ret)]
    pub fn char_index(val: &str, index: &mut u16) -> usize {
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

    #[instrument(level = "trace", ret)]
    pub fn apply_movement(
        pos: &mut u16,
        text: &mut String,
        action: FormAction<Infallible>,
        render_flag: &mut RenderFlag,
    ) {
        match action {
            FormAction::Left => {
                render_flag.set();
                *pos = pos.saturating_sub(1);
            }
            FormAction::Right => {
                render_flag.set();
                *pos = pos.saturating_add(1);
            }
            FormAction::Delete => {
                if let Some(index) = char_index(text, pos).checked_sub(1) {
                    let index = text.floor_char_boundary(index);
                    render_flag.set();
                    text.remove(index);
                    *pos -= 1;
                }
            }
            FormAction::Enter | FormAction::Quit | FormAction::Up | FormAction::Down => todo!(),
        }
    }
    #[instrument(level = "trace", ret)]
    pub fn apply_char(pos: &mut u16, text: &mut String, render_flag: &mut RenderFlag, c: char) {
        render_flag.set();
        let index = char_index(text, pos);
        text.insert(index, c);
        *pos = pos.strict_add(1);
    }
    #[instrument(level = "trace", ret)]
    pub fn apply_str(pos: &mut u16, text: &mut String, render_flag: &mut RenderFlag, s: String) {
        let chars = match s
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
        let index = char_index(text, pos);
        text.insert_str(index, &s);
        *pos = pos.strict_add(chars);
    }
    #[instrument(level = "trace")]
    pub fn position_cursor(
        pos: &mut u16,
        text: &str,
        cursor: &mut Option<Cursor>,
        area: ratatui::layout::Rect,
    ) {
        let mut position = area.as_position();
        position.x += 1;
        position.y += 1;
        let index = char_index(text, pos);
        let behind = &text[0..index];
        position.x += behind.cell_width();
        trace!("setting position to ({},{})", position.x, position.y);
        *cursor = Some(Cursor {
            position,
            kind: SetCursorStyle::SteadyBar,
        });
    }
    #[must_use]
    pub fn chars(val: &str) -> u16 {
        val.chars().map(|_| 1u16).sum()
    }

    #[cfg(test)]
    mod tests {
        use crate::text_field::support::char_index;

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
}

#[derive(Debug, Valuable, Default)]
pub struct TextField {
    pub text: String,
    pub pos: u16,
    #[valuable(skip)]
    checker: Option<fn(&str) -> bool>,
}

impl TextField {
    #[must_use]
    pub fn new(text: String) -> Self {
        let pos = support::chars(&text);
        Self {
            text,
            pos,
            checker: None,
        }
    }
    #[must_use]
    pub fn with_checker(text: String, checker: fn(&str) -> bool) -> Self {
        let pos = support::chars(&text);
        Self {
            text,
            pos,
            checker: Some(checker),
        }
    }
}
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for TextField {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(|text| TextField { text })
    }
}

#[cfg(feature = "serde")]
impl Serialize for TextField {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.text.serialize(serializer)
    }
}

impl<AR: From<Infallible> + Debug> FormItemBase<AR> for TextField {
    type SelectionInner = ();

    type Ret = Infallible;

    type Action = Infallible;

    fn accepts_text_input(&self, sel: &Self::SelectionInner) -> bool {
        true
    }
    fn apply_char(
        &mut self,
        sel: &mut Self::SelectionInner,
        text: char,
        render_flag: &mut RenderFlag,
    ) {
        support::apply_char(&mut self.pos, &mut self.text, render_flag, text);
    }
    fn apply_text(
        &mut self,
        sel: &mut Self::SelectionInner,
        text: String,
        render_flag: &mut RenderFlag,
    ) {
        support::apply_str(&mut self.pos, &mut self.text, render_flag, text);
    }

    fn accepts_movement_action(&self, sel: &Self::SelectionInner) -> bool {
        false
    }
    fn popup_area(
        &self,
        sel: &Self::SelectionInner,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Size,
    ) -> ratatui::prelude::Rect {
        Rect::ZERO
    }

    fn height(&self) -> u16 {
        3
    }

    fn height_buf(&self) -> u16 {
        0
    }
}

impl<R: 'static, AR: From<Infallible> + Debug> FormItem<R, AR> for TextField {
    fn apply_movement(
        &mut self,
        sel: &mut Self::SelectionInner,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: FormAction<Infallible>,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<ControlFlow<Navigation, Infallible>>> {
        support::apply_movement(&mut self.pos, &mut self.text, action, render_flag);
        Ok(None)
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<ControlFlow<Navigation, Self::Ret>>> {
        match action {}
    }

    fn apply_click_active(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        sel: &mut Self::SelectionInner,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Size,
        pos: ratatui::prelude::Position,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<ControlFlow<Navigation, Infallible>>> {
        Ok(None)
    }

    fn apply_click_inactive(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        size: ratatui::prelude::Size,
        pos: ratatui::prelude::Position,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
        render_flag: &mut RenderFlag,
    ) -> Result<(
        Option<Self::SelectionInner>,
        Option<ControlFlow<Navigation, Infallible>>,
    )> {
        Ok((Some(()), None))
    }

    fn render_pass_main(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        active: bool,
        name: &'static str,
    ) -> Result<()> {
        let mut block = Block::bordered().title(name);
        if active {
            block = block.border_type(BorderType::Double);
        }
        let main = block.inner(area);
        let wrong = self
            .checker
            .as_ref()
            .is_some_and(|checker| !checker(&self.text));
        let text = self.text.as_str();
        if wrong {
            Span::styled(text, Color::Red).render(main, buf);
        } else {
            text.render(main, buf);
        }
        block.render(area, buf);
        Ok(())
    }

    fn render_pass_popup(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        name: &'static str,
        sel: &mut Self::SelectionInner,
        cursor: &mut Option<Cursor>,
    ) -> Result<()> {
        support::position_cursor(&mut self.pos, &self.text, cursor, area);
        Ok(())
    }
}

#[derive(Debug, Valuable, Default)]
pub struct TextFieldDynamic {
    pub text: String,
    #[valuable(skip)]
    checker: Option<fn(&str) -> bool>,
    pub label: String,
    pub pos: u16,
}

impl TextFieldDynamic {
    #[must_use]
    pub fn new(text: String, label: String) -> Self {
        let pos = support::chars(&text);
        Self {
            text,
            label,
            checker: None,
            pos,
        }
    }
    #[must_use]
    pub fn with_checker(text: String, label: String, checker: fn(&str) -> bool) -> Self {
        let pos = support::chars(&text);
        Self {
            text,
            label,
            checker: Some(checker),
            pos,
        }
    }
}

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for TextFieldDynamic {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(|text| TextField { text })
    }
}

#[cfg(feature = "serde")]
impl Serialize for TextFieldDynamic {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.text.serialize(serializer)
    }
}

impl<AR: From<Infallible> + Debug> FormItemBase<AR> for TextFieldDynamic {
    type SelectionInner = ();

    type Ret = Infallible;

    type Action = Infallible;

    fn accepts_text_input(&self, sel: &Self::SelectionInner) -> bool {
        true
    }
    fn apply_char(
        &mut self,
        sel: &mut Self::SelectionInner,
        text: char,
        render_flag: &mut RenderFlag,
    ) {
        support::apply_char(&mut self.pos, &mut self.text, render_flag, text);
    }
    fn apply_text(
        &mut self,
        sel: &mut Self::SelectionInner,
        text: String,
        render_flag: &mut RenderFlag,
    ) {
        support::apply_str(&mut self.pos, &mut self.text, render_flag, text);
    }

    fn accepts_movement_action(&self, sel: &Self::SelectionInner) -> bool {
        false
    }
    fn popup_area(
        &self,
        sel: &Self::SelectionInner,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Size,
    ) -> ratatui::prelude::Rect {
        Rect::ZERO
    }

    fn height(&self) -> u16 {
        3
    }

    fn height_buf(&self) -> u16 {
        0
    }
}

impl<R: 'static, AR: From<Infallible> + Debug> FormItem<R, AR> for TextFieldDynamic {
    fn apply_movement(
        &mut self,
        sel: &mut Self::SelectionInner,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: FormAction<Infallible>,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<ControlFlow<Navigation, Infallible>>> {
        support::apply_movement(&mut self.pos, &mut self.text, action, render_flag);
        Ok(None)
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<ControlFlow<Navigation, Self::Ret>>> {
        unreachable!()
    }

    fn apply_click_active(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        sel: &mut Self::SelectionInner,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Size,
        pos: ratatui::prelude::Position,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<ControlFlow<Navigation, Infallible>>> {
        unimplemented!()
    }

    fn apply_click_inactive(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        size: ratatui::prelude::Size,
        pos: ratatui::prelude::Position,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
        render_flag: &mut RenderFlag,
    ) -> Result<(
        Option<Self::SelectionInner>,
        Option<ControlFlow<Navigation, Infallible>>,
    )> {
        Ok((Some(()), None))
    }

    fn render_pass_main(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        mut area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        active: bool,
        name: &'static str,
    ) -> Result<()> {
        let mut block = Block::bordered();
        if active {
            block = block.border_type(BorderType::Double);
        }
        let main = block.inner(area);
        let wrong = self
            .checker
            .as_ref()
            .is_some_and(|checker| !checker(&self.text));
        let text = self.text.as_str();
        if wrong {
            Span::styled(text, Color::Red).render(main, buf);
        } else {
            text.render(main, buf);
        }
        block.render(area, buf);
        area.x += 1;
        area.height = 1;
        area.width -= 2;
        name.render(area, buf);
        let name_width = name.cell_width();
        if name_width > 0 {
            area.x += name_width + 1;
        }
        if let Some(w) = area.width.checked_sub(name_width + 1) {
            area.width = w;
            self.label.as_str().render(area, buf);
        }
        Ok(())
    }

    fn render_pass_popup(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        name: &'static str,
        sel: &mut Self::SelectionInner,
        cursor: &mut Option<Cursor>,
    ) -> Result<()> {
        support::position_cursor(&mut self.pos, &self.text, cursor, area);
        Ok(())
    }
}
