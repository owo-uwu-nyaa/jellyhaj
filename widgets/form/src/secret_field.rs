use jellyhaj_widgets_core::Rect;
use ratatui::widgets::{Block, BorderType, Widget};

use crate::FormItem;

pub struct SecretField {
    pub secret: String,
}
#[cfg(feature = "serde")]
mod s {
    use serde::{Deserialize, Serialize};

    use crate::secret_field::SecretField;
    impl<'de> Deserialize<'de> for SecretField {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            String::deserialize(deserializer).map(|secret| SecretField { secret })
        }
    }

    impl Serialize for SecretField {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            self.secret.serialize(serializer)
        }
    }
}

impl<T> FormItem<T> for SecretField {
    const HEIGHT: u16 = 3;

    const HEIGHT_BUF: u16 = 0;

    type SelectionInner = ();

    fn accepts_text_input(&self, sel: Self::SelectionInner) -> bool {
        true
    }

    fn apply_char(&mut self, sel: &mut Self::SelectionInner, text: char) {
        self.secret.push(text);
    }

    fn apply_text(&mut self, sel: &mut Self::SelectionInner, text: String) {
        self.secret.push_str(&text);
    }

    fn accepts_movement_action(&self, sel: Self::SelectionInner) -> bool {
        false
    }

    fn apply_action(
        &mut self,
        sel: &mut Self::SelectionInner,
        action: crate::FormAction,
    ) -> jellyhaj_widgets_core::Result<Option<T>> {
        if crate::FormAction::Delete == action {
            self.secret.pop();
        }
        Ok(None)
    }

    fn popup_area(
        &self,
        sel: Self::SelectionInner,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Size,
    ) -> ratatui::prelude::Rect {
        Rect::ZERO
    }

    fn apply_click_active(
        &mut self,
        sel: &mut Self::SelectionInner,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Size,
        pos: ratatui::prelude::Position,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
    ) -> jellyhaj_widgets_core::Result<Option<T>> {
        unimplemented!()
    }

    fn apply_click_inactive(
        &mut self,
        size: ratatui::prelude::Size,
        pos: ratatui::prelude::Position,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
    ) -> jellyhaj_widgets_core::Result<(Option<Self::SelectionInner>, Option<T>)> {
        Ok((Some(()), None))
    }

    fn render_pass_main(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        active: bool,
        name: &'static str,
    ) -> jellyhaj_widgets_core::Result<()> {
        let mut block = Block::bordered().title(name);
        if active {
            block = block.border_type(BorderType::Double);
        }
        let main = block.inner(area);
        let len = self.secret.chars().map(|_| 1).sum::<usize>();
        for p in main.positions().take(len) {
            buf[p].set_char('*');
        }
        block.render(area, buf);
        Ok(())
    }

    fn render_pass_popup(
        &mut self,
        area: ratatui::prelude::Rect,
        full_area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        name: &'static str,
        sel: Self::SelectionInner,
    ) -> jellyhaj_widgets_core::Result<()> {
        Ok(())
    }
}
