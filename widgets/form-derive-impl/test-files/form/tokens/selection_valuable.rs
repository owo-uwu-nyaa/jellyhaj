const _: () = {
    static DEFS: &[::jellyhaj_form_widget::macro_impl::exports::VariantDef] = &[
        ::jellyhaj_form_widget::macro_impl::exports::VariantDef::new(
            "Simple1",
            ::jellyhaj_form_widget::macro_impl::exports::Fields::Unnamed(1),
        ),
        ::jellyhaj_form_widget::macro_impl::exports::VariantDef::new(
            "Simple2",
            ::jellyhaj_form_widget::macro_impl::exports::Fields::Unnamed(1),
        ),
        ::jellyhaj_form_widget::macro_impl::exports::VariantDef::new(
            "Flatten1",
            ::jellyhaj_form_widget::macro_impl::exports::Fields::Unnamed(1),
        ),
        ::jellyhaj_form_widget::macro_impl::exports::VariantDef::new(
            "Flatten2",
            ::jellyhaj_form_widget::macro_impl::exports::Fields::Unnamed(1),
        ),
        ::jellyhaj_form_widget::macro_impl::exports::VariantDef::new(
            "Simple3",
            ::jellyhaj_form_widget::macro_impl::exports::Fields::Unnamed(1),
        ),
    ];
    #[automatically_derived]
    impl ::jellyhaj_form_widget::macro_impl::exports::Valuable for ExampleSelection {
        fn as_value(&self) -> ::jellyhaj_form_widget::macro_impl::exports::Value<'_> {
            ::jellyhaj_form_widget::macro_impl::exports::Value::Enumerable(self)
        }
        fn visit(
            &self,
            visit: &mut dyn ::jellyhaj_form_widget::macro_impl::exports::Visit,
        ) {
            let val = match self {
                ExampleSelection::Simple1(v) => {
                    ::jellyhaj_form_widget::macro_impl::exports::Valuable::as_value(v)
                }
                ExampleSelection::Simple2(v) => {
                    ::jellyhaj_form_widget::macro_impl::exports::Valuable::as_value(v)
                }
                ExampleSelection::Flatten1(v) => {
                    ::jellyhaj_form_widget::macro_impl::exports::Valuable::as_value(v)
                }
                ExampleSelection::Flatten2(v) => {
                    ::jellyhaj_form_widget::macro_impl::exports::Valuable::as_value(v)
                }
                ExampleSelection::Simple3(v) => {
                    ::jellyhaj_form_widget::macro_impl::exports::Valuable::as_value(v)
                }
            };
            visit.visit_unnamed_fields(&[val])
        }
    }
    #[automatically_derived]
    impl ::jellyhaj_form_widget::macro_impl::exports::Enumerable for ExampleSelection {
        fn definition(
            &self,
        ) -> ::jellyhaj_form_widget::macro_impl::exports::EnumDef<'_> {
            ::jellyhaj_form_widget::macro_impl::exports::EnumDef::new_static(
                "ExampleSelection",
                DEFS,
            )
        }
        fn variant(&self) -> ::jellyhaj_form_widget::macro_impl::exports::Variant<'_> {
            match self {
                ExampleSelection::Simple1(_) => {
                    ::jellyhaj_form_widget::macro_impl::exports::Variant::Static(
                        &DEFS[0usize],
                    )
                }
                ExampleSelection::Simple2(_) => {
                    ::jellyhaj_form_widget::macro_impl::exports::Variant::Static(
                        &DEFS[1usize],
                    )
                }
                ExampleSelection::Flatten1(_) => {
                    ::jellyhaj_form_widget::macro_impl::exports::Variant::Static(
                        &DEFS[2usize],
                    )
                }
                ExampleSelection::Flatten2(_) => {
                    ::jellyhaj_form_widget::macro_impl::exports::Variant::Static(
                        &DEFS[3usize],
                    )
                }
                ExampleSelection::Simple3(_) => {
                    ::jellyhaj_form_widget::macro_impl::exports::Variant::Static(
                        &DEFS[4usize],
                    )
                }
            }
        }
    }
};
