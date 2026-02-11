use std::{convert::Infallible, fmt::Debug, marker::PhantomData, ops::Index};

use jellyhaj_widgets_core::{
    Buffer, JellyhajWidget, KeyModifiers, MouseEventKind, Position, Rect, Size, Wrapper,
    async_task::TaskSubmitter,
};
use ratatui::widgets::{Block, Padding, StatefulWidget, Widget};
use tui_scrollview::{ScrollView, ScrollViewState};

use crate::{FormAction, FormItem, QuitForm};
use color_eyre::Result;

pub trait FormState<const TOTAL_SIZE: usize> {
    type Selector;
    type AR: Debug + From<QuitForm> + From<Infallible>;
    fn with_selection<T, W: WithSelection<Self::AR, T>>(
        this: &Self::Selector,
        state: &Self,
        with: W,
    ) -> T;
    fn with_mut_selection<T, W: WithSelectionMut<Self::AR, T>>(
        this: &mut Self::Selector,
        state: &mut Self,
        with: W,
    ) -> T;
    fn with_index_mut<W: WithIndexMut<Self::AR>>(
        this: &mut Self::Selector,
        state: &mut Self,
        index: usize,
        with: W,
    ) -> Result<()>;
    fn with_iter<W: WithIterItems<Self::AR>>(state: &Self, with: &mut W) -> Result<()>;
    fn with_iter_mut<W: WithIterItemsMut<Self::AR>>(state: &mut Self, with: &mut W) -> Result<()>;
    fn show_if(state: &Self) -> [bool; TOTAL_SIZE];
    fn index(sel: &Self::Selector) -> usize;
    const TITLE: &str;

    fn make_widget_with<'s>(
        &'s mut self,
        selection: Self::Selector,
    ) -> Form<'s, { TOTAL_SIZE }, Self> {
        Form {
            sel: selection,
            state: self,
            store: [0; TOTAL_SIZE],
            offset: 0,
        }
    }
}

pub trait FormStateDefaultExt<const TOTAL_SIZE: usize>: FormState<TOTAL_SIZE> {
    fn make_widget_with_default<'s>(&'s mut self) -> Form<'s, { TOTAL_SIZE }, Self>;
}

impl<const TOTAL_SIZE: usize, F: FormState<TOTAL_SIZE>> FormStateDefaultExt<TOTAL_SIZE> for F
where
    F::Selector: Default,
{
    fn make_widget_with_default<'s>(&'s mut self) -> Form<'s, { TOTAL_SIZE }, Self> {
        Self::make_widget_with(self, Self::Selector::default())
    }
}

pub struct Form<'s, const TOTAL_SIZE: usize, State: FormState<{ TOTAL_SIZE }> + ?Sized> {
    sel: State::Selector,
    state: &'s mut State,
    store: [u16; TOTAL_SIZE],
    offset: u16,
}

impl<'s, const TOTAL_SIZE: usize, State: FormState<{ TOTAL_SIZE }>> JellyhajWidget
    for Form<'s, { TOTAL_SIZE }, State>
{
    type State = State::Selector;

    type Action = FormAction;

    type ActionResult = State::AR;

    fn min_width(&self) -> Option<u16> {
        Some(10)
    }

    fn min_height(&self) -> Option<u16> {
        Some(10)
    }

    fn into_state(self) -> Self::State {
        self.sel
    }

    fn accepts_text_input(&self) -> bool {
        State::with_selection(&self.sel, self.state, AcceptsTextInput)
    }

    fn accept_char(&mut self, text: char) {
        State::with_mut_selection(&mut self.sel, self.state, ApplyChar(text))
    }

    fn accept_text(&mut self, text: String) {
        State::with_mut_selection(&mut self.sel, self.state, ApplyText(text))
    }

    fn apply_action(
        &mut self,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        fn inner<const TOTAL_SIZE: usize, S: FormState<{ TOTAL_SIZE }>>(
            state: &mut S,
            sel: &mut S::Selector,
            action: FormAction,
        ) -> Result<Option<S::AR>> {
            S::with_mut_selection(sel, state, ApplyAction(action))
        }
        if State::with_selection(&self.sel, self.state, AcceptsMovementAction) {
            inner(self.state, &mut self.sel, action)
        } else {
            match action {
                FormAction::Up => {
                    let start = State::index(&self.sel);
                    let show_if = State::show_if(self.state);
                    let mut current = start;
                    let index = loop {
                        current = current.checked_sub(1).unwrap_or(TOTAL_SIZE.strict_sub(1));
                        if show_if[current] {
                            break current;
                        } else if current == start {
                            panic!("all form other than the current are hidden")
                        }
                    };
                    State::with_index_mut(&mut self.sel, self.state, index, SelectionDefault)?;
                    Ok(None)
                }
                FormAction::Down => {
                    let start = State::index(&self.sel);
                    let show_if = State::show_if(self.state);
                    let mut current = start;
                    let index = loop {
                        current = current.strict_add(1) % TOTAL_SIZE;
                        if show_if[current] {
                            break current;
                        } else if current == start {
                            panic!("all form other than the current are hidden")
                        }
                    };
                    State::with_index_mut(&mut self.sel, self.state, index, SelectionDefault)?;
                    Ok(None)
                }

                FormAction::Delete | FormAction::Enter => inner(self.state, &mut self.sel, action),
                FormAction::Quit => Ok(Some(QuitForm.into())),
                FormAction::Left | FormAction::Right => Ok(None),
            }
        }
    }

    fn click(
        &mut self,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
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
            let res = State::with_mut_selection(&mut self.sel, self.state, &mut cur)?;
            if cur.cought {
                return Ok(res);
            }
            let mut cur = ClickItem::<{ TOTAL_SIZE }, State::AR> {
                pos: position,
                res: None,
                size,
                store: &self.store,
                kind,
                modifier,
            };
            let index = {
                if position.y == 0 {
                    0
                } else {
                    self.store.partition_point(|h| {
                        let h = *h;
                        h < position.y
                    }) - 1
                }
            };
            State::with_index_mut(&mut self.sel, self.state, index, &mut cur)?;
            Ok(cur.res)
        } else {
            Ok(None)
        }
    }

    fn render_fallible_inner(
        &mut self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
    ) -> Result<()> {
        let outer = Block::bordered()
            .title(State::TITLE)
            .padding(Padding::uniform(1));
        let main = outer.inner(area);
        let show_if = State::show_if(self.state);

        let mut cur = CalcHeight::<{ TOTAL_SIZE }, State> {
            store: &mut self.store,
            first: true,
            show_if: &show_if,
            height: 0,
            height_buf: 0,
            sel: PhantomData,
        };
        State::with_iter(self.state, &mut cur)?;
        let height = cur.height.strict_add(cur.height_buf);
        if main.height < height {
            let mut scroll_view = ScrollView::new((main.width, height).into());
            let area = scroll_view.area();
            self.offset = crate::offset::calc_offset(
                height,
                main.height,
                self.store[State::index(&self.sel)],
            );
            let mut cur = Pass1::<{ TOTAL_SIZE }> {
                area,
                store: &self.store,
                show_if: &show_if,
                buf: scroll_view.buf_mut(),
                cur: State::index(&self.sel),
            };
            State::with_iter_mut(self.state, &mut cur)?;
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
                cur: State::index(&self.sel),
            };
            State::with_iter_mut(self.state, &mut cur)?;
        }
        let cur = Pass2::<{ TOTAL_SIZE }> {
            area: main,
            store: &self.store,
            buf,
            offset: self.offset,
        };
        State::with_mut_selection(&mut self.sel, self.state, cur)?;
        outer.render(area, buf);
        Ok(())
    }
}

pub trait WithSelection<AR, T> {
    fn with<const INDEX: usize, I: FormItem<AR>>(
        self,
        sel: &I::SelectionInner,
        state: &I,
        name: &'static str,
    ) -> T;
}

pub trait WithSelectionMut<AR, T> {
    fn with_mut<const INDEX: usize, I: FormItem<AR>>(
        self,
        sel: &mut I::SelectionInner,
        state: &mut I,
        name: &'static str,
    ) -> T;
}

pub trait WithIterItems<AR> {
    fn with<const INDEX: usize, I: FormItem<AR>>(
        &mut self,
        state: &I,
        name: &'static str,
    ) -> Result<()>;
}

pub trait WithIterItemsMut<AR> {
    fn with_mut<const INDEX: usize, I: FormItem<AR>>(
        &mut self,
        state: &mut I,
        name: &'static str,
    ) -> Result<()>;
}

pub trait WithIndexMut<AR> {
    fn with_mut<const INDEX: usize, I: FormItem<AR>>(
        self,
        state: &mut I,
        name: &'static str,
    ) -> Result<I::SelectionInner>;
}

struct AcceptsTextInput;

impl<AR> WithSelection<AR, bool> for AcceptsTextInput {
    fn with<const INDEX: usize, I: FormItem<AR>>(
        self,
        sel: &I::SelectionInner,
        state: &I,
        name: &'static str,
    ) -> bool {
        state.accepts_text_input(sel)
    }
}

struct ApplyChar(char);

impl<AR> WithSelectionMut<AR, ()> for ApplyChar {
    fn with_mut<const INDEX: usize, I: FormItem<AR>>(
        self,
        sel: &mut I::SelectionInner,
        state: &mut I,
        name: &'static str,
    ) {
        state.apply_char(sel, self.0);
    }
}

struct ApplyText(String);

impl<AR> WithSelectionMut<AR, ()> for ApplyText {
    fn with_mut<const INDEX: usize, I: FormItem<AR>>(
        self,
        sel: &mut I::SelectionInner,
        state: &mut I,
        name: &'static str,
    ) {
        state.apply_text(sel, self.0);
    }
}

struct ApplyAction(FormAction);

impl<AR> WithSelectionMut<AR, Result<Option<AR>>> for ApplyAction {
    fn with_mut<const INDEX: usize, I: FormItem<AR>>(
        self,
        sel: &mut I::SelectionInner,
        state: &mut I,
        name: &'static str,
    ) -> Result<Option<AR>> {
        Ok(state.apply_action(sel, self.0)?.map(Into::into))
    }
}

struct AcceptsMovementAction;

impl<AR> WithSelection<AR, bool> for AcceptsMovementAction {
    fn with<const INDEX: usize, I: FormItem<AR>>(
        self,
        sel: &I::SelectionInner,
        state: &I,
        name: &'static str,
    ) -> bool {
        state.accepts_movement_action(sel)
    }
}

struct SelectionDefault;

impl<AR> WithIndexMut<AR> for SelectionDefault {
    fn with_mut<const INDEX: usize, I: FormItem<AR>>(
        self,
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

impl<'s, const TOTAL_SIZE: usize, AR> WithSelectionMut<AR, Result<Option<AR>>>
    for &mut ClickCurrent<'s, { TOTAL_SIZE }>
{
    fn with_mut<const INDEX: usize, I: FormItem<AR>>(
        self,
        sel: &mut I::SelectionInner,
        state: &mut I,
        name: &'static str,
    ) -> Result<Option<AR>> {
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
                sel,
                this_area,
                self.size,
                self.pos,
                self.kind,
                self.modifier,
            )
            .map(|v| v.map(Into::into));
            self.cought = true;
            res
        } else {
            Ok(None)
        }
    }
}

struct ClickItem<'s, const TOTAL_SIZE: usize, AR> {
    pos: Position,
    res: Option<AR>,
    size: Size,
    store: &'s [u16],
    kind: MouseEventKind,
    modifier: KeyModifiers,
}

impl<'s, const TOTAL_SIZE: usize, AR> WithIndexMut<AR> for &mut ClickItem<'s, TOTAL_SIZE, AR> {
    fn with_mut<const INDEX: usize, I: FormItem<AR>>(
        self,
        state: &mut I,
        name: &'static str,
    ) -> Result<I::SelectionInner> {
        let base = self.pos.y - self.store[INDEX];
        if base < I::HEIGHT {
            let (s, res) = I::apply_click_inactive(
                state,
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
            self.res = res.map(Into::into);
            Ok(s.unwrap_or_default())
        } else {
            Ok(I::SelectionInner::default())
        }
    }
}

struct CalcHeight<'s, const TOTAL_SIZE: usize, S: FormState<{ TOTAL_SIZE }>> {
    store: &'s mut [u16; TOTAL_SIZE],
    show_if: &'s [bool; TOTAL_SIZE],
    first: bool,
    height: u16,
    height_buf: u16,
    sel: PhantomData<S>,
}

impl<const TOTAL_SIZE: usize, S: FormState<{ TOTAL_SIZE }>> WithIterItems<S::AR>
    for CalcHeight<'_, { TOTAL_SIZE }, S>
{
    fn with<const INDEX: usize, I: FormItem<S::AR>>(
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

impl<'s, const TOTAL_SIZE: usize, AR> WithIterItemsMut<AR> for Pass1<'s, { TOTAL_SIZE }> {
    fn with_mut<const INDEX: usize, I: FormItem<AR>>(
        &mut self,
        state: &mut I,
        name: &'static str,
    ) -> Result<()> {
        if *self.show_if.index(INDEX) {
            let mut this_area = self.area;
            this_area.height = I::HEIGHT;
            this_area.y += self.store[INDEX];
            I::render_pass_main(state, this_area, self.buf, self.cur == INDEX, name)?;
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

impl<'s, const TOTAL_SIZE: usize, AR> WithSelectionMut<AR, Result<()>>
    for Pass2<'s, { TOTAL_SIZE }>
{
    fn with_mut<const INDEX: usize, I: FormItem<AR>>(
        self,
        sel: &mut I::SelectionInner,
        state: &mut I,
        name: &'static str,
    ) -> Result<()> {
        let mut this_area = self.area;
        this_area.height = I::HEIGHT;
        this_area.y += self.store[INDEX] - self.offset;
        I::render_pass_popup(state, this_area, self.area, self.buf, name, sel)
    }
}
