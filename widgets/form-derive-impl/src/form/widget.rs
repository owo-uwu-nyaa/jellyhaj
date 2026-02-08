use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, LitStr, Path, Type};

pub fn make_widget(
    state_ty: &Ident,
    widget_ty: &Ident,
    selection_ty: &Ident,
    action_result: &Type,
    height_store_ty: &Type,
    quit_form_ty: &Type,
    exports: &Path,
    action_ty: &Type,
    with_current_helpers: &Path,
    with_current_fn: &Ident,
    with_current_mut_fn: &Ident,
    height_fn: &Ident,
    item_start_fn: &Ident,
    up_fn: &Ident,
    down_fn: &Ident,
    pass1_fn: &Ident,
    pass2_fn: &Ident,
    assert_current_shown_fn: &Ident,
    click_fn: &Ident,
    calc_offset_fn: &Path,
    descr: &LitStr,
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

        impl<'s> #widget_ty<'s>{
            pub fn new<'n>(
                state: &'n mut #state_ty,
            ) -> #widget_ty<'n>{
                #widget_ty{
                    state, selection: #exports::Default::default(), store: #exports::Default::default(), offset: 0
                }
            }
            pub fn with_selection<'n>(
                state: &'n mut #state_ty,
                selection: #selection_ty,
            ) -> #widget_ty<'n>{
                #widget_ty{
                    state, selection, store: #exports::Default::default(), offset: 0
                }
            }
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
                    let res = #with_current_mut_fn(state,sel, #with_current_helpers::ApplyAction(action))?;
                    #assert_current_shown_fn(state, *sel);
                    #exports::Result::Ok(res)
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
                mut position: #exports::Position,
                mut size: #exports::Size,
                kind: #exports::MouseEventKind,
                modifier: #exports::KeyModifiers,
            ) -> #exports::Result<#exports::Option<Self::ActionResult>>{
                if position.x > 2 && position.y > 2 &&
                    position.x < size.width-1 &&
                    position.y < size.height-1
                {
                    position.x-=2;
                    position.y-=2;
                    size.width-=4;
                    size.height-=4;
                    #click_fn(
                        &mut self.state,
                        &mut self.selection,
                        &self.store,
                        position,
                        size,
                        kind,
                        modifier,
                        self.offset
                    )
                }else{
                    #exports::Result::Ok(#exports::None)
                }
            }

            fn render_fallible_inner(
                &mut self,
                area: #exports::Rect,
                buf: &mut #exports::Buffer,
                task: #exports::TaskSubmitter<Self::Action, impl #exports::Wrapper<Self::Action>>,
            ) -> #exports::Result<()>{
                let outer = #exports::Block::bordered().title(#descr).padding(#exports::Padding::uniform(1));
                let main = outer.inner(area);
                let height = #height_fn(&self.state, &mut self.store);
                if main.height < height {
                    let mut scroll_view = #exports::ScrollView::new(
                        #exports::Size{width: main.width, height}
                    );
                    let area = scroll_view.area();
                    self.offset = #calc_offset_fn(
                        height, main.height, #item_start_fn(&self.store, self.selection)
                    );
                    #pass1_fn(
                        &mut self.state,
                        self.selection,
                        scroll_view.buf_mut(),
                        area,
                        &self.store
                    )?;
                    let mut state = #exports::ScrollViewState::with_offset(
                        #exports::Position{x:0, y: self.offset}
                    );
                    #exports::StatefulWidget::render(scroll_view, main, buf, &mut state);
                }else{
                    self.offset = 0;
                    #pass1_fn(
                        &mut self.state,
                        self.selection,
                        buf,
                        main,
                        &self.store,

                    )?;
                }
                #pass2_fn(
                    &mut self.state,
                    self.selection,
                    &self.store,
                    buf,
                    main,
                    self.offset
                )?;
                #exports::Widget::render(outer, area, buf);
                #exports::Result::Ok(())
            }
        }
    }
}
