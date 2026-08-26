use std::{convert::Infallible, fmt::Debug, iter::FusedIterator};

use jellyhaj_widgets_core::{WidgetContext, Wrapper};
use tracing::debug;
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

    fn with_selection<W: WithSelection<Self::AR>>(
        &self,
        base_index: usize,
        this: &Self::Selector,
        with: W,
    ) -> bool;
    fn with_selection_mut<W: WithSelectionMut<Self::AR>>(
        &mut self,
        base_index: usize,
        this: &mut Self::Selector,
        with: W,
    );
    fn with_selection_mut_cx<R: 'static, T: Default, W: WithSelectionMutCX<R, Self::AR, T>>(
        &mut self,
        base_index: usize,
        this: &mut Self::Selector,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        with: W,
    ) -> Result<T>;
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
    ) -> Result<Option<T>>;
    fn show_if(&self, index: usize) -> bool;
    fn index(&self, sel: &Self::Selector) -> usize;
    fn total_size(&self) -> usize;
}

#[derive(Debug, Clone, Copy, Valuable, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct ComponentId(usize);

#[derive(Debug, Valuable, Default)]
pub struct VecSelector<Sel: Debug + Valuable + Default> {
    index: ComponentId,
    inner: Sel,
}

#[derive(Debug)]
pub struct VecAction<Action: Debug> {
    index: ComponentId,
    inner: Action,
}

#[derive(Clone, Copy)]
struct VecActionWrapper {
    index: ComponentId,
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

#[derive(Valuable, Debug)]
#[must_use]
pub struct ComponentVec<F: FormComponent> {
    inner: Vec<(ComponentId, F)>,
    id_gen: usize,
}

impl<F: FormComponent> ComponentVec<F> {
    fn find(&self, id: ComponentId) -> usize {
        if let Ok(i) = self.inner.binary_search_by_key(&id, |v| v.0) {
            i
        } else {
            debug!(id = id.0, "element not found");
            0
        }
    }
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &F> + ExactSizeIterator + FusedIterator {
        self.inner.iter().map(|(_, v)| v)
    }
    pub fn iter_mut(
        &mut self,
    ) -> impl DoubleEndedIterator<Item = &mut F> + ExactSizeIterator + FusedIterator {
        self.inner.iter_mut().map(|(_, v)| v)
    }
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    #[must_use]
    pub const fn len(&self) -> usize {
        self.inner.len()
    }
    pub fn retain(&mut self, mut f: impl FnMut(&F) -> bool) {
        self.inner.retain(|v| f(&v.1));
    }
    pub const fn new() -> Self {
        Self {
            inner: Vec::new(),
            id_gen: 0,
        }
    }

    pub fn new_with(i: impl IntoIterator<Item = F>) -> Self {
        let mut id_gen = 0usize;
        let inner = i
            .into_iter()
            .map(|f| {
                let id = id_gen;
                id_gen += 1;
                (ComponentId(id), f)
            })
            .collect();
        Self { inner, id_gen }
    }
    pub fn push(&mut self, new: F) {
        let id = self.id_gen;
        self.id_gen += 1;
        self.inner.push((ComponentId(id), new));
    }
}

impl<F: FormComponent> Default for ComponentVec<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: FormComponent> FromIterator<F> for ComponentVec<F> {
    fn from_iter<T: IntoIterator<Item = F>>(iter: T) -> Self {
        Self::new_with(iter)
    }
}

impl<F: FormComponent> FormComponent for ComponentVec<F>
where
    F::Selector: Default,
{
    type Selector = VecSelector<F::Selector>;

    type AR = F::AR;

    type Action = VecAction<F::Action>;

    fn with_selection<W: WithSelection<Self::AR>>(
        &self,
        mut base_index: usize,
        this: &Self::Selector,
        with: W,
    ) -> bool {
        if self.inner.is_empty() {
            return false;
        }
        let index = self.find(this.index);
        base_index += index_offset(&self.inner, index);
        self.inner[index]
            .1
            .with_selection(base_index, &this.inner, with)
    }

    fn with_selection_mut<W: WithSelectionMut<Self::AR>>(
        &mut self,
        mut base_index: usize,
        this: &mut Self::Selector,
        with: W,
    ) {
        if self.inner.is_empty() {
            return;
        }
        let index = self.find(this.index);
        base_index += index_offset(&self.inner, index);
        self.inner[index]
            .1
            .with_selection_mut(base_index, &mut this.inner, with);
    }

    fn with_selection_mut_cx<R: 'static, T: Default, W: WithSelectionMutCX<R, Self::AR, T>>(
        &mut self,
        mut base_index: usize,
        this: &mut Self::Selector,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        with: W,
    ) -> Result<T> {
        if self.inner.is_empty() {
            return Ok(Default::default());
        }
        let index = self.find(this.index);
        base_index += index_offset(&self.inner, index);
        self.inner[index].1.with_selection_mut_cx(
            base_index,
            &mut this.inner,
            cx.wrap_with(VecActionWrapper { index: this.index }),
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
        for (i, inner) in &mut self.inner {
            let total_size = inner.total_size();
            if rel_index < total_size {
                let mut res = VecSelector {
                    index: *i,
                    inner: Default::default(),
                };
                inner.with_index_mut(
                    base_index,
                    &mut res.inner,
                    cx.wrap_with(VecActionWrapper { index: *i }),
                    index,
                    with,
                )?;
                *this = res;
                return Ok(());
            }
            rel_index -= total_size;
            base_index += total_size;
        }
        Ok(())
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
        self.inner.iter_mut().try_for_each(|(index, inner)| {
            let res = inner.with_iter_mut(
                base_index,
                cx.wrap_with(VecActionWrapper { index: *index }),
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
    ) -> Result<Option<T>> {
        let index = self.find(action.index);
        base_index += index_offset(&self.inner, index);
        self.inner[index].1.with_action_mut(
            base_index,
            action.inner,
            cx.wrap_with(VecActionWrapper {
                index: action.index,
            }),
            with,
        )
    }

    fn show_if(&self, mut index: usize) -> bool {
        for inner in self.iter() {
            let total = inner.total_size();
            if index < total {
                return inner.show_if(index);
            }
            index -= total;
        }
        panic!("index is out of bounds")
    }

    fn index(&self, sel: &Self::Selector) -> usize {
        let index = self.find(sel.index);
        index_offset(&self.inner, index) + self.inner[index].1.index(&sel.inner)
    }

    fn total_size(&self) -> usize {
        self.iter().map(FormComponent::total_size).sum::<usize>()
    }
}

fn index_offset<T, C: FormComponent>(this: &[(T, C)], index: usize) -> usize {
    this[0..index]
        .iter()
        .map(|(_, v)| v)
        .map(FormComponent::total_size)
        .sum::<usize>()
}
