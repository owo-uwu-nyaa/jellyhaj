use std::{convert::Infallible, fmt::Debug, ops::ControlFlow};

use jellyhaj_core::state::Navigation;
use jellyhaj_widgets_core::{
    MouseEventKind, Rect, RenderFlag, Result, WidgetContext, Wrapper,
    valuable::{Fields, NamedValues, StructDef, Structable, Valuable, Value},
};
use ratatui::{
    buffer::CellWidth,
    crossterm::event::MouseButton,
    widgets::{Block, BorderType, Widget},
};
use valuable::NamedField;

use crate::{FormAction, FormItem, FormItemBase};

pub trait ActionCreator: Debug {
    type T;
    fn make_action(&self) -> Self::T;
}

impl<C: Clone + Debug> ActionCreator for C {
    type T = Self;

    fn make_action(&self) -> Self::T {
        self.clone()
    }
}

#[derive(Default, Debug)]
pub struct Button<Creator: ActionCreator> {
    creator: Creator,
    width: u16,
}

impl<Creator: ActionCreator> Valuable for Button<Creator> {
    fn as_value(&self) -> Value<'_> {
        Value::Structable(self)
    }

    fn visit(&self, visit: &mut dyn jellyhaj_widgets_core::valuable::Visit) {
        visit.visit_named_fields(&NamedValues::new(&[], &[]));
    }
}

impl<Creator: ActionCreator> Structable for Button<Creator> {
    fn definition(&self) -> StructDef<'_> {
        StructDef::new_static("Button", Fields::Named(&[]))
    }
}

impl<Creator: ActionCreator> Button<Creator> {
    pub const fn new(creator: Creator) -> Self {
        Self { creator, width: 0 }
    }
}

struct Centered {
    offset: u16,
    size: u16,
}

const fn center(full: u16, requested: u16) -> Centered {
    if full > requested {
        let buf = full - requested;
        Centered {
            offset: buf / 2,
            size: requested,
        }
    } else {
        Centered {
            offset: 0,
            size: full,
        }
    }
}

impl<C: ActionCreator, AR: From<C::T> + Debug> FormItemBase<AR> for Button<C> {
    type SelectionInner = ();

    type Ret = C::T;

    type Action = Infallible;

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

impl<R: 'static, C: ActionCreator, AR: From<C::T> + Debug> FormItem<R, AR> for Button<C> {
    fn apply_movement(
        &mut self,
        sel: &mut Self::SelectionInner,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: crate::FormAction<Infallible>,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<ControlFlow<Navigation, C::T>>> {
        if matches!(action, FormAction::Enter) {
            Ok(Some(ControlFlow::Continue(self.creator.make_action())))
        } else {
            Ok(None)
        }
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
    ) -> Result<Option<ControlFlow<Navigation, C::T>>> {
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
        Option<ControlFlow<Navigation, C::T>>,
    )> {
        let centered = center(size.width, self.width);
        if kind == MouseEventKind::Down(MouseButton::Left)
            && pos.x >= centered.offset
            && pos.x < centered.offset + centered.size
        {
            Ok((
                Some(()),
                Some(ControlFlow::Continue(self.creator.make_action())),
            ))
        } else {
            Ok((None, None))
        }
    }

    fn render_pass_main(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        mut area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        active: bool,
        name: &'static str,
    ) -> Result<()> {
        self.width = name.cell_width() + 2;
        let centered = center(area.width, self.width);
        area.x += centered.offset;
        area.width = centered.size;
        let mut block = Block::bordered();
        if active {
            block = block.border_type(BorderType::Double);
        }
        let main = block.inner(area);
        name.render(main, buf);
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

#[derive(Default, Debug)]
pub struct DynamicButton<Creator: ActionCreator> {
    creator: Creator,
    pub name: String,
    width: u16,
}

impl<Creator: ActionCreator> Valuable for DynamicButton<Creator> {
    fn as_value(&self) -> Value<'_> {
        Value::Structable(self)
    }

    fn visit(&self, visit: &mut dyn jellyhaj_widgets_core::valuable::Visit) {
        visit.visit_named_fields(&NamedValues::new(DEFS, &[self.name.as_value()]));
    }
}

static DEFS: &[NamedField] = &[NamedField::new("name")];

impl<Creator: ActionCreator> Structable for DynamicButton<Creator> {
    fn definition(&self) -> StructDef<'_> {
        StructDef::new_static("Button", Fields::Named(DEFS))
    }
}

impl<Creator: ActionCreator> DynamicButton<Creator> {
    pub const fn new(name: String, creator: Creator) -> Self {
        Self {
            name,
            creator,
            width: 0,
        }
    }
}

impl<C: ActionCreator, AR: From<C::T> + Debug> FormItemBase<AR> for DynamicButton<C> {
    type SelectionInner = ();

    type Ret = C::T;

    type Action = Infallible;

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

impl<R: 'static, C: ActionCreator, AR: From<C::T> + Debug> FormItem<R, AR> for DynamicButton<C> {
    fn apply_movement(
        &mut self,
        sel: &mut Self::SelectionInner,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: crate::FormAction<Infallible>,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<ControlFlow<Navigation, C::T>>> {
        if matches!(action, FormAction::Enter) {
            Ok(Some(ControlFlow::Continue(self.creator.make_action())))
        } else {
            Ok(None)
        }
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
    ) -> Result<Option<ControlFlow<Navigation, C::T>>> {
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
        Option<ControlFlow<Navigation, C::T>>,
    )> {
        let centered = center(size.width, self.width);
        if kind == MouseEventKind::Down(MouseButton::Left)
            && pos.x >= centered.offset
            && pos.x < centered.offset + centered.size
        {
            Ok((
                Some(()),
                Some(ControlFlow::Continue(self.creator.make_action())),
            ))
        } else {
            Ok((None, None))
        }
    }

    fn render_pass_main(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        mut area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        active: bool,
        name: &'static str,
    ) -> Result<()> {
        let name_width = name.cell_width();
        self.width = name_width + u16::from(!name.is_empty()) + self.name.cell_width() + 2;
        let centered = center(area.width, self.width);
        area.x += centered.offset;
        area.width = centered.size;
        let mut block = Block::bordered();
        if active {
            block = block.border_type(BorderType::Double);
        }
        let mut main = block.inner(area);
        name.render(main, buf);
        if name_width > 0 {
            main.x += name_width + 1;
        }
        if let Some(w) = main.width.checked_sub(name_width + 1) {
            main.width = w;
            self.name.as_str().render(main, buf);
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
    ) -> Result<()> {
        Ok(())
    }
}
