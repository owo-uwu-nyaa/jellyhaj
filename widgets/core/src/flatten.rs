use std::{fmt::Debug, marker::PhantomData, ops::ControlFlow};

use color_eyre::eyre::Result;

use crate::{
    JellyhajWidgetState,
    mapper::{MapperState, ResultMapper},
    outer::Named,
};

pub struct Mapper<
    R: 'static,
    A: Debug + 'static,
    B: Debug + 'static,
    S: JellyhajWidgetState<R, ActionResult = ControlFlow<A, ControlFlow<A, B>>>,
> {
    _a: PhantomData<fn(A) -> A>,
    _t: PhantomData<fn(B) -> B>,
    _r: PhantomData<fn(R) -> R>,
    _s: PhantomData<fn(S) -> S>,
}

impl<
    R: 'static,
    A: Debug + 'static,
    B: Debug + 'static,
    S: JellyhajWidgetState<R, ActionResult = ControlFlow<A, ControlFlow<A, B>>>,
> Default for Mapper<R, A, B, S>
{
    fn default() -> Self {
        Self {
            _a: Default::default(),
            _t: Default::default(),
            _s: Default::default(),
            _r: PhantomData,
        }
    }
}

impl<
    R: 'static,
    A: Debug + 'static,
    B: Debug + 'static,
    S: JellyhajWidgetState<R, ActionResult = ControlFlow<A, ControlFlow<A, B>>>,
> ResultMapper<R, S> for Mapper<R, A, B, S>
{
    type R = ControlFlow<A, B>;

    fn map_widget(
        _: &mut <S as JellyhajWidgetState<R>>::Widget,
        res: <S as JellyhajWidgetState<R>>::ActionResult,
    ) -> Result<Option<Self::R>> {
        Ok(Some(match res {
            ControlFlow::Continue(c) => c,
            ControlFlow::Break(b) => ControlFlow::Break(b),
        }))
    }

    fn map_state(
        _: &mut S,
        res: <S as JellyhajWidgetState<R>>::ActionResult,
    ) -> Result<Option<Self::R>> {
        Ok(Some(match res {
            ControlFlow::Continue(c) => c,
            ControlFlow::Break(b) => ControlFlow::Break(b),
        }))
    }
}

pub struct Name;
impl Named for Name {
    const NAME: &str = "flatten";
}

pub type FlattenState<R, A, B, S> = MapperState<R, Name, S, Mapper<R, A, B, S>>;
