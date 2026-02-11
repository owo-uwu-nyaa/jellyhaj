use std::{convert::Infallible, fmt::Debug, marker::PhantomData, ops::Index};

use jellyhaj_widgets_core::{
    Buffer, JellyhajWidget, KeyModifiers, MouseEventKind, Position, Rect, Size, Wrapper,
    async_task::TaskSubmitter,
};
use ratatui::widgets::{Block, Padding, StatefulWidget, Widget};
use tui_scrollview::{ScrollView, ScrollViewState};

use crate::{FormAction, FormItem, QuitForm};
use color_eyre::Result;

pub trait FormSelector<const TOTAL_SIZE: usize> {
    type State;
    type AR: Debug + From<QuitForm> + From<Infallible>;
    fn with_selection<W: WithSelection<Self::AR>>(this: &Self, state: &Self::State, with: &mut W);
    fn with_mut_selection<W: WithSelectionMut<Self::AR>>(
        this: &mut Self,
        state: &mut Self::State,
        with: &mut W,
    );
    fn with_index_mut<W: WithIterItemsMut<Self::AR>>(
        state: &mut Self::State,
        index: usize,
        with: &mut W,
    ) -> Result<()>;
    fn with_iter<W: WithIterItems<Self::AR>>(state: &Self::State, with: &mut W) -> Result<()>;
    fn with_iter_mut<W: WithIterItemsMut<Self::AR>>(
        state: &mut Self::State,
        with: &mut W,
    ) -> Result<()>;
    fn show_if(state: &Self::State) -> [bool; TOTAL_SIZE];
    fn index(&self) -> usize;
    const TITLE: &str;
}

pub struct Form<const TOTAL_SIZE: usize, Selector: FormSelector<{ TOTAL_SIZE }>> {
    sel: Selector,
    state: Selector::State,
    store: [u16; TOTAL_SIZE],
    offset: u16,
}

impl<const TOTAL_SIZE: usize, Selector: FormSelector<{ TOTAL_SIZE }>> JellyhajWidget
    for Form<{ TOTAL_SIZE }, Selector>
{
    type State = Selector;

    type Action = FormAction;

    type ActionResult = Selector::AR;

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
        todo!()
    }

    fn accept_char(&mut self, text: char) {
        todo!()
    }

    fn accept_text(&mut self, text: String) {
        todo!()
    }

    fn apply_action(
        &mut self,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        todo!()
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
            let mut cur = ClickCurrent::<{ TOTAL_SIZE }, Selector::AR> {
                kind,
                modifier,
                pos: position,
                res: Ok(None),
                cought: false,
                store: &self.store,
                size,
                offset: self.offset,
            };
            Selector::with_mut_selection(&mut self.sel, &mut self.state, &mut cur);
            let res = cur.res?;
            if cur.cought {
                return Ok(res);
            }
            let mut cur = ClickItem::<{ TOTAL_SIZE }, Selector::AR> {
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
            Selector::with_index_mut(&mut self.state, index, &mut cur)?;
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
            .title(Selector::TITLE)
            .padding(Padding::uniform(1));
        let main = outer.inner(area);
        let show_if = Selector::show_if(&self.state);

        let mut cur = CalcHeight::<{ TOTAL_SIZE }, Selector> {
            store: &mut self.store,
            first: true,
            show_if: &show_if,
            height: 0,
            height_buf: 0,
            sel: PhantomData,
        };
        Selector::with_iter(&self.state, &mut cur)?;
        let height = cur.height.strict_add(cur.height_buf);
        if main.height < height {
            let mut scroll_view = ScrollView::new((main.width, height).into());
            let area = scroll_view.area();
            self.offset = crate::macro_impl::offset::calc_offset(
                height,
                main.height,
                self.store[self.sel.index()],
            );
            let mut cur = Pass1::<{ TOTAL_SIZE }> {
                area,
                store: &self.store,
                show_if: &show_if,
                buf: scroll_view.buf_mut(),
                cur: self.sel.index(),
            };
            Selector::with_iter_mut(&mut self.state, &mut cur)?;
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
                cur: self.sel.index(),
            };
            Selector::with_iter_mut(&mut self.state, &mut cur)?;
        }
        let mut cur = Pass2::<{ TOTAL_SIZE }> {
            area: main,
            store: &self.store,
            buf,
            offset: self.offset,
            res: Ok(()),
        };
        Selector::with_mut_selection(&mut self.sel, &mut self.state, &mut cur);
        outer.render(area, buf);
        Ok(())
    }
}

pub trait WithSelection<AR> {
    fn with<const INDEX: usize, R: Into<AR>, I: FormItem<R>>(
        &mut self,
        sel: &I::SelectionInner,
        state: &I,
        name: &'static str,
    );
}

pub trait WithSelectionMut<AR> {
    fn with_mut<const INDEX: usize, R: Into<AR>, I: FormItem<R>>(
        &mut self,
        sel: &mut I::SelectionInner,
        state: &mut I,
        name: &'static str,
    ) -> Option<I::SelectionInner>;
}

pub trait WithIterItems<AR> {
    fn with_mut<const INDEX: usize, R: Into<AR>, I: FormItem<R>>(
        &mut self,
        state: &I,
        name: &'static str,
        offset: &u16,
    ) -> Result<()>;
}

pub trait WithIterItemsMut<AR> {
    fn with_mut<const INDEX: usize, R: Into<AR>, I: FormItem<R>>(
        &mut self,
        state: &mut I,
        name: &'static str,
    ) -> Result<Option<I::SelectionInner>>;
}

pub struct ClickCurrent<'s, const TOTAL_SIZE: usize, AR> {
    kind: MouseEventKind,
    modifier: KeyModifiers,
    pos: Position,
    res: Result<Option<AR>>,
    cought: bool,
    store: &'s [u16],
    size: Size,
    offset: u16,
}

impl<'s, const TOTAL_SIZE: usize, AR> WithSelectionMut<AR>
    for ClickCurrent<'s, { TOTAL_SIZE }, AR>
{
    fn with_mut<const INDEX: usize, R: Into<AR>, I: FormItem<R>>(
        &mut self,
        sel: &mut I::SelectionInner,
        state: &mut I,
        name: &'static str,
    ) -> Option<I::SelectionInner> {
        let this_area = Rect {
            x: 0,
            y: self.store[INDEX] - self.offset,
            width: self.size.width,
            height: I::HEIGHT,
        };
        let active = I::popup_area(state, *sel, this_area, self.size);
        if (active.height - active.y > self.pos.y) && (active.width - active.x > self.pos.x) {
            self.res = I::apply_click_active(
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
        }
        None
    }
}

pub struct ClickItem<'s, const TOTAL_SIZE: usize, AR> {
    pos: Position,
    res: Option<AR>,
    size: Size,
    store: &'s [u16],
    kind: MouseEventKind,
    modifier: KeyModifiers,
}

impl<'s, const TOTAL_SIZE: usize, AR> WithIterItemsMut<AR> for ClickItem<'s, TOTAL_SIZE, AR> {
    fn with_mut<const INDEX: usize, R: Into<AR>, I: FormItem<R>>(
        &mut self,
        state: &mut I,
        name: &'static str,
    ) -> Result<Option<I::SelectionInner>> {
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
            Ok(s)
        } else {
            Ok(None)
        }
    }
}

struct CalcHeight<'s, const TOTAL_SIZE: usize, S: FormSelector<{ TOTAL_SIZE }>> {
    store: &'s mut [u16; TOTAL_SIZE],
    show_if: &'s [bool; TOTAL_SIZE],
    first: bool,
    height: u16,
    height_buf: u16,
    sel: PhantomData<S>,
}

impl<const TOTAL_SIZE: usize, S: FormSelector<{ TOTAL_SIZE }>> WithIterItems<S::AR>
    for CalcHeight<'_, { TOTAL_SIZE }, S>
{
    fn with_mut<const INDEX: usize, R: Into<S::AR>, I: FormItem<R>>(
        &mut self,
        state: &I,
        name: &'static str,
        offset: &u16,
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
    fn with_mut<const INDEX: usize, R: Into<AR>, I: FormItem<R>>(
        &mut self,
        state: &mut I,
        name: &'static str,
    ) -> Result<Option<I::SelectionInner>> {
        if *self.show_if.index(INDEX) {
            let mut this_area = self.area;
            this_area.height = I::HEIGHT;
            this_area.y += self.store[INDEX];
            I::render_pass_main(state, this_area, self.buf, self.cur == INDEX, name)?;
        }
        Ok(None)
    }
}

struct Pass2<'s, const TOTAL_SIZE: usize> {
    area: Rect,
    store: &'s [u16; TOTAL_SIZE],
    buf: &'s mut Buffer,
    offset: u16,
    res: Result<()>,
}

impl<'s, const TOTAL_SIZE: usize, AR> WithSelectionMut<AR> for Pass2<'s, { TOTAL_SIZE }> {
    fn with_mut<const INDEX: usize, R: Into<AR>, I: FormItem<R>>(
        &mut self,
        sel: &mut I::SelectionInner,
        state: &mut I,
        name: &'static str,
    ) -> Option<I::SelectionInner> {
        let mut this_area = self.area;
        this_area.height = I::HEIGHT;
        this_area.y += self.store[INDEX] - self.offset;
        self.res = I::render_pass_popup(state, this_area, self.area, self.buf, name, *sel);
        None
    }
}
