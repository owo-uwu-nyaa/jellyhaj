use std::{convert::Infallible, ops::ControlFlow};

use jellyhaj_core::state::Navigation;
use jellyhaj_widgets_core::{Rect, Result, WidgetContext, Wrapper};
use ratatui::widgets::{Block, BorderType, Widget};
use valuable::Valuable;

use crate::{FormAction, FormItem, FormItemInfo};

#[derive(Debug, Valuable, Default)]
pub struct TextField {
    pub text: String,
}

impl TextField {
    pub fn new(text: String) -> Self {
        Self { text }
    }
}
#[cfg(feature = "serde")]
mod s {
    use serde::{Deserialize, Serialize};

    use crate::text_field::TextField;

    impl<'de> Deserialize<'de> for TextField {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            String::deserialize(deserializer).map(|text| TextField { text })
        }
    }

    impl Serialize for TextField {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            self.text.serialize(serializer)
        }
    }
}

impl<AR: From<Infallible>> FormItemInfo<AR> for TextField {
    const HEIGHT: u16 = 3;

    const HEIGHT_BUF: u16 = 0;

    type SelectionInner = ();

    type Ret = Infallible;

    type Action = Infallible;
}

impl<R: 'static, AR: From<Infallible>> FormItem<R, AR> for TextField {
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

    fn apply_movement(
        &mut self,
        sel: &mut Self::SelectionInner,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: FormAction<Infallible>,
    ) -> Result<Option<ControlFlow<Navigation, Infallible>>> {
        if let FormAction::Delete = action {
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

    fn popup_area(
        &self,
        sel: &Self::SelectionInner,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Size,
    ) -> ratatui::prelude::Rect {
        Rect::ZERO
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
