use std::{
    convert::Infallible,
    fmt::Debug,
    marker::PhantomData,
    ops::{ControlFlow, Index},
};

use jellyhaj_core::{CommandMapper, keybinds::FormCommand, state::Navigation};
use jellyhaj_widgets_core::{
    Buffer, JellyhajWidget, KeyModifiers, MouseEventKind, Position, Rect, Size, WidgetContext,
    Wrapper,
    valuable::{Fields, NamedField, NamedValues, StructDef, Structable, Valuable, Value},
};
use ratatui::widgets::{Block, Padding, StatefulWidget, Widget};
use tui_scrollview::{ScrollView, ScrollViewState};

use crate::{FormAction, FormItem};
use color_eyre::Result;

pub trait FormData<const TOTAL_SIZE: usize>: Sized + Send + Unpin + Valuable + 'static {
    type Selector: Debug + Send + Valuable;
    type AR: Debug + From<Infallible>;
    type Action: Debug + Send + 'static;

    fn with_selection<R: 'static, T, W: WithSelection<R, Self::AR, T>>(
        this: &Self::Selector,
        state: &Self,
        with: W,
    ) -> T;
    fn with_selection_mut<R: 'static, T, W: WithSelectionMut<R, Self::AR, T>>(
        this: &mut Self::Selector,
        state: &mut Self,
        with: W,
    ) -> T;
    fn with_selection_mut_cx<R: 'static, T, W: WithSelectionMutCX<R, Self::AR, T>>(
        this: &mut Self::Selector,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        state: &mut Self,
        with: W,
    ) -> T;
    fn with_index_mut<R: 'static, W: WithIndexMut<R, Self::AR>>(
        this: &mut Self::Selector,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        state: &mut Self,
        index: usize,
        with: W,
    ) -> Result<()>;
    fn with_iter<R: 'static, W: WithIterItems<R, Self::AR>>(
        state: &Self,
        with: &mut W,
    ) -> Result<()>;
    fn with_iter_mut<R: 'static, W: WithIterItemsMut<R, Self::AR>>(
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        state: &mut Self,
        with: &mut W,
    ) -> Result<()>;
    fn with_action_mut<R: 'static, T, W: WithActionMut<R, Self::AR, T>>(
        action: Self::Action,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        state: &mut Self,
        with: W,
    ) -> T;
    fn show_if(state: &Self) -> [bool; TOTAL_SIZE];
    fn index(sel: &Self::Selector) -> usize;
    const TITLE: &str;

    fn make_with(self, selection: Self::Selector) -> Form<{ TOTAL_SIZE }, Self> {
        Form {
            sel: selection,
            data: self,
            store: [0; _],
            offset: 0,
        }
    }
}

pub trait FormDataDefaultExt<const TOTAL_SIZE: usize>: FormData<TOTAL_SIZE> {
    fn make_with_default(self) -> Form<{ TOTAL_SIZE }, Self>;
}

impl<const TOTAL_SIZE: usize, F: FormData<TOTAL_SIZE>> FormDataDefaultExt<TOTAL_SIZE> for F
where
    F::Selector: Default,
{
    fn make_with_default<'s>(self) -> Form<{ TOTAL_SIZE }, Self> {
        Self::make_with(self, Self::Selector::default())
    }
}

pub struct Form<const TOTAL_SIZE: usize, Data: FormData<{ TOTAL_SIZE }>> {
    pub sel: Data::Selector,
    pub data: Data,
    store: [u16; TOTAL_SIZE],
    offset: u16,
}

static FORM_FIELDS: &[NamedField] = &[NamedField::new("sel"), NamedField::new("data")];

impl<const TOTAL_SIZE: usize, Data: FormData<{ TOTAL_SIZE }>> Valuable for Form<TOTAL_SIZE, Data> {
    fn as_value(&self) -> Value<'_> {
        Value::Structable(self)
    }

    fn visit(&self, visit: &mut dyn jellyhaj_widgets_core::valuable::Visit) {
        visit.visit_named_fields(&NamedValues::new(
            FORM_FIELDS,
            &[self.sel.as_value(), self.data.as_value()],
        ));
    }
}
impl<const TOTAL_SIZE: usize, Data: FormData<{ TOTAL_SIZE }>> Structable
    for Form<TOTAL_SIZE, Data>
{
    fn definition(&self) -> StructDef<'_> {
        StructDef::new_static("Form", Fields::Named(FORM_FIELDS))
    }
}

impl<const TOTAL_SIZE: usize, R: 'static, Data: FormData<{ TOTAL_SIZE }>> JellyhajWidget<R>
    for Form<{ TOTAL_SIZE }, Data>
{
    type Action = FormAction<Data::Action>;

    type ActionResult = ControlFlow<Navigation, Data::AR>;

    const NAME: &str = "form";

    fn visit_children(&self, visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {}

    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {}

    fn min_width(&self) -> Option<u16> {
        Some(10)
    }

    fn min_height(&self) -> Option<u16> {
        Some(10)
    }

    fn accepts_text_input(&self) -> bool {
        Data::with_selection::<R, _, _>(&self.sel, &self.data, AcceptsTextInput)
    }

    fn accept_char(&mut self, text: char) {
        Data::with_selection_mut::<R, _, _>(&mut self.sel, &mut self.data, ApplyChar(text))
    }

    fn accept_text(&mut self, text: String) {
        Data::with_selection_mut::<R, _, _>(&mut self.sel, &mut self.data, ApplyText(text))
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        let action: FormAction<Infallible> = match action {
            FormAction::Quit => FormAction::Quit,
            FormAction::Up => FormAction::Up,
            FormAction::Down => FormAction::Down,
            FormAction::Left => FormAction::Left,
            FormAction::Right => FormAction::Right,
            FormAction::Delete => FormAction::Delete,
            FormAction::Enter => FormAction::Enter,
            FormAction::Inner(action) => {
                return Data::with_action_mut(
                    action,
                    cx.wrap_with(FormAction::Inner),
                    &mut self.data,
                    ApplyAction,
                );
            }
        };
        if Data::with_selection::<R, _, _>(&self.sel, &self.data, AcceptsMovementAction) {
            self.dispatch_active_action(cx, action)
        } else {
            match action {
                FormAction::Up => {
                    let start = Data::index(&self.sel);
                    let show_if = Data::show_if(&self.data);
                    let mut current = start;
                    let index = loop {
                        current = current.checked_sub(1).unwrap_or(TOTAL_SIZE.strict_sub(1));
                        if show_if[current] {
                            break current;
                        } else if current == start {
                            panic!("all form other than the current are hidden")
                        }
                    };
                    Data::with_index_mut(
                        &mut self.sel,
                        cx.wrap_with(FormAction::Inner),
                        &mut self.data,
                        index,
                        SelectionDefault,
                    )?;
                    Ok(None)
                }
                FormAction::Down => {
                    let start = Data::index(&self.sel);
                    let show_if = Data::show_if(&self.data);
                    let mut current = start;
                    let index = loop {
                        current = current.strict_add(1) % TOTAL_SIZE;
                        if show_if[current] {
                            break current;
                        } else if current == start {
                            panic!("all form other than the current are hidden")
                        }
                    };
                    Data::with_index_mut(
                        &mut self.sel,
                        cx.wrap_with(FormAction::Inner),
                        &mut self.data,
                        index,
                        SelectionDefault,
                    )?;
                    Ok(None)
                }

                FormAction::Delete => self.dispatch_active_action(cx, FormAction::Delete),
                FormAction::Enter => self.dispatch_active_action(cx, FormAction::Enter),
                FormAction::Quit => Ok(Some(ControlFlow::Break(Navigation::PopContext))),
                FormAction::Left | FormAction::Right => Ok(None),
            }
        }
    }

    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        mut position: Position,
        mut size: Size,
        kind: MouseEventKind,
        modifier: KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        if position.x > 2
            && position.y > 2
            && position.x < size.width - 1
            && position.y < size.height - 1
        {
            position.x -= 2;
            position.y -= 2;
            size.width -= 4;
            size.height -= 4;
            let mut cur = ClickCurrent::<{ TOTAL_SIZE }> {
                kind,
                modifier,
                pos: position,
                cought: false,
                store: &self.store,
                size,
                offset: self.offset,
            };
            let res = Data::with_selection_mut_cx(
                &mut self.sel,
                cx.wrap_with(FormAction::Inner),
                &mut self.data,
                &mut cur,
            )?;
            if cur.cought {
                return Ok(res);
            }
            let mut cur = ClickItem::<{ TOTAL_SIZE }, Data::AR> {
                pos: position,
                res: None,
                size,
                store: &self.store,
                kind,
                modifier,
            };
            let index = find_index(&self.store, position);
            Data::with_index_mut(
                &mut self.sel,
                cx.wrap_with(FormAction::Inner),
                &mut self.data,
                index,
                &mut cur,
            )?;
            Ok(cur.res)
        } else {
            Ok(None)
        }
    }

    fn render_fallible_inner(
        &mut self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()> {
        let outer = Block::bordered()
            .title(Data::TITLE)
            .padding(Padding::uniform(1));
        let main = outer.inner(area);
        let show_if = Data::show_if(&self.data);

        let mut cur = CalcHeight::<{ TOTAL_SIZE }, Data> {
            store: &mut self.store,
            first: true,
            show_if: &show_if,
            height: 0,
            height_buf: 0,
            sel: PhantomData,
        };
        Data::with_iter::<R, _>(&self.data, &mut cur)?;
        let height = cur.height.strict_add(cur.height_buf);
        if main.height < height {
            let mut scroll_view = ScrollView::new((main.width, height).into());
            let area = scroll_view.area();
            self.offset =
                crate::offset::calc_offset(height, main.height, self.store[Data::index(&self.sel)]);
            let mut cur = Pass1::<{ TOTAL_SIZE }> {
                area,
                store: &self.store,
                show_if: &show_if,
                buf: scroll_view.buf_mut(),
                cur: Data::index(&self.sel),
            };
            Data::with_iter_mut(cx.wrap_with(FormAction::Inner), &mut self.data, &mut cur)?;
            scroll_view.render(
                main,
                buf,
                &mut ScrollViewState::with_offset((0, self.offset).into()),
            );
        } else {
            self.offset = 0;
            let mut cur = Pass1::<{ TOTAL_SIZE }> {
                area: main,
                store: &self.store,
                show_if: &show_if,
                buf,
                cur: Data::index(&self.sel),
            };
            Data::with_iter_mut(cx.wrap_with(FormAction::Inner), &mut self.data, &mut cur)?;
        }
        let cur = Pass2::<{ TOTAL_SIZE }> {
            area: main,
            store: &self.store,
            buf,
            offset: self.offset,
        };
        Data::with_selection_mut_cx(
            &mut self.sel,
            cx.wrap_with(FormAction::Inner),
            &mut self.data,
            cur,
        )?;
        outer.render(area, buf);
        Ok(())
    }
}

fn find_index(store: &[u16], position: Position) -> usize {
    if position.y == 0 {
        0
    } else {
        store.partition_point(|h| {
            let h = *h;
            h < position.y
        }) - 1
    }
}

impl<const TOTAL_SIZE: usize, Data: FormData<{ TOTAL_SIZE }>> Form<TOTAL_SIZE, Data> {
    fn dispatch_active_action<R: 'static>(
        &mut self,
        cx: WidgetContext<'_, FormAction<Data::Action>, impl Wrapper<FormAction<Data::Action>>, R>,
        action: FormAction<Infallible>,
    ) -> Result<Option<ControlFlow<Navigation, Data::AR>>> {
        Data::with_selection_mut_cx(
            &mut self.sel,
            cx.wrap_with(FormAction::Inner),
            &mut self.data,
            ApplyMovement(action),
        )
    }
}

pub trait WithSelection<R: 'static, AR, T> {
    fn with<const INDEX: usize, I: FormItem<R, AR>>(
        self,
        sel: &I::SelectionInner,
        state: &I,
        name: &'static str,
    ) -> T;
}

pub trait WithSelectionMut<R: 'static, AR, T> {
    fn with_mut<const INDEX: usize, I: FormItem<R, AR>>(
        self,
        sel: &mut I::SelectionInner,
        state: &mut I,
        name: &'static str,
    ) -> T;
}

pub trait WithSelectionMutCX<R: 'static, AR, T> {
    fn with_mut<const INDEX: usize, I: FormItem<R, AR>>(
        self,
        sel: &mut I::SelectionInner,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        name: &'static str,
    ) -> T;
}

pub trait WithIterItems<R: 'static, AR> {
    fn with<const INDEX: usize, I: FormItem<R, AR>>(
        &mut self,
        state: &I,
        name: &'static str,
    ) -> Result<()>;
}

pub trait WithIterItemsMut<R: 'static, AR> {
    fn with_mut<const INDEX: usize, I: FormItem<R, AR>>(
        &mut self,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        name: &'static str,
    ) -> Result<()>;
}

pub trait WithIndexMut<R: 'static, AR> {
    fn with_mut<const INDEX: usize, I: FormItem<R, AR>>(
        self,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        name: &'static str,
    ) -> Result<I::SelectionInner>;
}

pub trait WithActionMut<R: 'static, AR, T> {
    fn with_mut<const INDEX: usize, I: FormItem<R, AR>>(
        self,
        action: I::Action,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
    ) -> T;
}

struct AcceptsTextInput;

impl<R: 'static, AR> WithSelection<R, AR, bool> for AcceptsTextInput {
    fn with<const INDEX: usize, I: FormItem<R, AR>>(
        self,
        sel: &I::SelectionInner,
        state: &I,
        name: &'static str,
    ) -> bool {
        state.accepts_text_input(sel)
    }
}

struct ApplyChar(char);

impl<R: 'static, AR> WithSelectionMut<R, AR, ()> for ApplyChar {
    fn with_mut<const INDEX: usize, I: FormItem<R, AR>>(
        self,
        sel: &mut I::SelectionInner,
        state: &mut I,
        name: &'static str,
    ) {
        state.apply_char(sel, self.0);
    }
}

struct ApplyText(String);

impl<R: 'static, AR> WithSelectionMut<R, AR, ()> for ApplyText {
    fn with_mut<const INDEX: usize, I: FormItem<R, AR>>(
        self,
        sel: &mut I::SelectionInner,
        state: &mut I,
        name: &'static str,
    ) {
        state.apply_text(sel, self.0);
    }
}

struct ApplyMovement(FormAction<Infallible>);

impl<R: 'static, AR> WithSelectionMutCX<R, AR, Result<Option<ControlFlow<Navigation, AR>>>>
    for ApplyMovement
{
    fn with_mut<const INDEX: usize, I: FormItem<R, AR>>(
        self,
        sel: &mut I::SelectionInner,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        name: &'static str,
    ) -> Result<Option<ControlFlow<Navigation, AR>>> {
        Ok(state.apply_movement(sel, cx, self.0)?.map(|cf| match cf {
            ControlFlow::Continue(c) => ControlFlow::Continue(c.into()),
            ControlFlow::Break(n) => ControlFlow::Break(n),
        }))
    }
}

struct ApplyAction;

impl<R: 'static, AR> WithActionMut<R, AR, Result<Option<ControlFlow<Navigation, AR>>>>
    for ApplyAction
{
    fn with_mut<const INDEX: usize, I: FormItem<R, AR>>(
        self,
        action: I::Action,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
    ) -> Result<Option<ControlFlow<Navigation, AR>>> {
        Ok(state.apply_action(cx, action)?.map(|cf| match cf {
            ControlFlow::Continue(c) => ControlFlow::Continue(c.into()),
            ControlFlow::Break(n) => ControlFlow::Break(n),
        }))
    }
}

struct AcceptsMovementAction;

impl<R: 'static, AR> WithSelection<R, AR, bool> for AcceptsMovementAction {
    fn with<const INDEX: usize, I: FormItem<R, AR>>(
        self,
        sel: &I::SelectionInner,
        state: &I,
        name: &'static str,
    ) -> bool {
        state.accepts_movement_action(sel)
    }
}

struct SelectionDefault;

impl<R: 'static, AR> WithIndexMut<R, AR> for SelectionDefault {
    fn with_mut<const INDEX: usize, I: FormItem<R, AR>>(
        self,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        name: &'static str,
    ) -> Result<I::SelectionInner> {
        Ok(I::SelectionInner::default())
    }
}

struct ClickCurrent<'s, const TOTAL_SIZE: usize> {
    kind: MouseEventKind,
    modifier: KeyModifiers,
    pos: Position,
    cought: bool,
    store: &'s [u16],
    size: Size,
    offset: u16,
}

impl<'s, R: 'static, const TOTAL_SIZE: usize, AR>
    WithSelectionMutCX<R, AR, Result<Option<ControlFlow<Navigation, AR>>>>
    for &mut ClickCurrent<'s, { TOTAL_SIZE }>
{
    fn with_mut<const INDEX: usize, I: FormItem<R, AR>>(
        self,
        sel: &mut I::SelectionInner,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        name: &'static str,
    ) -> Result<Option<ControlFlow<Navigation, AR>>> {
        let this_area = Rect {
            x: 0,
            y: self.store[INDEX] - self.offset,
            width: self.size.width,
            height: I::HEIGHT,
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

struct ClickItem<'s, const TOTAL_SIZE: usize, AR> {
    pos: Position,
    res: Option<ControlFlow<Navigation, AR>>,
    size: Size,
    store: &'s [u16],
    kind: MouseEventKind,
    modifier: KeyModifiers,
}

impl<'s, R: 'static, const TOTAL_SIZE: usize, AR> WithIndexMut<R, AR>
    for &mut ClickItem<'s, TOTAL_SIZE, AR>
{
    fn with_mut<const INDEX: usize, I: FormItem<R, AR>>(
        self,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        name: &'static str,
    ) -> Result<I::SelectionInner> {
        let base = self.pos.y - self.store[INDEX];
        if base < I::HEIGHT {
            let (s, res) = I::apply_click_inactive(
                state,
                cx,
                Size {
                    width: self.size.width,
                    height: I::HEIGHT,
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

struct CalcHeight<'s, const TOTAL_SIZE: usize, S: FormData<{ TOTAL_SIZE }>> {
    store: &'s mut [u16; TOTAL_SIZE],
    show_if: &'s [bool; TOTAL_SIZE],
    first: bool,
    height: u16,
    height_buf: u16,
    sel: PhantomData<S>,
}

impl<const TOTAL_SIZE: usize, R: 'static, S: FormData<{ TOTAL_SIZE }>> WithIterItems<R, S::AR>
    for CalcHeight<'_, { TOTAL_SIZE }, S>
{
    fn with<const INDEX: usize, I: FormItem<R, S::AR>>(
        &mut self,
        state: &I,
        name: &'static str,
    ) -> Result<()> {
        if self.show_if[INDEX] {
            if self.first {
                self.first = false;
                self.store[INDEX] = self.height;
                self.height = I::HEIGHT;
                self.height_buf = I::HEIGHT_BUF;
            } else {
                self.height = self.height.strict_add(1);
                self.store[INDEX] = self.height;
                self.height = self.height.strict_add(I::HEIGHT);
                self.height_buf = self
                    .height_buf
                    .saturating_sub(1)
                    .saturating_sub(I::HEIGHT)
                    .strict_add(I::HEIGHT_BUF);
            }
        } else {
            self.store[INDEX] = self.height;
        }
        Ok(())
    }
}

struct Pass1<'s, const TOTAL_SIZE: usize> {
    area: Rect,
    store: &'s [u16; TOTAL_SIZE],
    show_if: &'s [bool; TOTAL_SIZE],
    buf: &'s mut Buffer,
    cur: usize,
}

impl<'s, const TOTAL_SIZE: usize, R: 'static, AR> WithIterItemsMut<R, AR>
    for Pass1<'s, { TOTAL_SIZE }>
{
    fn with_mut<const INDEX: usize, I: FormItem<R, AR>>(
        &mut self,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        name: &'static str,
    ) -> Result<()> {
        if *self.show_if.index(INDEX) {
            let mut this_area = self.area;
            this_area.height = I::HEIGHT;
            this_area.y += self.store[INDEX];
            I::render_pass_main(state, cx, this_area, self.buf, self.cur == INDEX, name)?;
        }
        Ok(())
    }
}

struct Pass2<'s, const TOTAL_SIZE: usize> {
    area: Rect,
    store: &'s [u16; TOTAL_SIZE],
    buf: &'s mut Buffer,
    offset: u16,
}

impl<'s, const TOTAL_SIZE: usize, R: 'static, AR> WithSelectionMutCX<R, AR, Result<()>>
    for Pass2<'s, { TOTAL_SIZE }>
{
    fn with_mut<const INDEX: usize, I: FormItem<R, AR>>(
        self,
        sel: &mut I::SelectionInner,
        cx: WidgetContext<'_, I::Action, impl Wrapper<I::Action>, R>,
        state: &mut I,
        name: &'static str,
    ) -> Result<()> {
        let mut this_area = self.area;
        this_area.height = I::HEIGHT;
        this_area.y += self.store[INDEX] - self.offset;
        I::render_pass_popup(state, cx, this_area, self.area, self.buf, name, sel)
    }
}

pub struct FormCommandMapper<I: Debug + Send + 'static> {
    _i: PhantomData<fn() -> I>,
}

impl<I: Debug + Send + 'static> Default for FormCommandMapper<I> {
    fn default() -> Self {
        Self {
            _i: Default::default(),
        }
    }
}

impl<I: Debug + Send + 'static> CommandMapper<FormCommand> for FormCommandMapper<I> {
    type A = FormAction<I>;

    fn map(&self, command: FormCommand) -> ControlFlow<Navigation, Self::A> {
        ControlFlow::Continue(match command {
            FormCommand::Quit => FormAction::Quit,
            FormCommand::Up => FormAction::Up,
            FormCommand::Down => FormAction::Down,
            FormCommand::Left => FormAction::Left,
            FormCommand::Right => FormAction::Right,
            FormCommand::Delete => FormAction::Delete,
            FormCommand::Enter => FormAction::Enter,
            FormCommand::Global(g) => return ControlFlow::Break(g.into()),
        })
    }
}
