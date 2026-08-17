use jellyhaj_core::widgets::KeybindAction;
use jellyhaj_widgets_core::{
    JellyhajWidget, JellyhajWidgetBase, Result, WidgetContext, Wrapper, outer::Named,
};
use std::{fmt::Debug, marker::PhantomData, ops::ControlFlow};
use valuable::Valuable;

pub trait ActionWrapperBase<W: JellyhajWidgetBase>: Valuable + Send + 'static {
    type Action: Debug + Send + 'static;
}

pub trait ActionWrapper<R: 'static, W: JellyhajWidget<R>>: ActionWrapperBase<W> {
    fn init(
        &mut self,
        this: &mut W,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    );
    fn map_action(
        &mut self,
        this: &mut W,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<W::ActionResult>>;
}

#[derive(Valuable)]
pub struct ActionWrapperWidget<
    Action: Send + Debug + 'static,
    N: Named,
    W: JellyhajWidgetBase<Action = KeybindAction<Action>>,
    A: ActionWrapperBase<W>,
> {
    #[valuable(skip)]
    inner: W,
    wrapper: A,
    #[valuable(skip)]
    name: PhantomData<fn(N) -> N>,
}

impl<
    Action: Send + Debug + 'static,
    N: Named,
    W: JellyhajWidgetBase<Action = KeybindAction<Action>>,
    A: ActionWrapperBase<W>,
> JellyhajWidgetBase for ActionWrapperWidget<Action, N, W, A>
{
    type Action = KeybindAction<ControlFlow<A::Action, Action>>;

    type ActionResult = W::ActionResult;

    const NAME: &str = N::NAME;

    fn visit_children(&self, visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        self.inner.visit_children(visitor);
    }

    fn min_width(&self) -> Option<u16> {
        self.inner.min_width()
    }

    fn min_height(&self) -> Option<u16> {
        self.inner.min_height()
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

impl<
    Action: Send + Debug + 'static,
    R: 'static,
    N: Named,
    W: JellyhajWidget<R, Action = KeybindAction<Action>>,
    A: ActionWrapper<R, W>,
> JellyhajWidget<R> for ActionWrapperWidget<Action, N, W, A>
{
    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {
        self.inner.init(cx.wrap_with(Wrapper1::new()));
        self.wrapper
            .init(&mut self.inner, cx.wrap_with(Wrapper2::new()));
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        let inner = match action {
            KeybindAction::Key(k) => KeybindAction::Key(k),
            KeybindAction::Inner(ControlFlow::Continue(a)) => KeybindAction::Inner(a),
            KeybindAction::Inner(ControlFlow::Break(a)) => {
                return self
                    .wrapper
                    .map_action(&mut self.inner, cx.wrap_with(Wrapper2::new()), a);
            }
        };
        self.inner
            .apply_action(cx.wrap_with(Wrapper1::new()), inner)
    }

    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        position: ratatui::prelude::Position,
        size: ratatui::prelude::Size,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        self.inner.click(
            cx.wrap_with(Wrapper1::new()),
            position,
            size,
            kind,
            modifier,
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
