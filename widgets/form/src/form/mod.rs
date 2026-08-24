pub mod component;
pub mod helpers;

use std::{cmp::Ordering, convert::Infallible, fmt::Debug, marker::PhantomData, ops::ControlFlow};

use jellyhaj_core::{CommandMapper, keybinds::FormCommand, state::Navigation};
use jellyhaj_widgets_core::{
    JellyhajWidget, JellyhajWidgetBase, KeyModifiers, MouseEventKind, Position, Rect, RenderFlag,
    Size, WidgetContext, Wrapper,
    valuable::{Fields, NamedField, NamedValues, StructDef, Structable, Valuable, Value},
};
use ratatui::widgets::{Block, Padding, StatefulWidget, Widget};
use tracing::{instrument, trace};
use tui_scrollview::{ScrollView, ScrollViewState};

use crate::{
    FormAction,
    form::{
        component::FormComponent,
        helpers::{
            AcceptsMovementAction, AcceptsTextInput, ApplyAction, ApplyChar, ApplyText, CalcHeight,
            ClickCurrent, ClickItem, Pass1, Pass2, SelectionDefault,
        },
    },
};
use color_eyre::Result;

pub trait FormResultMapper<S: FormData> {
    type Res: Debug;
    fn map(
        state: &mut Form<S>,
        form_result: <S as FormComponent>::AR,
        cx: WidgetContext<
            '_,
            FormAction<<S as FormComponent>::Action>,
            impl Wrapper<FormAction<<S as FormComponent>::Action>>,
            (),
        >,
        render_flag: &mut jellyhaj_widgets_core::RenderFlag,
    ) -> Result<Option<Self::Res>>;
}

pub struct IdFormResultMapper;
impl<S: FormData> FormResultMapper<S> for IdFormResultMapper {
    type Res = S::AR;

    fn map(
        _state: &mut Form<S>,
        form_result: S::AR,
        cx: WidgetContext<'_, FormAction<S::Action>, impl Wrapper<FormAction<S::Action>>, ()>,
        _render_flag: &mut jellyhaj_widgets_core::RenderFlag,
    ) -> Result<Option<Self::Res>> {
        Ok(Some(form_result))
    }
}

pub trait FormData: FormComponent {
    type Mapper;
    const TITLE: &str;
}

pub trait FormDataExt: FormData {
    fn make_with_default(self) -> Form<Self>;
    fn make_with(self, selection: Self::Selector) -> Form<Self>;
}

impl<F: FormData> FormDataExt for F {
    fn make_with_default(self) -> Form<Self> {
        Self::make_with(self, Self::Selector::default())
    }
    fn make_with(self, selection: Self::Selector) -> Form<Self> {
        let store = Vec::with_capacity(self.total_size());
        Form {
            sel: selection,
            data: self,
            store,
            offset: 0,
        }
    }
}

pub struct Form<Data: FormData> {
    pub sel: Data::Selector,
    pub data: Data,
    store: Vec<u16>,
    offset: u16,
}

impl<Data: FormData> Form<Data> {
    pub fn up<R: 'static>(
        &mut self,
        cx: WidgetContext<'_, FormAction<Data::Action>, impl Wrapper<FormAction<Data::Action>>, R>,
        render_flag: &mut RenderFlag,
    ) -> Result<()> {
        let start = Data::index(&self.data, &self.sel);
        let mut current = start;
        let index = loop {
            current = current
                .checked_sub(1)
                .unwrap_or_else(|| self.data.total_size().strict_sub(1));
            if self.data.show_if(current) {
                break current;
            } else if current == start {
                panic!("all form other than the current are hidden")
            }
        };
        self.data.with_index_mut(
            0,
            &mut self.sel,
            cx.wrap_with(FormAction::Inner),
            index,
            SelectionDefault,
        )?;
        trace!(selector=?&self.sel, "form moved up");
        render_flag.set();
        Ok(())
    }

    pub fn down<R: 'static>(
        &mut self,
        cx: WidgetContext<'_, FormAction<Data::Action>, impl Wrapper<FormAction<Data::Action>>, R>,
        render_flag: &mut RenderFlag,
    ) -> Result<()> {
        let start = self.data.index(&self.sel);
        let mut current = start;
        let index = loop {
            current = current.strict_add(1) % self.data.total_size();
            if self.data.show_if(current) {
                break current;
            } else if current == start {
                panic!("all form other than the current are hidden")
            }
        };
        self.data.with_index_mut(
            0,
            &mut self.sel,
            cx.wrap_with(FormAction::Inner),
            index,
            SelectionDefault,
        )?;
        trace!(selector=?&self.sel, "form moved down");
        render_flag.set();
        Ok(())
    }
}

static FORM_FIELDS: &[NamedField] = &[NamedField::new("sel"), NamedField::new("data")];

impl<Data: FormData> Valuable for Form<Data> {
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
impl<Data: FormData> Structable for Form<Data> {
    fn definition(&self) -> StructDef<'_> {
        StructDef::new_static("Form", Fields::Named(FORM_FIELDS))
    }
}

impl<Mapper: FormResultMapper<Data>, Data: FormData<Mapper = Mapper>> JellyhajWidgetBase
    for Form<Data>
{
    type Action = FormAction<Data::Action>;

    type ActionResult = ControlFlow<Navigation, Mapper::Res>;

    const NAME: &str = "form";

    fn visit_children(&self, visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {}

    fn min_width(&self) -> Option<u16> {
        Some(10)
    }

    fn min_height(&self) -> Option<u16> {
        Some(10)
    }

    fn accepts_text_input(&self) -> bool {
        self.data.with_selection(0, &self.sel, AcceptsTextInput)
    }

    fn accept_char(&mut self, text: char, render_flag: &mut RenderFlag) {
        self.data
            .with_selection_mut(0, &mut self.sel, ApplyChar { text, render_flag });
    }

    fn accept_text(&mut self, text: String, render_flag: &mut RenderFlag) {
        self.data
            .with_selection_mut(0, &mut self.sel, ApplyText { text, render_flag });
    }
}

impl<R: 'static, Mapper: FormResultMapper<Data>, Data: FormData<Mapper = Mapper>> JellyhajWidget<R>
    for Form<Data>
{
    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {}

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<Self::ActionResult>> {
        let res = 'res: {
            let action: FormAction<Infallible> = match action {
                FormAction::Quit => FormAction::Quit,
                FormAction::Up => FormAction::Up,
                FormAction::Down => FormAction::Down,
                FormAction::Left => FormAction::Left,
                FormAction::Right => FormAction::Right,
                FormAction::Delete => FormAction::Delete,
                FormAction::Enter => FormAction::Enter,
                FormAction::Inner(action) => {
                    break 'res self.data.with_action_mut(
                        0,
                        action,
                        cx.wrap_with(FormAction::Inner),
                        ApplyAction(render_flag),
                    );
                }
            };
            if self
                .data
                .with_selection(0, &self.sel, AcceptsMovementAction)
            {
                self.dispatch_active_action(cx, action, render_flag)
            } else {
                match action {
                    FormAction::Up => {
                        self.up(cx, render_flag)?;
                        Ok(None)
                    }
                    FormAction::Down => {
                        self.down(cx, render_flag)?;
                        Ok(None)
                    }

                    FormAction::Delete => {
                        self.dispatch_active_action(cx, FormAction::Delete, render_flag)
                    }
                    FormAction::Enter => {
                        self.dispatch_active_action(cx, FormAction::Enter, render_flag)
                    }
                    FormAction::Quit => Ok(Some(ControlFlow::Break(Navigation::PopContext))),
                    FormAction::Left | FormAction::Right => Ok(None),
                }
            }
        };
        Ok(match res? {
            None => None,
            Some(ControlFlow::Break(v)) => Some(ControlFlow::Break(v)),
            Some(ControlFlow::Continue(v)) => {
                Mapper::map(self, v, cx.with_cx(&()), render_flag)?.map(ControlFlow::Continue)
            }
        })
    }

    #[instrument(skip(self, cx, render_flag), name = "click_form")]
    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        mut position: Position,
        mut size: Size,
        kind: MouseEventKind,
        modifier: KeyModifiers,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<Self::ActionResult>> {
        let res = 'res: {
            if position.x > 2
                && position.y > 2
                && position.x < size.width - 1
                && position.y < size.height - 1
            {
                position.x -= 2;
                position.y -= 2;
                size.width -= 4;
                size.height -= 4;
                trace!(?size, ?position, "inner geometry");
                let mut cur = ClickCurrent {
                    kind,
                    modifier,
                    pos: position,
                    cought: false,
                    store: &self.store,
                    size,
                    offset: self.offset,
                    render_flag,
                };
                let res = self.data.with_selection_mut_cx(
                    0,
                    &mut self.sel,
                    cx.wrap_with(FormAction::Inner),
                    &mut cur,
                )?;
                if cur.cought {
                    trace!("click caught");
                    break 'res res;
                }

                if kind.is_down() {
                    let index = find_index(&self.store, position.y);
                    trace!(index, "found clicked widget");
                    let cur_index = self.data.index(&self.sel);
                    if index != cur_index {
                        render_flag.set();
                    }
                    let mut cur = ClickItem::<Data::AR> {
                        pos: position,
                        res: None,
                        size,
                        store: &self.store,
                        kind,
                        modifier,
                        render_flag,
                    };
                    self.data.with_index_mut(
                        0,
                        &mut self.sel,
                        cx.wrap_with(FormAction::Inner),
                        index,
                        &mut cur,
                    )?;
                    cur.res
                } else {
                    None
                }
            } else {
                None
            }
        };
        Ok(match res {
            None => None,
            Some(ControlFlow::Break(v)) => Some(ControlFlow::Break(v)),
            Some(ControlFlow::Continue(v)) => {
                Mapper::map(self, v, cx.with_cx(&()), render_flag)?.map(ControlFlow::Continue)
            }
        })
    }

    #[instrument(skip_all, name = "render_form")]
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

        let mut cur = CalcHeight::<Data> {
            store: &mut self.store,
            height: 0,
            height_buf: 0,
            data: &self.data,
        };
        self.data.with_iter::<R, _>(0, &mut cur)?;
        let height = cur.height.strict_add(cur.height_buf);
        trace!(height, "calculated total required height");
        if main.height < height {
            let mut scroll_view = ScrollView::new((main.width, height).into());
            let area = scroll_view.area();
            self.offset = crate::offset::calc_offset(
                height,
                main.height,
                self.store[self.data.index(&self.sel)],
            );
            let mut cur = Pass1 {
                area,
                store: &self.store,
                buf: scroll_view.buf_mut(),
                cur: self.data.index(&self.sel),
            };
            self.data
                .with_iter_mut(0, cx.wrap_with(FormAction::Inner), &mut cur, true)?;
            scroll_view.render(
                main,
                buf,
                &mut ScrollViewState::with_offset((0, self.offset).into()),
            );
        } else {
            self.offset = 0;
            let mut cur = Pass1 {
                area: main,
                store: &self.store,
                buf,
                cur: self.data.index(&self.sel),
            };
            self.data
                .with_iter_mut(0, cx.wrap_with(FormAction::Inner), &mut cur, true)?;
        }
        let cur = Pass2 {
            area: main,
            store: &self.store,
            buf,
            offset: self.offset,
        };
        self.data
            .with_selection_mut_cx(0, &mut self.sel, cx.wrap_with(FormAction::Inner), cur)?;
        outer.render(area, buf);
        Ok(())
    }
}

#[inline(never)]
fn find_index(store: &[u16], y: u16) -> usize {
    let mut last = u16::MAX;
    let mut index = 0;
    for (i, v) in store.iter().copied().enumerate().filter(|(_, v)| {
        let res = last != *v;
        last = *v;
        res
    }) {
        match v.cmp(&y) {
            Ordering::Less => index = i,
            Ordering::Equal => return i,
            Ordering::Greater => {
                return index;
            }
        }
    }
    index
}

#[cfg(test)]
mod tests {

    use crate::form::find_index;

    #[test]
    fn test_find_index() {
        assert_eq!(0, find_index(&[0, 2, 2, 4], 0));
        assert_eq!(0, find_index(&[0, 0, 2, 2, 4], 0));
        assert_eq!(0, find_index(&[0, 2, 2, 4], 1));
        assert_eq!(0, find_index(&[0, 0, 2, 2, 4], 1));
        assert_eq!(2, find_index(&[0, 0, 2, 4], 2));
        assert_eq!(2, find_index(&[0, 0, 2, 2, 4], 2));
        assert_eq!(2, find_index(&[0, 0, 2, 4], 3));
        assert_eq!(2, find_index(&[0, 0, 2, 2, 4], 3));
        assert_eq!(3, find_index(&[0, 0, 2, 4], 4));
        assert_eq!(3, find_index(&[0, 0, 2, 4], 5));
    }
}

pub struct FormCommandMapper<I: Debug + Send + 'static> {
    _i: PhantomData<fn() -> I>,
}

impl<I: Debug + Send + 'static> Default for FormCommandMapper<I> {
    fn default() -> Self {
        Self { _i: PhantomData }
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
