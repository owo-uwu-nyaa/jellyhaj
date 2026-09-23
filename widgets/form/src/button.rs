use std::{cmp::max, convert::Infallible, fmt::Debug, ops::ControlFlow};

use jellyhaj_core::state::Navigation;
use jellyhaj_widgets_core::{
    Cursor, MouseEventKind, Position, Rect, RenderFlag, Result, WidgetContext, Wrapper,
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
        cursor: &mut Option<Cursor>,
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
        cursor: &mut Option<Cursor>,
    ) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct SymbolButton<Creator: ActionCreator, I: Debug + Valuable> {
    creator: Creator,
    selected: bool,
    pub inner: I,
    symbol: char,
}

impl<Creator: ActionCreator, I: Debug + Valuable> SymbolButton<Creator, I> {
    pub const fn new(creator: Creator, symbol: char, inner: I) -> Self {
        Self {
            creator,
            selected: false,
            inner,
            symbol,
        }
    }
}

const _: () = {
    static FIELDS: &[NamedField] = &[
        NamedField::new("selected"),
        NamedField::new("symbol"),
        NamedField::new("inner"),
    ];
    impl<Creator: ActionCreator, I: Debug + Valuable> Valuable for SymbolButton<Creator, I> {
        fn as_value(&self) -> Value<'_> {
            Value::Structable(self)
        }

        fn visit(&self, visit: &mut dyn valuable::Visit) {
            visit.visit_named_fields(&NamedValues::new(
                FIELDS,
                &[
                    self.selected.as_value(),
                    self.symbol.as_value(),
                    self.inner.as_value(),
                ],
            ));
        }
    }

    impl<Creator: ActionCreator, I: Debug + Valuable> Structable for SymbolButton<Creator, I> {
        fn definition(&self) -> StructDef<'_> {
            StructDef::new_static("SymbolButton", Fields::Named(FIELDS))
        }
    }
};

impl<C: ActionCreator, AR: From<C::T> + Debug, I: FormItemBase<AR> + Debug> FormItemBase<AR>
    for SymbolButton<C, I>
{
    type SelectionInner = I::SelectionInner;

    type Ret = AR;

    type Action = I::Action;

    fn height(&self) -> u16 {
        max(3, self.inner.height())
    }

    fn height_buf(&self) -> u16 {
        self.inner
            .height_buf()
            .saturating_sub(3u16.saturating_sub(self.inner.height()))
    }

    fn accepts_movement_action(&self, sel: &Self::SelectionInner) -> bool {
        (!self.selected) && self.inner.accepts_movement_action(sel)
    }

    fn popup_area(
        &self,
        sel: &Self::SelectionInner,
        mut area: Rect,
        full_area: ratatui::prelude::Size,
    ) -> Rect {
        if self.selected {
            Rect::ZERO
        } else {
            area.width -= 4;
            self.inner.popup_area(sel, area, full_area)
        }
    }

    fn accepts_text_input(&self, sel: &Self::SelectionInner) -> bool {
        (!self.selected) && self.inner.accepts_text_input(sel)
    }

    fn apply_char(
        &mut self,
        sel: &mut Self::SelectionInner,
        text: char,
        render_flag: &mut RenderFlag,
    ) {
        self.inner.apply_char(sel, text, render_flag);
    }

    fn apply_text(
        &mut self,
        sel: &mut Self::SelectionInner,
        text: String,
        render_flag: &mut RenderFlag,
    ) {
        self.inner.apply_text(sel, text, render_flag);
    }
}

impl<R: 'static, C: ActionCreator, AR: From<C::T> + Debug, I: FormItem<R, AR> + Debug>
    FormItem<R, AR> for SymbolButton<C, I>
{
    fn apply_movement(
        &mut self,
        sel: &mut Self::SelectionInner,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: FormAction<Infallible>,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<ControlFlow<Navigation, Self::Ret>>> {
        match action {
            FormAction::Left if self.selected => {
                self.selected = false;
                render_flag.set();
            }
            FormAction::Right if !self.selected => {
                self.selected = true;
                render_flag.set();
            }
            FormAction::Enter if self.selected => {
                return Ok(Some(ControlFlow::Continue(
                    self.creator.make_action().into(),
                )));
            }
            FormAction::Up
            | FormAction::Down
            | FormAction::Quit
            | FormAction::Enter
            | FormAction::Delete => {
                return self
                    .inner
                    .apply_movement(sel, cx, action, render_flag)
                    .map(|v| v.map(|v| v.map_continue(Into::into)));
            }
            _ => {}
        }
        Ok(None)
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<ControlFlow<Navigation, Self::Ret>>> {
        self.inner
            .apply_action(cx, action, render_flag)
            .map(|v| v.map(|v| v.map_continue(Into::into)))
    }

    fn apply_click_active(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        sel: &mut Self::SelectionInner,
        mut area: Rect,
        full_area: ratatui::prelude::Size,
        pos: ratatui::prelude::Position,
        kind: MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<ControlFlow<Navigation, Self::Ret>>> {
        area.width -= 4;
        self.inner
            .apply_click_active(cx, sel, area, full_area, pos, kind, modifier, render_flag)
            .map(|v| v.map(|v| v.map_continue(Into::into)))
    }

    fn apply_click_inactive(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        mut size: ratatui::prelude::Size,
        pos: ratatui::prelude::Position,
        kind: MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
        render_flag: &mut RenderFlag,
    ) -> Result<(
        Option<Self::SelectionInner>,
        Option<ControlFlow<Navigation, Self::Ret>>,
    )> {
        if pos.x > size.width - 4 {
            if pos.x > size.width - 3
                && pos.y < 3
                && kind == MouseEventKind::Down(MouseButton::Left)
            {
                Ok((
                    None,
                    Some(ControlFlow::Continue(self.creator.make_action().into())),
                ))
            } else {
                Ok((None, None))
            }
        } else {
            size.width -= 4;
            self.inner
                .apply_click_inactive(cx, size, pos, kind, modifier, render_flag)
                .map(|(v1, v2)| (v1, v2.map(|v| v.map_continue(Into::into))))
        }
    }

    fn render_pass_main(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        active: bool,
        name: &'static str,
    ) -> Result<()> {
        let button_area = Rect {
            x: area.x + area.width - 3,
            y: area.y,
            width: 3,
            height: 3,
        };

        buf[Position {
            x: area.x + area.width - 2,
            y: area.y + 1,
        }]
        .set_char(self.symbol);
        let mut block = Block::bordered();
        if self.selected & active {
            block = block.border_type(BorderType::Double);
        }
        block.render(button_area, buf);
        let mut main = area;
        main.width -= 4;
        self.inner
            .render_pass_main(cx, main, buf, active && !self.selected, name)
    }

    fn render_pass_popup(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        mut area: Rect,
        full_area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        name: &'static str,
        sel: &mut Self::SelectionInner,
        cursor: &mut Option<Cursor>,
    ) -> Result<()> {
        area.width -= 4;
        self.inner
            .render_pass_popup(cx, area, full_area, buf, name, sel, cursor)
    }
}
