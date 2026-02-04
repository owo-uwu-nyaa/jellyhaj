use proc_macro2::{Literal, Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::{Expr, Ident, ItemStruct, LitStr, Path, Result, Type, parse_quote, parse2};

mod height;
mod parse;
mod render1;
mod render2;
mod selection;
mod show_if;
mod widget;

struct FormItem {
    pub name: Ident,
    pub ty: Type,
    pub descr: LitStr,
    pub selection: Path,
    pub selection_id: Ident,
    pub show_if: Option<Expr>,
    pub show_if_fun: Option<Ident>,
}

struct ParseResult {
    fields: Vec<FormItem>,
    name: LitStr,
    action_result: Type,
    state_ty: Ident,
    selection_ty: Ident,
    height_store_ty: Ident,
    widget_ty: Ident,
    full: ItemStruct,
}

pub struct Helpers {
    size_helpers: Path,
    with_current_helpers: Path,
    form_item_tr: Type,
    action_ty: Type,
    quit_form_ty: Type,
    exports: Path,
    up_fn: Ident,
    down_fn: Ident,
    height_fn: Ident,
    item_start_fn: Ident,
    pass1_fn: Ident,
    pass2_fn: Ident,
    with_current_fn: Ident,
    with_current_mut_fn: Ident,
}

fn make_helpers(p: &ParseResult) -> (Helpers, TokenStream) {
    let name = p.state_ty.to_string();
    let items = &p.fields;
    let state_ty = &p.state_ty;
    let selection_ty = &p.selection_ty;
    let action_result_ty = &p.action_result;
    let height_store_ty = &p.height_store_ty;
    let size_helpers: Path = parse_quote!(::jellyhaj_form_widget::macro_impl::size);
    let with_current_helpers: Path = parse_quote!(::jellyhaj_form_widget::macro_impl::with_current);
    let form_item_tr: Type = parse_quote!(::jellyhaj_form_widget::FormItem<#action_result_ty>);
    let exports: Path = parse_quote!(::jellyhaj_form_widget::macro_impl::exports);
    let action_ty = parse_quote!(::jellyhaj_form_widget::FormAction);
    let quit_form_ty = parse_quote!(::jellyhaj_form_widget::QuitForm);
    let detect_loop_name = format_ident!("_widget_helper_{name}_detect_loop");
    let detect_loop_fn =
        selection::detect_loop_fn(items, selection_ty, &detect_loop_name, &exports);
    let up_name = format_ident!("_widget_helper_{name}_up");
    let up_fn = selection::up_fn(
        items,
        state_ty,
        selection_ty,
        &up_name,
        &exports,
        &detect_loop_name,
    );
    let down_name = format_ident!("_widget_helper_{name}_down");
    let down_fn = selection::down_fn(
        items,
        state_ty,
        selection_ty,
        &down_name,
        &exports,
        &detect_loop_name,
    );
    let height_name = format_ident!("_widget_helper_{name}_height");
    let height_fn = height::height_fn(
        items,
        state_ty,
        &height_name,
        &size_helpers,
        &exports,
        &form_item_tr,
        action_result_ty,
        height_store_ty,
    );
    let item_start_name = format_ident!("_widget_helper_{name}_item_start");
    let item_start_fn = height::item_start_fn(
        items,
        state_ty,
        selection_ty,
        &item_start_name,
        &size_helpers,
        &form_item_tr,
        action_result_ty,
    );
    let pass1_name = format_ident!("_widget_helper_{name}_pass1");
    let pass1_fn = render1::pass1_fn(
        items,
        state_ty,
        selection_ty,
        &pass1_name,
        &exports,
        &form_item_tr,
        height_store_ty,
    );
    let pass2_name = format_ident!("_widget_helper_{name}_pass2");
    let pass2_fn = render2::pass2_fn(
        items,
        state_ty,
        selection_ty,
        &pass2_name,
        &size_helpers,
        &exports,
        &form_item_tr,
    );
    let with_current_name = format_ident!("_widget_helper_{name}_with_current");
    let with_current_fn = selection::with_current_fn(
        items,
        state_ty,
        selection_ty,
        &with_current_name,
        &with_current_helpers,
        action_result_ty,
    );
    let with_current_mut_name = format_ident!("_widget_helper_{name}_with_current_mut");
    let with_current_mut_fn = selection::with_current_mut_fn(
        items,
        state_ty,
        selection_ty,
        &with_current_mut_name,
        &with_current_helpers,
        action_result_ty,
    );

    let helpers = Helpers {
        size_helpers,
        with_current_helpers,
        form_item_tr,
        exports,
        quit_form_ty,
        height_fn: height_name,
        item_start_fn: item_start_name,
        pass1_fn: pass1_name,
        pass2_fn: pass2_name,
        with_current_fn: with_current_name,
        with_current_mut_fn: with_current_mut_name,
        action_ty,
        up_fn: up_name,
        down_fn: down_name,
    };
    let helper_impl = quote! {
        #detect_loop_fn
        #up_fn
        #down_fn
        #height_fn
        #item_start_fn
        #pass1_fn
        #pass2_fn
        #with_current_fn
        #with_current_mut_fn
    };
    (helpers, helper_impl)
}

pub fn form(args: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let parsed = parse::parse(args, input)?;
    let (helpers, helper_code) = make_helpers(&parsed);
    let show_ifs = show_if::gen_show_ifs(&parsed.fields, &parsed.state_ty);
    let make_selection = selection::make_selection(
        &parsed.fields,
        &parsed.selection_ty,
        &helpers.form_item_tr,
        &helpers.exports,
    );
    let height_store =
        height::make_height_store(&parsed.fields, &parsed.height_store_ty, &helpers.exports);
    let make_widget = widget::make_widget(
        &parsed.state_ty,
        &parsed.widget_ty,
        &parsed.selection_ty,
        &parsed.action_result,
        &parsed.height_store_ty,
        &helpers.quit_form_ty,
        &helpers.exports,
        &helpers.action_ty,
        &helpers.with_current_helpers,
        &helpers.with_current_fn,
        &helpers.with_current_mut_fn,
        &helpers.up_fn,
        &helpers.down_fn,
    );
    let full = &parsed.full;
    Ok(quote! {
        #full
        #show_ifs
        #make_selection
        #height_store
        #helper_code
        #make_widget
    })
}

fn render_body(
    items: &[FormItem],
    descr: &LitStr,
    self_ty: &Ident,
    selection_type: &Ident,
) -> TokenStream {
    quote! {
        let outer = ::jellyhaj_form_widget::macro_impl::outer_block(#descr);
        let main = outer.inner(area);
        if main.height < height{
            let mut scroll_view = ::jellyhaj_form_widget::macro_impl::ScrollView::new(
                jellyhaj_form_widget::macro_impl::Size{width: main.width, height}
            );
            render_inner(self, scroll_view.area(), scroll_view.buf_mut());
            let offset = ::jellyhaj_form_widget::macro_impl::offset::calc_offset(
                height, main.height, item_start(self.selection)
            );
            let mut state = ::jellyhaj_form_widget::macro_impl::ScrollViewState::with_offset(
                ::jellyhaj_form_widget::macro_impl::Position{x:0, y: }
            );
            jellyhaj_form_widget::macro_impl::StatefulWidget::render(scroll_view, main, buf, &mut state);
        }else{
            render_inner(self, main, buf);
        }

    }
}
