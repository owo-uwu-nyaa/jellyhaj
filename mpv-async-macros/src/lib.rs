mod node_list;
mod node_map;

use node_list::{NodeListArgs, node_list_impl};
use node_map::{NodeMapArgs, node_map_impl};
use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro]
pub fn mpv_node_map_internal(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as NodeMapArgs);
    node_map_impl(input).into()
}

#[proc_macro]
pub fn mpv_node_list_internal(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as NodeListArgs);
    node_list_impl(input).into()
}
