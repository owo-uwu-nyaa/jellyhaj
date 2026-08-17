use std::fmt::Debug;
use std::{convert::Infallible, ops::ControlFlow};

use jellyhaj_core::state::Navigation;
use jellyhaj_widgets_core::{Rect, Result, WidgetContext, Wrapper};
use ratatui::buffer::CellWidth;
use ratatui::widgets::{Block, BorderType, Widget};
use valuable::Valuable;

use crate::{FormAction, FormItem, FormItemBase};

#[derive(Debug, Valuable, Default)]
pub struct TextField {
    pub text: String,
}

impl TextField {
    #[must_use]
    pub const fn new(text: String) -> Self {
        Self { text }
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
    fn apply_char(&mut self, sel: &mut Self::SelectionInner, text: char) {
        self.text.push(text);
    }
    fn apply_text(&mut self, sel: &mut Self::SelectionInner, text: String) {
        self.text.push_str(&text);
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
    ) -> Result<Option<ControlFlow<Navigation, Infallible>>> {
        if matches!(action, FormAction::Delete) {
            self.text.pop();
        }
        Ok(None)
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
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
        self.text.as_str().render(main, buf);
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
    ) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Valuable, Default)]
pub struct TextFieldDynamic {
    pub text: String,
    pub label: String,
}

impl TextFieldDynamic {
    #[must_use]
    pub const fn new(text: String, label: String) -> Self {
        Self { text, label }
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
    fn apply_char(&mut self, sel: &mut Self::SelectionInner, text: char) {
        self.text.push(text);
    }
    fn apply_text(&mut self, sel: &mut Self::SelectionInner, text: String) {
        self.text.push_str(&text);
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
    ) -> Result<Option<ControlFlow<Navigation, Infallible>>> {
        if matches!(action, FormAction::Delete) {
            self.text.pop();
        }
        Ok(None)
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
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
        self.text.as_str().render(main, buf);
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
    ) -> Result<()> {
        Ok(())
    }
}
