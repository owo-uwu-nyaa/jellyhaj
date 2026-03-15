use std::{fmt::Debug, marker::PhantomData, ops::ControlFlow};

pub struct Mapper<
    T: Debug + 'static,
    A: Debug + 'static + Into<T>,
    R: 'static,
    S: JellyhajWidgetState<R, ActionResult = ControlFlow<T, A>>,
> {
    _t: PhantomData<fn(T) -> T>,
    _a: PhantomData<fn(A) -> A>,
    _r: PhantomData<fn(R) -> R>,
    _s: PhantomData<fn(S) -> S>,
}

impl<
    T: Debug + 'static,
    A: Debug + 'static + Into<T>,
    R: 'static,
    S: JellyhajWidgetState<R, ActionResult = ControlFlow<T, A>>,
> Default for Mapper<T, A, R, S>
{
    fn default() -> Self {
        Self {
            _t: PhantomData,
            _a: PhantomData,
            _s: PhantomData,
            _r: PhantomData,
        }
    }
}

impl<
    T: Debug + 'static,
    A: Debug + 'static + Into<T>,
    R: 'static,
    S: JellyhajWidgetState<R, ActionResult = ControlFlow<T, A>>,
> ResultMapper<R, S> for Mapper<T, A, R, S>
{
    type R = T;
    fn map_widget(
        _: &mut S::Widget,
        res: <S::Widget as JellyhajWidget<R>>::ActionResult,
    ) -> color_eyre::eyre::Result<Option<Self::R>> {
        Ok(Some(match res {
            ControlFlow::Continue(c) => c.into(),
            ControlFlow::Break(b) => b,
        }))
    }
    fn map_state(_: &mut S, res: S::ActionResult) -> color_eyre::eyre::Result<Option<Self::R>> {
        Ok(Some(match res {
            ControlFlow::Continue(c) => c.into(),
            ControlFlow::Break(b) => b,
        }))
    }
}

pub use crate::mapper::Named;
use crate::{
    JellyhajWidget, JellyhajWidgetState,
    mapper::{MapperState, ResultMapper},
};

pub type OuterState<N, T, A, R, S> = MapperState<R, N, S, Mapper<T, A, R, S>>;
