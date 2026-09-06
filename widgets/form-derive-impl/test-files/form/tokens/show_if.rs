impl Example {
    #[must_use]
    fn _show_if_simple2(&self) -> ::jellyhaj_form_widget::macro_impl::exports::bool {
        super::test(self.simple1)
    }
    #[must_use]
    fn _show_if_flatten2(&self) -> ::jellyhaj_form_widget::macro_impl::exports::bool {
        self.simple1
    }
}
