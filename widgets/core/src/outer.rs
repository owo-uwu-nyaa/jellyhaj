use std::{fmt::Debug, ops::ControlFlow};

pub struct Mapper;

impl<T: Debug + 'static, A: Debug + 'static + Into<T>> ResultMapper<ControlFlow<T, A>> for Mapper {
    type R = T;
    fn map(res: ControlFlow<T, A>) -> color_eyre::eyre::Result<Option<Self::R>> {
        Ok(Some(match res {
            ControlFlow::Continue(c) => c.into(),
            ControlFlow::Break(b) => b,
        }))
    }
}

pub use crate::mapper::Named;
use crate::mapper::{MapperWidget, ResultMapper};

pub type OuterWidget<N, W> = MapperWidget<N, W, Mapper>;

pub struct UnwrapName;
impl Named for UnwrapName {
    const NAME: &str = "unwrap";
}

pub type UnwrapWidget<W> = MapperWidget<UnwrapName, W, Mapper>;
