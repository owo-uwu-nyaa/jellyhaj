#[allow(unused_parens)]
#[automatically_derived]
impl ::jellyhaj_form_widget::form::component::FormComponentBase for Example {
    type Selector = ExampleSelection;
    type AR = crate::ExampleActionResult;
    type Action = ExampleAction;
    fn with_selection<W: ::jellyhaj_form_widget::form::helpers::WithSelection<Self::AR>>(
        &self,
        base_index: ::jellyhaj_form_widget::macro_impl::exports::usize,
        this: &ExampleSelection,
        with: W,
    ) -> ::jellyhaj_form_widget::macro_impl::exports::bool {
        match this {
            ExampleSelection::Simple1(s) => {
                W::with::<
                    bool,
                >(with, s, &self.simple1, "simple 1", (base_index + 0usize))
            }
            ExampleSelection::Simple2(s) => {
                W::with::<
                    &'static str,
                >(with, s, &self.simple2, "simple 2", (base_index + 1usize))
            }
            ExampleSelection::Flatten1(s) => {
                ::jellyhaj_form_widget::form::component::FormComponentBase::with_selection(
                    &self.flatten1,
                    (base_index + 2usize),
                    s,
                    with,
                )
            }
            ExampleSelection::Flatten2(s) => {
                ::jellyhaj_form_widget::form::component::FormComponentBase::with_selection(
                    &self.flatten2,
                    (base_index
                        + ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
                            &self.flatten1,
                        ) + 2usize),
                    s,
                    with,
                )
            }
            ExampleSelection::Simple3(s) => {
                W::with::<
                    Simple,
                >(
                    with,
                    s,
                    &self.simple3,
                    "simple 3",
                    (base_index
                        + ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
                            &self.flatten1,
                        )
                        + ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
                            &self.flatten2,
                        ) + 2usize),
                )
            }
        }
    }
    fn with_selection_mut<
        W: ::jellyhaj_form_widget::form::helpers::WithSelectionMut<Self::AR>,
    >(
        &mut self,
        base_index: ::jellyhaj_form_widget::macro_impl::exports::usize,
        this: &mut ExampleSelection,
        with: W,
    ) {
        match this {
            ExampleSelection::Simple1(s) => {
                W::with_mut::<
                    bool,
                >(with, s, &mut self.simple1, "simple 1", (base_index + 0usize))
            }
            ExampleSelection::Simple2(s) => {
                W::with_mut::<
                    &'static str,
                >(with, s, &mut self.simple2, "simple 2", (base_index + 1usize))
            }
            ExampleSelection::Flatten1(s) => {
                ::jellyhaj_form_widget::form::component::FormComponentBase::with_selection_mut(
                    &mut self.flatten1,
                    (base_index + 2usize),
                    s,
                    with,
                )
            }
            ExampleSelection::Flatten2(s) => {
                ::jellyhaj_form_widget::form::component::FormComponentBase::with_selection_mut(
                    &mut self.flatten2,
                    (base_index
                        + ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
                            &self.flatten1,
                        ) + 2usize),
                    s,
                    with,
                )
            }
            ExampleSelection::Simple3(s) => {
                W::with_mut::<
                    Simple,
                >(
                    with,
                    s,
                    &mut self.simple3,
                    "simple 3",
                    (base_index
                        + ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
                            &self.flatten1,
                        )
                        + ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
                            &self.flatten2,
                        ) + 2usize),
                )
            }
        }
    }
    fn show_if(
        &self,
        mut index: ::jellyhaj_form_widget::macro_impl::exports::usize,
    ) -> ::jellyhaj_form_widget::macro_impl::exports::bool {
        match index {
            1usize => return self._show_if_simple2(),
            0usize => return true,
            _ => {}
        }
        index -= 2usize;
        let cur = ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
            &self.flatten1,
        );
        if index < cur {
            return ::jellyhaj_form_widget::form::component::FormComponentBase::show_if(
                &self.flatten1,
                index,
            );
        }
        index -= cur;
        let cur = ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
            &self.flatten2,
        );
        if index < cur {
            return self._show_if_flatten2()
                && ::jellyhaj_form_widget::form::component::FormComponentBase::show_if(
                    &self.flatten2,
                    index,
                );
        }
        index -= cur;
        match index {
            0usize => return true,
            _ => {}
        };
        ::jellyhaj_form_widget::macro_impl::exports::panic!("index out of bounds")
    }
    fn index(
        &self,
        sel: &ExampleSelection,
    ) -> ::jellyhaj_form_widget::macro_impl::exports::usize {
        match sel {
            ExampleSelection::Simple1(_) => 0usize,
            ExampleSelection::Simple2(_) => 1usize,
            ExampleSelection::Flatten1(sel) => {
                (2usize
                    + ::jellyhaj_form_widget::form::component::FormComponentBase::index(
                        &self.flatten1,
                        sel,
                    ))
            }
            ExampleSelection::Flatten2(sel) => {
                (::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
                    &self.flatten1,
                ) + 2usize
                    + ::jellyhaj_form_widget::form::component::FormComponentBase::index(
                        &self.flatten2,
                        sel,
                    ))
            }
            ExampleSelection::Simple3(_) => {
                ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
                    &self.flatten1,
                )
                    + ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
                        &self.flatten2,
                    ) + 2usize
            }
        }
    }
    fn total_size(&self) -> ::jellyhaj_form_widget::macro_impl::exports::usize {
        ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
            &self.flatten1,
        )
            + ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
                &self.flatten2,
            ) + 3usize
    }
    fn make_selection_default(
        &self,
        mut base_index: ::jellyhaj_form_widget::macro_impl::exports::usize,
        index: ::jellyhaj_form_widget::macro_impl::exports::usize,
    ) -> ExampleSelection {
        match index - base_index {
            0usize => {
                return ExampleSelection::Simple1(
                    ::jellyhaj_form_widget::macro_impl::exports::Default::default(),
                );
            }
            1usize => {
                return ExampleSelection::Simple2(
                    ::jellyhaj_form_widget::macro_impl::exports::Default::default(),
                );
            }
            _ => {}
        }
        base_index += 2usize;
        let cur = ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
            &self.flatten1,
        );
        if index < base_index + cur {
            return ExampleSelection::Flatten1(
                ::jellyhaj_form_widget::form::component::FormComponentBase::make_selection_default(
                    &self.flatten1,
                    base_index,
                    index,
                ),
            )
        } else {
            base_index += cur;
        }
        let cur = ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
            &self.flatten2,
        );
        if index < base_index + cur {
            return ExampleSelection::Flatten2(
                ::jellyhaj_form_widget::form::component::FormComponentBase::make_selection_default(
                    &self.flatten2,
                    base_index,
                    index,
                ),
            )
        } else {
            base_index += cur;
        }
        match index - base_index {
            0usize => {
                return ExampleSelection::Simple3(
                    ::jellyhaj_form_widget::macro_impl::exports::Default::default(),
                );
            }
            _ => {}
        }
        ::jellyhaj_form_widget::macro_impl::exports::panic!("index out of bounds")
    }
}
