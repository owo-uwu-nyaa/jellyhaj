use std::{convert::Infallible, fmt::Debug};

use jellyhaj_widgets_core::{WidgetContext, Wrapper};
use valuable::Valuable;

use crate::form::helpers::{
    WithActionMut, WithIndexMut, WithIterItems, WithIterItemsMut, WithSelection, WithSelectionMut,
    WithSelectionMutCX,
};

use color_eyre::Result;

pub trait FormComponent: Sized + Send + Unpin + Valuable + 'static {
    type Selector: Debug + Send + Valuable + Default;
    type AR: Debug + From<Infallible>;
    type Action: Debug + Send + 'static;

    fn with_selection<T, W: WithSelection<Self::AR, T>>(
        &self,
        base_index: usize,
        this: &Self::Selector,
        with: W,
    ) -> T;
    fn with_selection_mut<T, W: WithSelectionMut<Self::AR, T>>(
        &mut self,
        base_index: usize,
        this: &mut Self::Selector,
        with: W,
    ) -> T;
    fn with_selection_mut_cx<R: 'static, T, W: WithSelectionMutCX<R, Self::AR, T>>(
        &mut self,
        base_index: usize,
        this: &mut Self::Selector,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        with: W,
    ) -> T;
    fn with_index_mut<R: 'static, W: WithIndexMut<R, Self::AR>>(
        &mut self,
        base_index: usize,
        this: &mut Self::Selector,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        index: usize,
        with: W,
    ) -> Result<()>;
    fn with_iter<R: 'static, W: WithIterItems<R, Self::AR>>(
        &self,
        base_index: usize,
        with: &mut W,
    ) -> Result<()>;
    fn with_iter_mut<R: 'static, W: WithIterItemsMut<R, Self::AR>>(
        &mut self,
        base_index: usize,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        with: &mut W,
        show: bool,
    ) -> Result<()>;
    fn with_action_mut<R: 'static, T, W: WithActionMut<R, Self::AR, T>>(
        &mut self,
        base_index: usize,
        action: Self::Action,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        with: W,
    ) -> T;
    fn show_if(&self, index: usize) -> bool;
    fn index(&self, sel: &Self::Selector) -> usize;
    fn total_size(&self) -> usize;
}

#[derive(Debug, Valuable, Default)]
pub struct VecSelector<Sel: Debug + Valuable + Default> {
    index: usize,
    inner: Sel,
}

#[derive(Debug)]
pub struct VecAction<Action: Debug> {
    index: usize,
    inner: Action,
}

#[derive(Clone, Copy)]
struct VecActionWrapper {
    index: usize,
}
impl<V: Debug + Send + 'static> Wrapper<V> for VecActionWrapper {
    type F = VecAction<V>;

    fn wrap(&self, val: V) -> Self::F {
        VecAction {
            index: self.index,
            inner: val,
        }
    }
}

impl<F: FormComponent> FormComponent for Vec<F>
where
    F::Selector: Default,
{
    type Selector = VecSelector<F::Selector>;

    type AR = F::AR;

    type Action = VecAction<F::Action>;

    fn with_selection<T, W: WithSelection<Self::AR, T>>(
        &self,
        mut base_index: usize,
        this: &Self::Selector,
        with: W,
    ) -> T {
        let index = this.index;
        base_index += self[0..index]
            .iter()
            .map(FormComponent::total_size)
            .sum::<usize>();
        self[base_index].with_selection(base_index, &this.inner, with)
    }

    fn with_selection_mut<T, W: WithSelectionMut<Self::AR, T>>(
        &mut self,
        mut base_index: usize,
        this: &mut Self::Selector,
        with: W,
    ) -> T {
        let index = this.index;
        base_index += self[0..index]
            .iter()
            .map(FormComponent::total_size)
            .sum::<usize>();
        self[base_index].with_selection_mut(base_index, &mut this.inner, with)
    }

    fn with_selection_mut_cx<R: 'static, T, W: WithSelectionMutCX<R, Self::AR, T>>(
        &mut self,
        mut base_index: usize,
        this: &mut Self::Selector,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        with: W,
    ) -> T {
        let index = this.index;
        base_index += index_offset(self, index);
        self[base_index].with_selection_mut_cx(
            base_index,
            &mut this.inner,
            cx.wrap_with(VecActionWrapper { index }),
            with,
        )
    }

    fn with_index_mut<R: 'static, W: WithIndexMut<R, Self::AR>>(
        &mut self,
        mut base_index: usize,
        this: &mut Self::Selector,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        index: usize,
        with: W,
    ) -> Result<()> {
        let mut rel_index = index.strict_sub(base_index);
        for (i, inner) in self.iter_mut().enumerate() {
            let total_size = inner.total_size();
            if rel_index < total_size {
                let mut res = VecSelector {
                    index,
                    inner: Default::default(),
                };
                inner.with_index_mut(
                    base_index,
                    &mut res.inner,
                    cx.wrap_with(VecActionWrapper { index: i }),
                    index,
                    with,
                )?;
                *this = res;
                return Ok(());
            }
            rel_index -= total_size;
            base_index += total_size;
        }
        panic!("index does not exist")
    }

    fn with_iter<R: 'static, W: WithIterItems<R, Self::AR>>(
        &self,
        mut base_index: usize,
        with: &mut W,
    ) -> Result<()> {
        self.iter().try_for_each(|inner| {
            let res = inner.with_iter(base_index, with);
            base_index += inner.total_size();
            res
        })?;
        Ok(())
    }

    fn with_iter_mut<R: 'static, W: WithIterItemsMut<R, Self::AR>>(
        &mut self,
        mut base_index: usize,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        with: &mut W,
        show: bool,
    ) -> Result<()> {
        self.iter_mut().enumerate().try_for_each(|(index, inner)| {
            let res = inner.with_iter_mut(
                base_index,
                cx.wrap_with(VecActionWrapper { index }),
                with,
                show,
            );
            base_index += inner.total_size();
            res
        })
    }

    fn with_action_mut<R: 'static, T, W: WithActionMut<R, Self::AR, T>>(
        &mut self,
        mut base_index: usize,
        action: Self::Action,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        with: W,
    ) -> T {
        base_index += index_offset(self, action.index);
        self[action.index].with_action_mut(
            base_index,
            action.inner,
            cx.wrap_with(VecActionWrapper {
                index: action.index,
            }),
            with,
        )
    }

    fn show_if(&self, mut index: usize) -> bool {
        for inner in self {
            let total = inner.total_size();
            if index < total {
                return inner.show_if(index);
            }
            index -= total;
        }
        panic!("index is out of bounds")
    }

    fn index(&self, sel: &Self::Selector) -> usize {
        index_offset(self, sel.index) + self[sel.index].index(&sel.inner)
    }

    fn total_size(&self) -> usize {
        self.iter().map(FormComponent::total_size).sum::<usize>()
    }
}

fn index_offset<C: FormComponent>(this: &[C], index: usize) -> usize {
    this[0..index]
        .iter()
        .map(FormComponent::total_size)
        .sum::<usize>()
}
