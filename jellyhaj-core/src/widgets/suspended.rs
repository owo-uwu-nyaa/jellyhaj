use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use futures_intrusive::sync::ManualResetEvent;
use spawn::spawn_future;
use tokio::{
    select,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tracing::instrument;

use crate::{
    state::Navigation,
    widgets::{
        DropGuard, RunResult, Visitor, WidgetCreator,
        erased::ErasedWidgetExt,
        list::{ListAccessToken, StateEntry, append_element, remove_element, replace_element},
        shaded::widget::Erased,
        state::StateValue,
    },
};

pub struct SuspendedInner {
    task: Cell<Option<JoinHandle<RunResult>>>,
    drop_guard: DropGuard,
    pub name: &'static str,
    pub send_visitor: UnboundedSender<Visitor>,
}

/**
 * # Safety
 * `state_token` and `state_entry` must belong to the same list
 *  */
#[instrument(skip_all)]
async unsafe fn run_suspended(
    mut state: Erased,
    stop: Rc<ManualResetEvent>,
    mut visitors: UnboundedReceiver<Visitor>,
    widget_creator: WidgetCreator,
    state_entry: Weak<StateEntry>,
    state_token: Rc<RefCell<ListAccessToken>>,
) -> RunResult {
    loop {
        select! {
            nav = state.next_filtered_event() => {
                match nav.map(Navigation::from) {
                    Some(Navigation::PopContext) => {
                        let mut token = state_token.borrow_mut();
                        if let Some(entry) = state_entry.upgrade() {
                            unsafe {remove_element(&entry, &mut token)};
                        }
                        return RunResult::Empty;
                    }
                    Some(Navigation::Exit) => {
                        return RunResult::Exit;
                    }
                    Some(Navigation::Replace(next)) => {
                        let mut token = state_token.borrow_mut();
                        if let Some(entry) = state_entry.upgrade() {
                            unsafe{
                                let new = Rc::new_cyclic(|this|StateEntry::new(StateValue::Suspended(SuspendedInner::new(
                                        widget_creator(next),
                                        this.clone(),
                                        widget_creator.clone(),
                                        state_token.clone()
                                    ))));
                                replace_element(
                                    &entry,
                                    &new,
                                    &mut token,
                                );
                            }
                        }
                        return RunResult::Empty;
                    }
                    Some(Navigation::Push(next)) => {
                        let mut token = state_token.borrow_mut();
                        if let Some(entry) = state_entry.upgrade() {
                            unsafe{
                                let new = Rc::new_cyclic(|this|StateEntry::new(StateValue::Suspended(SuspendedInner::new(
                                        widget_creator(next),
                                        this.clone(),
                                        widget_creator.clone(),
                                        state_token.clone()
                                    ))));
                                append_element(
                                    &entry,
                                    new.clone(),
                                    &mut token,
                                );
                            }
                        }
                    }
                    Some(Navigation::PushWithoutTui(next)) => {
                        let mut token = state_token.borrow_mut();
                        if let Some(entry) = state_entry.upgrade(){
                            unsafe{
                                let new = Rc::new(StateEntry::new(StateValue::WithoutTui(next)));
                                append_element(&entry,new.clone() , &mut token);
                            }
                        }
                    }
                    None => return RunResult::Exit
                }
            }
            () = stop.wait() => {
                return RunResult::Cont(state)
            }
            visitor = visitors.recv() => {
                if let Some(visitor) = visitor{
                    visitor(&|visitor|state.visit(visitor));
                }else {
                    return RunResult::Cont(state)
                }
            }
        };
    }
}

impl SuspendedInner {
    pub fn get_widget(&self) -> JoinHandle<RunResult> {
        self.drop_guard.inner.set();
        self.task.take().expect("tried to get task twice")
    }

    /**
     * # Safety
     * `state_token` and `this` must belong to the same list
     *  */
    pub unsafe fn new(
        widget: Erased,
        this: Weak<StateEntry>,
        widget_creator: WidgetCreator,
        state_token: Rc<RefCell<ListAccessToken>>,
    ) -> Self {
        let stop = Rc::new(ManualResetEvent::new(false));
        let (visitor_send, visitor_recv) = tokio::sync::mpsc::unbounded_channel();
        let name = widget.name();
        let task = spawn_future(
            unsafe {
                run_suspended(
                    widget,
                    stop.clone(),
                    visitor_recv,
                    widget_creator,
                    this,
                    state_token,
                )
            },
            name,
        );
        Self {
            task: Cell::new(Some(task)),
            drop_guard: DropGuard { inner: stop },
            name,
            send_visitor: visitor_send,
        }
    }
}
