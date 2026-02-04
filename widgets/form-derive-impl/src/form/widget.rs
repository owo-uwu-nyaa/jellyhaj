use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Path, Type};

pub fn make_widget(
    state_ty: &Ident,
    widget_ty: &Ident,
    selection_ty: &Ident,
    action_result: &Type,
    height_store_ty: &Ident,
    quit_form_ty: &Type,
    exports: &Path,
    action_ty: &Type,
    with_current_helpers: &Path,
    with_current_fn: &Ident,
    with_current_mut_fn: &Ident,
    up_fn: &Ident,
    down_fn: &Ident,
) -> TokenStream {
    quote! {
        const _: () = {
            fn assert_into_quit<T: #exports::From<#quit_form_ty>>() {}
            fn assert_t(){
                assert_into_quit::<#action_result>()
            }
        };

        pub struct #widget_ty<'s>{
            state: &'s mut #state_ty,
            selection: #selection_ty,
            store: #height_store_ty,
            offset: u16,
        }

        impl #exports::JellyhajWidget for #widget_ty<'_>{
            type State = #selection_ty;
            type Action = #action_ty;
            type ActionResult = #action_result;

            fn min_width(&self) -> #exports::Option<u16>{
                Some(10)
            }
            fn min_height(&self) -> #exports::Option<u16>{
                Some(10)
            }

            fn into_state(self) -> Self::State{
                self.selection
            }

            fn accepts_text_input(&self) -> bool{
                #with_current_fn(&self.state, self.selection, #with_current_helpers::AcceptsTextInput)
            }

            fn accept_char(&mut self, text: char){
                #with_current_mut_fn(&mut self.state,&mut self.selection, #with_current_helpers::ApplyChar(text))
            }
            fn accept_text(&mut self, text: #exports::String){
                #with_current_mut_fn(&mut self.state,&mut self.selection, #with_current_helpers::ApplyText(text))
            }

            fn apply_action(
                &mut self,
                task: #exports::TaskSubmitter<Self::Action, impl #exports::Wrapper<Self::Action>>,
                action: Self::Action,
            ) -> #exports::Result<#exports::Option<Self::ActionResult>>{
                fn inner(
                    state: &mut #state_ty, sel: &mut #selection_ty, action: #action_ty
                )-> #exports::Result<#exports::Option<#action_result>>{
                    #with_current_mut_fn(state,sel, #with_current_helpers::ApplyAction(action))
                }
                if #with_current_fn(&self.state, self.selection, #with_current_helpers::AcceptsMovementAction){
                    inner(&mut self.state, &mut self.selection, action)
                }else{
                    match action{
                        #action_ty::Up => {
                            self.selection = #up_fn(&self.state, self.selection);
                            #exports::Result::Ok(#exports::None)
                        }
                        #action_ty::Down => {
                            self.selection = #down_fn(&self.state, self.selection);
                            #exports::Result::Ok(#exports::None)
                        }
                        #action_ty::Delete => {
                            inner(&mut self.state, &mut self.selection, action)
                        }
                        #action_ty::Enter => {
                            inner(&mut self.state, &mut self.selection, action)
                        }
                        #action_ty::Quit => {
                            #exports::Result::Ok(#exports::Some(#exports::From::from(#quit_form_ty)))
                        }
                        #action_ty::Left | #action_ty::Right => {
                            #exports::Result::Ok(#exports::None)
                        }
                    }
                }
            }

            fn click(
                &mut self,
                task: #exports::TaskSubmitter<Self::Action, impl #exports::Wrapper<Self::Action>>,
                position: #exports::Position,
                size: #exports::Size,
                kind: #exports::MouseEventKind,
                modifier: #exports::KeyModifiers,
            ) -> #exports::Result<#exports::Option<Self::ActionResult>>{#exports::Result::Ok(#exports::None)}

            fn render_fallible_inner(
                &mut self,
                area: #exports::Rect,
                buf: &mut #exports::Buffer,
                task: #exports::TaskSubmitter<Self::Action, impl #exports::Wrapper<Self::Action>>,
            ) -> #exports::Result<()>{#exports::Result::Ok(())}
        }
    }
}
