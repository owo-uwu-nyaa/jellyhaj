use jellyhaj_form_widget::{QuitForm, Selection, form};

#[derive(Debug, Clone, Copy, Selection, PartialEq, Eq)]
enum Vals {
    #[descr("v1")]
    V1,
    #[descr("v2")]
    V2,
}

#[form("test", QuitForm)]
struct Test {
    #[descr("v1")]
    v1: bool,
    #[descr("v2")]
    #[show_if(self.v1)]
    v2: bool,
    #[descr("v3")]
    v3: bool,
    #[descr("super important")]
    #[show_if(self.v1 && self.v2 && self.v3 && self.show)]
    v4: bool,
    #[descr("values")]
    v5: Vals,
    #[skip]
    show: bool,
}
