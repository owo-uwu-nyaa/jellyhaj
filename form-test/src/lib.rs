use jellyhaj_form_widget::{QuitForm, Selection, form};

#[form("test", QuitForm)]
struct Test {
    #[descr("v1")]
    v1: bool,
    #[descr("v2")]
    #[show_if(self.v1)]
    v2: bool,
}
