use std::{fmt::Debug, ops::ControlFlow};

use color_eyre::eyre::Result;

use crate::{
    mapper::{MapperWidget, ResultMapper},
    outer::Named,
};

pub struct Mapper;

impl<A: Debug + 'static, B: Debug + 'static> ResultMapper<ControlFlow<A, ControlFlow<A, B>>>
    for Mapper
{
    type R = ControlFlow<A, B>;

    fn map(res: ControlFlow<A, ControlFlow<A, B>>) -> Result<Option<Self::R>> {
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

pub type FlattenWidget<W> = MapperWidget<Name, W, Mapper>;
