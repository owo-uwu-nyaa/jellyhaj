use std::{convert::Infallible, ops::ControlFlow};

use color_eyre::Result;
use jellyhaj_core::state::Navigation;
use jellyhaj_widgets_core::{
    Buffer, KeyModifiers, MouseEventKind, Position, Rect, Size, WidgetContext, Wrapper,
};

use crate::{
    FormAction, FormItem, FormItemBase,
    form::{Form, FormData, component::FormComponent},
};

impl<Data: FormData> Form<Data> {
    pub(crate) fn dispatch_active_action<R: 'static>(
        &mut self,
        cx: WidgetContext<'_, FormAction<Data::Action>, impl Wrapper<FormAction<Data::Action>>, R>,
        action: FormAction<Infallible>,
    ) -> Result<Option<ControlFlow<Navigation, Data::AR>>> {
        self.data.with_selection_mut_cx(
            0,
            &mut self.sel,
            cx.wrap_with(FormAction::Inner),
            ApplyMovement(action),
        )
    }
}

pub trait WithSelection<AR, T> {
    fn with<I: FormItemBase<AR>>(
        self,
        sel: &I::SelectionInner,
        state: &I,
        name: &'static str,
        index: usize,
    ) -> T;
}

pub trait WithSelectionMut<AR, T> {
    fn with_mut<I: FormItemBase<AR>>(
        self,
        sel: &mut I::SelectionInner,
        state: &mut I,
        name: &'static str,
        index: usize,
    ) -> T;
}

pub trait WithSelectionMutCX<R: 'static, AR, T> {
    fn with_mut<I: FormItem<R, AR>>(
        self,
        sel: &mut I::SelectionInner,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        name: &'static str,
        index: usize,
    ) -> T;
}

pub trait WithIterItems<R: 'static, AR> {
    fn with<I: FormItem<R, AR>>(
        &mut self,
        state: &I,
        name: &'static str,
        index: usize,
    ) -> Result<()>;
}

pub trait WithIterItemsMut<R: 'static, AR> {
    fn with_mut<I: FormItem<R, AR>>(
        &mut self,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        name: &'static str,
        index: usize,
        show: bool,
    ) -> Result<()>;
}

pub trait WithIndexMut<R: 'static, AR> {
    fn with_mut<I: FormItem<R, AR>>(
        self,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        name: &'static str,
        index: usize,
    ) -> Result<I::SelectionInner>;
}

pub trait WithActionMut<R: 'static, AR, T> {
    fn with_mut<I: FormItem<R, AR>>(
        self,
        action: I::Action,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        index: usize,
    ) -> T;
}

pub(crate) struct AcceptsTextInput;

impl<AR> WithSelection<AR, bool> for AcceptsTextInput {
    fn with<I: FormItemBase<AR>>(
        self,
        sel: &I::SelectionInner,
        state: &I,
        name: &'static str,
        index: usize,
    ) -> bool {
        state.accepts_text_input(sel)
    }
}

pub(crate) struct ApplyChar(pub(crate) char);

impl<AR> WithSelectionMut<AR, ()> for ApplyChar {
    fn with_mut<I: FormItemBase<AR>>(
        self,
        sel: &mut I::SelectionInner,
        state: &mut I,
        name: &'static str,
        index: usize,
    ) {
        state.apply_char(sel, self.0);
    }
}

pub(crate) struct ApplyText(pub(crate) String);

impl<AR> WithSelectionMut<AR, ()> for ApplyText {
    fn with_mut<I: FormItemBase<AR>>(
        self,
        sel: &mut I::SelectionInner,
        state: &mut I,
        name: &'static str,
        index: usize,
    ) {
        state.apply_text(sel, self.0);
    }
}

pub(crate) struct ApplyMovement(pub(crate) FormAction<Infallible>);

impl<R: 'static, AR> WithSelectionMutCX<R, AR, Result<Option<ControlFlow<Navigation, AR>>>>
    for ApplyMovement
{
    fn with_mut<I: FormItem<R, AR>>(
        self,
        sel: &mut I::SelectionInner,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        name: &'static str,
        index: usize,
    ) -> Result<Option<ControlFlow<Navigation, AR>>> {
        Ok(state.apply_movement(sel, cx, self.0)?.map(|cf| match cf {
            ControlFlow::Continue(c) => ControlFlow::Continue(c.into()),
            ControlFlow::Break(n) => ControlFlow::Break(n),
        }))
    }
}

pub(crate) struct ApplyAction;

impl<R: 'static, AR> WithActionMut<R, AR, Result<Option<ControlFlow<Navigation, AR>>>>
    for ApplyAction
{
    fn with_mut<I: FormItem<R, AR>>(
        self,
        action: I::Action,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        index: usize,
    ) -> Result<Option<ControlFlow<Navigation, AR>>> {
        Ok(state.apply_action(cx, action)?.map(|cf| match cf {
            ControlFlow::Continue(c) => ControlFlow::Continue(c.into()),
            ControlFlow::Break(n) => ControlFlow::Break(n),
        }))
    }
}

pub(crate) struct AcceptsMovementAction;

impl<AR> WithSelection<AR, bool> for AcceptsMovementAction {
    fn with<I: FormItemBase<AR>>(
        self,
        sel: &I::SelectionInner,
        state: &I,
        name: &'static str,
        index: usize,
    ) -> bool {
        state.accepts_movement_action(sel)
    }
}

pub(crate) struct SelectionDefault;

impl<R: 'static, AR> WithIndexMut<R, AR> for SelectionDefault {
    fn with_mut<I: FormItem<R, AR>>(
        self,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        name: &'static str,
        index: usize,
    ) -> Result<I::SelectionInner> {
        Ok(I::SelectionInner::default())
    }
}

pub(crate) struct ClickCurrent<'s> {
    pub(crate) kind: MouseEventKind,
    pub(crate) modifier: KeyModifiers,
    pub(crate) pos: Position,
    pub(crate) cought: bool,
    pub(crate) store: &'s [u16],
    pub(crate) size: Size,
    pub(crate) offset: u16,
}

impl<R: 'static, AR> WithSelectionMutCX<R, AR, Result<Option<ControlFlow<Navigation, AR>>>>
    for &mut ClickCurrent<'_>
{
    fn with_mut<I: FormItem<R, AR>>(
        self,
        sel: &mut I::SelectionInner,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        name: &'static str,
        index: usize,
    ) -> Result<Option<ControlFlow<Navigation, AR>>> {
        let this_area = Rect {
            x: 0,
            y: self.store[index] - self.offset,
            width: self.size.width,
            height: state.height(),
        };
        let active = I::popup_area(state, sel, this_area, self.size);
        if (active.height - active.y > self.pos.y) && (active.width - active.x > self.pos.x) {
            let res = I::apply_click_active(
                state,
                cx,
                sel,
                this_area,
                self.size,
                self.pos,
                self.kind,
                self.modifier,
            )
            .map(|v| {
                v.map(|cf| match cf {
                    ControlFlow::Continue(c) => ControlFlow::Continue(c.into()),
                    ControlFlow::Break(n) => ControlFlow::Break(n),
                })
            });
            self.cought = true;
            res
        } else {
            Ok(None)
        }
    }
}

pub(crate) struct ClickItem<'s, AR> {
    pub(crate) pos: Position,
    pub(crate) res: Option<ControlFlow<Navigation, AR>>,
    pub(crate) size: Size,
    pub(crate) store: &'s [u16],
    pub(crate) kind: MouseEventKind,
    pub(crate) modifier: KeyModifiers,
}

impl<R: 'static, AR> WithIndexMut<R, AR> for &mut ClickItem<'_, AR> {
    fn with_mut<I: FormItem<R, AR>>(
        self,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        name: &'static str,
        index: usize,
    ) -> Result<I::SelectionInner> {
        let base = self.pos.y - self.store[index];
        if base < state.height() {
            let (s, res) = I::apply_click_inactive(
                state,
                cx,
                Size {
                    width: self.size.width,
                    height: state.height(),
                },
                Position {
                    x: self.pos.x,
                    y: base,
                },
                self.kind,
                self.modifier,
            )?;
            self.res = res.map(|cf| match cf {
                ControlFlow::Continue(c) => ControlFlow::Continue(c.into()),
                ControlFlow::Break(n) => ControlFlow::Break(n),
            });
            Ok(s.unwrap_or_default())
        } else {
            Ok(I::SelectionInner::default())
        }
    }
}

pub(crate) struct CalcHeight<'s, S: FormComponent> {
    pub(crate) data: &'s S,
    pub(crate) store: &'s mut Vec<u16>,
    pub(crate) height: u16,
    pub(crate) height_buf: u16,
}

impl<R: 'static, S: FormData> WithIterItems<R, S::AR> for CalcHeight<'_, S> {
    fn with<I: FormItem<R, S::AR>>(
        &mut self,
        state: &I,
        name: &'static str,
        index: usize,
    ) -> Result<()> {
        if self.data.show_if(index) {
            if index == 0 {
                self.store.clear();
                self.store.push(0);
                self.height = state.height();
                self.height_buf = state.height_buf();
            } else {
                self.height = self.height.strict_add(1);
                self.store.push(self.height);
                self.height = self.height.strict_add(state.height());
                self.height_buf = self
                    .height_buf
                    .saturating_sub(1)
                    .saturating_sub(state.height())
                    .strict_add(state.height_buf());
            }
        } else {
            self.store.push(self.height);
        }
        Ok(())
    }
}

pub(crate) struct Pass1<'s> {
    pub(crate) area: Rect,
    pub(crate) store: &'s [u16],
    pub(crate) buf: &'s mut Buffer,
    pub(crate) cur: usize,
}

impl<R: 'static, AR> WithIterItemsMut<R, AR> for Pass1<'_> {
    fn with_mut<I: FormItem<R, AR>>(
        &mut self,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        name: &'static str,
        index: usize,
        show: bool,
    ) -> Result<()> {
        if show {
            let mut this_area = self.area;
            this_area.height = state.height();
            this_area.y += self.store[index];
            I::render_pass_main(state, cx, this_area, self.buf, self.cur == index, name)?;
        }
        Ok(())
    }
}

pub(crate) struct Pass2<'s> {
    pub(crate) area: Rect,
    pub(crate) store: &'s [u16],
    pub(crate) buf: &'s mut Buffer,
    pub(crate) offset: u16,
}

impl<R: 'static, AR> WithSelectionMutCX<R, AR, Result<()>> for Pass2<'_> {
    fn with_mut<I: FormItem<R, AR>>(
        self,
        sel: &mut I::SelectionInner,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        name: &'static str,
        index: usize,
    ) -> Result<()> {
        let mut this_area = self.area;
        this_area.height = state.height();
        this_area.y += self.store[index] - self.offset;
        I::render_pass_popup(state, cx, this_area, self.area, self.buf, name, sel)
    }
}
