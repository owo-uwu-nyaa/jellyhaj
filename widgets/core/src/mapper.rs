use std::{fmt::Debug, marker::PhantomData, ops::ControlFlow};

use color_eyre::eyre::Result;
use jellyhaj_async_task::Wrapper;
use ratatui::crossterm::event::{KeyModifiers, MouseEventKind};
use valuable::{Fields, NamedValues, StructDef, Structable, Valuable, Value, Visit};

use crate::{
    JellyhajWidget, KeybindAction, RenderFlag, WidgetContext, WidgetTreeVisitor,
    jellyhaj::JellyhajWidgetBase,
};

pub trait Named: 'static {
    const NAME: &str;
}

pub trait ResultMapper<I> {
    type R: Debug + 'static;
    fn map(res: I) -> Result<Option<Self::R>>;
}

pub struct ResultMapperWidget<N: Named, W: JellyhajWidgetBase, M: 'static> {
    pub inner: W,
    named: PhantomData<fn(N) -> ()>,
    mapper: PhantomData<fn(M) -> ()>,
}

impl<N: Named, W: JellyhajWidgetBase, M> ResultMapperWidget<N, W, M> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            named: PhantomData,
            mapper: PhantomData,
        }
    }
}

impl<N: Named, W: JellyhajWidgetBase, M> Valuable for ResultMapperWidget<N, W, M> {
    fn as_value(&self) -> Value<'_> {
        Value::Structable(self)
    }

    fn visit(&self, visit: &mut dyn Visit) {
        visit.visit_named_fields(&NamedValues::new(&[], &[]));
    }
}

impl<N: Named, W: JellyhajWidgetBase, M> Structable for ResultMapperWidget<N, W, M> {
    fn definition(&self) -> StructDef<'_> {
        StructDef::new_static("MapperState", Fields::Named(&[]))
    }
}

impl<N: Named, W: JellyhajWidgetBase, M: ResultMapper<W::ActionResult>> JellyhajWidgetBase
    for ResultMapperWidget<N, W, M>
{
    type Action = W::Action;

    type ActionResult = M::R;

    const NAME: &str = N::NAME;

    fn visit_children(&self, visitor: &mut impl WidgetTreeVisitor) {
        visitor.visit::<W>(&self.inner);
    }

    fn min_width(&self) -> Option<u16> {
        self.inner.min_width()
    }
    fn min_height(&self) -> Option<u16> {
        self.inner.min_height()
    }

    fn accepts_text_input(&self) -> bool {
        self.inner.accepts_text_input()
    }
    fn accept_char(&mut self, text: char, render_flag: &mut RenderFlag) {
        self.inner.accept_char(text, render_flag);
    }
    fn accept_text(&mut self, text: String, render_flag: &mut RenderFlag) {
        self.inner.accept_text(text, render_flag);
    }
}

impl<R: 'static, N: Named, W: JellyhajWidget<R>, M: ResultMapper<W::ActionResult>> JellyhajWidget<R>
    for ResultMapperWidget<N, W, M>
{
    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {
        self.inner.init(cx);
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<Self::ActionResult>> {
        match self.inner.apply_action(cx, action, render_flag)? {
            None => Ok(None),
            Some(v) => M::map(v),
        }
    }

    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        position: ratatui::prelude::Position,
        size: ratatui::prelude::Size,
        kind: ratatui::crossterm::event::MouseEventKind,
        modifier: ratatui::crossterm::event::KeyModifiers,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<Self::ActionResult>> {
        match self
            .inner
            .click(cx, position, size, kind, modifier, render_flag)?
        {
            None => Ok(None),
            Some(v) => M::map(v),
        }
    }

    fn render_fallible_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()> {
        self.inner.render_fallible_inner(area, buf, cx)
    }
}
pub trait ActionMapperBase<W: JellyhajWidgetBase>: Valuable + Send + 'static {
    type Action: Debug + Send + 'static;
}

pub trait ActionMapper<R: 'static, W: JellyhajWidget<R>>: ActionMapperBase<W> {
    fn init(
        &mut self,
        this: &mut W,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        this_cx: WidgetContext<
            '_,
            <W as JellyhajWidgetBase>::Action,
            impl Wrapper<<W as JellyhajWidgetBase>::Action>,
            R,
        >,
    );
    fn map_action(
        &mut self,
        this: &mut W,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        this_cx: WidgetContext<
            '_,
            <W as JellyhajWidgetBase>::Action,
            impl Wrapper<<W as JellyhajWidgetBase>::Action>,
            R,
        >,
        action: Self::Action,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<<W as JellyhajWidgetBase>::ActionResult>>;
}

pub trait KeybindActionWidgetBase:
    JellyhajWidgetBase<Action = KeybindAction<Self::InnerAction>>
{
    type InnerAction: Debug + Send + 'static;
}
impl<A: Debug + Send + 'static, W: JellyhajWidgetBase<Action = KeybindAction<A>>>
    KeybindActionWidgetBase for W
{
    type InnerAction = A;
}

#[derive(Valuable)]
pub struct ActionMapperWidget<N: Named, W: KeybindActionWidgetBase, A: ActionMapperBase<W>> {
    #[valuable(skip)]
    inner: W,
    wrapper: A,
    #[valuable(skip)]
    name: PhantomData<fn(N) -> N>,
}

impl<N: Named, W: KeybindActionWidgetBase, A: ActionMapperBase<W>> ActionMapperWidget<N, W, A> {
    pub fn new(inner: W, wrapper: A) -> Self {
        Self {
            inner,
            wrapper,
            name: PhantomData,
        }
    }
}

impl<N: Named, W: KeybindActionWidgetBase, A: ActionMapperBase<W>> JellyhajWidgetBase
    for ActionMapperWidget<N, W, A>
{
    type Action = KeybindAction<ControlFlow<A::Action, W::InnerAction>>;

    type ActionResult = W::ActionResult;

    const NAME: &str = N::NAME;

    fn visit_children(&self, visitor: &mut impl WidgetTreeVisitor) {
        self.inner.visit_children(visitor);
    }

    fn min_width(&self) -> Option<u16> {
        self.inner.min_width()
    }

    fn min_height(&self) -> Option<u16> {
        self.inner.min_height()
    }

    fn accepts_text_input(&self) -> bool {
        self.inner.accepts_text_input()
    }

    fn accept_char(&mut self, text: char, render_flag: &mut RenderFlag) {
        self.inner.accept_char(text, render_flag);
    }

    fn accept_text(&mut self, text: String, render_flag: &mut RenderFlag) {
        self.inner.accept_text(text, render_flag);
    }
}

struct Wrapper1<A1, A2>(PhantomData<fn(A1) -> A1>, PhantomData<fn(A2) -> A2>);

impl<A1, A2> Wrapper1<A1, A2> {
    fn new() -> Self {
        Self(PhantomData, PhantomData)
    }
}

impl<A1, A2> Clone for Wrapper1<A1, A2> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A1, A2> Copy for Wrapper1<A1, A2> {}

impl<A1: Debug + Send + 'static, A2: Debug + Send + 'static> Wrapper<KeybindAction<A1>>
    for Wrapper1<A1, A2>
{
    type F = KeybindAction<ControlFlow<A2, A1>>;

    fn wrap(&self, val: KeybindAction<A1>) -> Self::F {
        match val {
            KeybindAction::Inner(v) => KeybindAction::Inner(ControlFlow::Continue(v)),
            KeybindAction::Key(key_event) => KeybindAction::Key(key_event),
        }
    }
}

struct Wrapper2<A1, A2>(PhantomData<fn(A1) -> A1>, PhantomData<fn(A2) -> A2>);

impl<A1, A2> Wrapper2<A1, A2> {
    fn new() -> Self {
        Self(PhantomData, PhantomData)
    }
}

impl<A1, A2> Clone for Wrapper2<A1, A2> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A1, A2> Copy for Wrapper2<A1, A2> {}

impl<A1: Debug + Send + 'static, A2: Debug + Send + 'static> Wrapper<A2> for Wrapper2<A1, A2> {
    type F = KeybindAction<ControlFlow<A2, A1>>;

    fn wrap(&self, val: A2) -> Self::F {
        KeybindAction::Inner(ControlFlow::Break(val))
    }
}

impl<R: 'static, N: Named, W: JellyhajWidget<R> + KeybindActionWidgetBase, A: ActionMapper<R, W>>
    JellyhajWidget<R> for ActionMapperWidget<N, W, A>
{
    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {
        self.inner.init(cx.wrap_with(Wrapper1::new()));
        self.wrapper.init(
            &mut self.inner,
            cx.wrap_with(Wrapper2::new()),
            cx.wrap_with(Wrapper1::new()),
        );
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<Self::ActionResult>> {
        let inner = match action {
            KeybindAction::Key(k) => KeybindAction::Key(k),
            KeybindAction::Inner(ControlFlow::Continue(a)) => KeybindAction::Inner(a),
            KeybindAction::Inner(ControlFlow::Break(a)) => {
                return self.wrapper.map_action(
                    &mut self.inner,
                    cx.wrap_with(Wrapper2::new()),
                    cx.wrap_with(Wrapper1::new()),
                    a,
                    render_flag,
                );
            }
        };
        self.inner
            .apply_action(cx.wrap_with(Wrapper1::new()), inner, render_flag)
    }

    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        position: ratatui::prelude::Position,
        size: ratatui::prelude::Size,
        kind: MouseEventKind,
        modifier: KeyModifiers,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<Self::ActionResult>> {
        self.inner.click(
            cx.wrap_with(Wrapper1::new()),
            position,
            size,
            kind,
            modifier,
            render_flag,
        )
    }

    fn render_fallible_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()> {
        self.inner
            .render_fallible_inner(area, buf, cx.wrap_with(Wrapper1::new()))
    }
}
