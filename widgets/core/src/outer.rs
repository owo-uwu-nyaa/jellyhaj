use std::{fmt::Debug, marker::PhantomData, ops::ControlFlow};

pub struct Mapper<
    T: Debug + 'static,
    A: Debug + 'static + Into<T>,
    S: JellyhajWidgetState<ActionResult = ControlFlow<T, A>>,
> {
    _t: PhantomData<fn(T) -> T>,
    _a: PhantomData<fn(A) -> A>,
    _s: PhantomData<fn(S) -> S>,
}

impl<
    T: Debug + 'static,
    A: Debug + 'static + Into<T>,
    S: JellyhajWidgetState<ActionResult = ControlFlow<T, A>>,
> Default for Mapper<T, A, S>
{
    fn default() -> Self {
        Self {
            _t: Default::default(),
            _a: Default::default(),
            _s: Default::default(),
        }
    }
}

impl<
    T: Debug + 'static,
    A: Debug + 'static + Into<T>,
    S: JellyhajWidgetState<ActionResult = ControlFlow<T, A>>,
> ResultMapper<S> for Mapper<T, A, S>
{
    type R = T;
    fn map_widget(
        _: &mut S::Widget,
        res: <S::Widget as JellyhajWidget>::ActionResult,
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

pub type OuterState<N, T, A, S> = MapperState<N, S, Mapper<T, A, S>>;
