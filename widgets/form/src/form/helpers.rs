use std::{convert::Infallible, fmt::Debug, ops::ControlFlow};

use color_eyre::Result;
use jellyhaj_core::state::Navigation;
use jellyhaj_widgets_core::{
    Buffer, KeyModifiers, MouseEventKind, Position, Rect, RenderFlag, Size, WidgetContext, Wrapper,
    spawn::tracing::trace,
};
use tracing::{Span, field, instrument};

use crate::{
    FormAction, FormItem, FormItemBase,
    form::{Form, FormData, component::FormComponent},
};

impl<Data: FormData> Form<Data> {
    pub(crate) fn dispatch_active_action<R: 'static>(
        &mut self,
        cx: WidgetContext<'_, FormAction<Data::Action>, impl Wrapper<FormAction<Data::Action>>, R>,
        action: FormAction<Infallible>,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<ControlFlow<Navigation, Data::AR>>> {
        self.data.with_selection_mut_cx(
            0,
            &mut self.sel,
            cx.wrap_with(FormAction::Inner),
            ApplyMovement {
                action,
                render_flag,
            },
        )
    }
}

/**
 * Get information about currently selected item.
 * Result should default to false
*/
pub trait WithSelection<AR: Debug> {
    fn with<I: FormItemBase<AR>>(
        self,
        sel: &I::SelectionInner,
        state: &I,
        name: &'static str,
        index: usize,
    ) -> bool;
}

/**
 * Modify the currently selected item.
 * If the current item does not exist just do nothing
 *  */
pub trait WithSelectionMut<AR: Debug> {
    fn with_mut<I: FormItemBase<AR>>(
        self,
        sel: &mut I::SelectionInner,
        state: &mut I,
        name: &'static str,
        index: usize,
    );
}

pub trait WithSelectionMutCX<R: 'static, AR: Debug, T: Default> {
    fn with_mut<I: FormItem<R, AR>>(
        self,
        sel: &mut I::SelectionInner,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        name: &'static str,
        index: usize,
    ) -> Result<T>;
}

pub trait WithIterItems<R: 'static, AR: Debug> {
    fn with<I: FormItem<R, AR>>(
        &mut self,
        state: &I,
        name: &'static str,
        index: usize,
    ) -> Result<()>;
}

pub trait WithIterItemsMut<R: 'static, AR: Debug> {
    fn with_mut<I: FormItem<R, AR>>(
        &mut self,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        name: &'static str,
        index: usize,
        show: bool,
    ) -> Result<()>;
}

pub trait WithIndexMut<R: 'static, AR: Debug> {
    fn with_mut<I: FormItem<R, AR>>(
        self,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        name: &'static str,
        index: usize,
    ) -> Result<I::SelectionInner>;
}

pub trait WithActionMut<R: 'static, AR: Debug, T> {
    fn with_mut<I: FormItem<R, AR>>(
        self,
        action: I::Action,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        index: usize,
    ) -> Result<Option<T>>;
}

pub(crate) struct AcceptsTextInput;

impl<AR: Debug> WithSelection<AR> for AcceptsTextInput {
    #[instrument(skip(self, state), name = "accepts_text_input", level = "trace", ret)]
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

pub(crate) struct ApplyChar<'r> {
    pub(crate) text: char,
    pub(crate) render_flag: &'r mut RenderFlag,
}

impl<AR: Debug> WithSelectionMut<AR> for ApplyChar<'_> {
    #[instrument(
        skip(self, sel, state),
        name = "apply_char",
        level = "trace",
        fields(val)
    )]
    fn with_mut<I: FormItemBase<AR>>(
        self,
        sel: &mut I::SelectionInner,
        state: &mut I,
        name: &'static str,
        index: usize,
    ) {
        let mut val = [0u8; 4];
        let val = self.text.encode_utf8(&mut val);
        trace!(val, "apply char");
        state.apply_char(sel, self.text, self.render_flag);
    }
}

pub(crate) struct ApplyText<'r> {
    pub(crate) text: String,
    pub(crate) render_flag: &'r mut RenderFlag,
}

impl<AR: Debug> WithSelectionMut<AR> for ApplyText<'_> {
    #[instrument(skip(self, sel, state), name = "apply_text", level = "trace", fields(text = self.text.as_str()))]
    fn with_mut<I: FormItemBase<AR>>(
        self,
        sel: &mut I::SelectionInner,
        state: &mut I,
        name: &'static str,
        index: usize,
    ) {
        trace!("apply text");
        state.apply_text(sel, self.text, self.render_flag);
    }
}

pub(crate) struct ApplyMovement<'r> {
    pub(crate) action: FormAction<Infallible>,
    pub(crate) render_flag: &'r mut RenderFlag,
}

impl<R: 'static, AR: Debug> WithSelectionMutCX<R, AR, Option<ControlFlow<Navigation, AR>>>
    for ApplyMovement<'_>
{
    #[instrument(skip(self, cx, state), name = "apply_movement", level = "trace", fields(action = ?self.action), ret, err)]
    fn with_mut<I: FormItem<R, AR>>(
        self,
        sel: &mut I::SelectionInner,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        name: &'static str,
        index: usize,
    ) -> Result<Option<ControlFlow<Navigation, AR>>> {
        let res = state
            .apply_movement(sel, cx, self.action, self.render_flag)?
            .map(|cf| match cf {
                ControlFlow::Continue(c) => ControlFlow::Continue(c.into()),
                ControlFlow::Break(n) => ControlFlow::Break(n),
            });
        Ok(res)
    }
}

pub(crate) struct ApplyAction<'r>(pub(crate) &'r mut RenderFlag);

impl<R: 'static, AR: Debug> WithActionMut<R, AR, ControlFlow<Navigation, AR>> for ApplyAction<'_> {
    #[instrument(
        skip(self, cx, state),
        name = "apply_action",
        level = "trace",
        ret,
        err
    )]
    fn with_mut<I: FormItem<R, AR>>(
        self,
        action: I::Action,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        index: usize,
    ) -> Result<Option<ControlFlow<Navigation, AR>>> {
        let res = state.apply_action(cx, action, self.0)?.map(|cf| match cf {
            ControlFlow::Continue(c) => ControlFlow::Continue(c.into()),
            ControlFlow::Break(n) => ControlFlow::Break(n),
        });
        Ok(res)
    }
}

pub(crate) struct AcceptsMovementAction;

impl<AR: Debug> WithSelection<AR> for AcceptsMovementAction {
    #[instrument(
        skip(self, state),
        name = "accepts_movement_action",
        level = "trace",
        ret
    )]
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

impl<R: 'static, AR: Debug> WithIndexMut<R, AR> for SelectionDefault {
    #[instrument(
        skip(self, cx, state),
        name = "selection_default",
        level = "trace",
        ret
    )]
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
    pub(crate) render_flag: &'s mut RenderFlag,
}

impl<R: 'static, AR: Debug> WithSelectionMutCX<R, AR, Option<ControlFlow<Navigation, AR>>>
    for &mut ClickCurrent<'_>
{
    #[instrument(
        skip(self, cx, state),
        name = "click_current",
        level = "trace",
        fields(kind = ?self.kind, modifier = ?self.modifier, pos = ?self.pos, size = ?self.size, offset = self.offset, this_area, active),
        ret, err
    )]
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
        let span = Span::current();
        span.record("this_area", field::debug(&this_area));
        let active = I::popup_area(state, sel, this_area, self.size);
        span.record("active", field::debug(&active));
        if (active.height - active.y > self.pos.y) && (active.width - active.x > self.pos.x) {
            trace!("dispatch apply click active");
            let res = I::apply_click_active(
                state,
                cx,
                sel,
                this_area,
                self.size,
                self.pos,
                self.kind,
                self.modifier,
                self.render_flag,
            )
            .map(|v| {
                v.map(|cf| match cf {
                    ControlFlow::Continue(c) => ControlFlow::Continue(c.into()),
                    ControlFlow::Break(n) => ControlFlow::Break(n),
                })
            });
            trace!("cought");
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
    pub(crate) render_flag: &'s mut RenderFlag,
}

impl<R: 'static, AR: Debug> WithIndexMut<R, AR> for &mut ClickItem<'_, AR> {
    #[instrument(
        skip(self, cx, state),
        name = "click_item",
        level = "trace",
        fields(kind = ?self.kind, modifier = ?self.modifier, pos = ?self.pos, size = ?self.size, base),
        ret, err
    )]
    fn with_mut<I: FormItem<R, AR>>(
        self,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        name: &'static str,
        index: usize,
    ) -> Result<I::SelectionInner> {
        let base = self.pos.y - self.store[index];
        Span::current().record("base", base);
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
                self.render_flag,
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
    #[instrument(
        skip(self, state),
        name = "calc_height",
        level = "trace",
        fields(height = self.height, height_buf = self.height_buf , show_if ),
        err
    )]
    fn with<I: FormItem<R, S::AR>>(
        &mut self,
        state: &I,
        name: &'static str,
        index: usize,
    ) -> Result<()> {
        let span = Span::current();
        let show_if = self.data.show_if(index);
        span.record("show_if", show_if);
        let pos = if show_if {
            if index == 0 {
                self.store.clear();
                self.height = state.height();
                self.height_buf = state.height_buf();
                0
            } else {
                self.height = self.height.strict_add(1);
                let pos = self.height;
                self.height = self.height.strict_add(state.height());
                self.height_buf = self
                    .height_buf
                    .saturating_sub(1)
                    .saturating_sub(state.height())
                    .strict_add(state.height_buf());
                pos
            }
        } else {
            self.height
        };
        self.store.push(pos);
        span.record("pos", pos);
        trace!(pos, "height result");
        Ok(())
    }
}

pub(crate) struct Pass1<'s> {
    pub(crate) area: Rect,
    pub(crate) store: &'s [u16],
    pub(crate) buf: &'s mut Buffer,
    pub(crate) cur: usize,
}

impl<R: 'static, AR: Debug> WithIterItemsMut<R, AR> for Pass1<'_> {
    #[instrument(
        skip(self, cx, state),
        name = "pass1",
        level = "trace",
        fields(area = ?self.area, cur = self.cur, this_area),
        err
    )]
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
            Span::current().record("this_area", field::debug(&this_area));
            trace!("render pass 1");
            I::render_pass_main(state, cx, this_area, self.buf, self.cur == index, name)?;
        } else {
            trace!("render skipped");
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

impl<R: 'static, AR: Debug> WithSelectionMutCX<R, AR, ()> for Pass2<'_> {
    #[instrument(
        skip(self, cx, state),
        name = "pass2",
        level = "trace",
        fields(area = ?self.area, offset = self.offset, this_area),
        err
    )]
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
        Span::current().record("this_area", field::debug(&this_area));
        trace!("render pass 2");
        I::render_pass_popup(state, cx, this_area, self.area, self.buf, name, sel)
    }
}
