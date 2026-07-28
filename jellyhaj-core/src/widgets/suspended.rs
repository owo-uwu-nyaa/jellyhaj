use std::sync::{Arc, Weak};

use futures_intrusive::sync::ManualResetEvent;
use parking_lot::{Mutex, RwLock};
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
        DropGuard, RunResult, ShadedErased, Visitor, WidgetCreator,
        erased::ErasedWidgetExt,
        list::{ListAccessToken, StateEntry, append_element, remove_element, replace_element},
        state::StateValue,
    },
};

pub struct SuspendedInner {
    task: Mutex<Option<JoinHandle<RunResult>>>,
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
    mut state: ShadedErased,
    stop: Arc<ManualResetEvent>,
    mut visitors: UnboundedReceiver<Visitor>,
    widget_creator: WidgetCreator,
    state_entry: Weak<StateEntry>,
    state_token: Arc<RwLock<ListAccessToken>>,
) -> RunResult {
    loop {
        select! {
            nav = state.next_filtered_event() => {
                match nav.map(Navigation::from) {
                    Some(Navigation::PopContext) => {
                        let mut token = state_token.write();
                        if let Some(entry) = state_entry.upgrade() {
                            unsafe {remove_element(&entry, &mut token)};
                        }
                        return RunResult::Empty;
                    }
                    Some(Navigation::Exit) => {
                        return RunResult::Exit;
                    }
                    Some(Navigation::Replace(next)) => {
                        let mut token = state_token.write();
                        if let Some(entry) = state_entry.upgrade() {
                            unsafe{
                                let new = Arc::new_cyclic(|this|StateEntry::new(StateValue::Suspended(SuspendedInner::new(
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
                        let mut token = state_token.write();
                        if let Some(entry) = state_entry.upgrade() {
                            unsafe{
                                let new = Arc::new_cyclic(|this|StateEntry::new(StateValue::Suspended(SuspendedInner::new(
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
                        let mut token = state_token.write();
                        if let Some(entry) = state_entry.upgrade(){
                            unsafe{
                                let new = Arc::new(StateEntry::new(StateValue::WithoutTui(next)));
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
    pub async fn get_widget(&self) -> RunResult {
        self.drop_guard.inner.set();
        let handle = self.task.lock().take().expect("tried to get task twice");
        handle.await.expect("polling state paniced")
    }

    /**
     * # Safety
     * `state_token` and `this` must belong to the same list
     *  */
    pub unsafe fn new(
        widget: super::ShadedErased,
        this: Weak<StateEntry>,
        widget_creator: WidgetCreator,
        state_token: Arc<RwLock<ListAccessToken>>,
    ) -> Self {
        let stop = Arc::new(ManualResetEvent::new(false));
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
            task: Mutex::new(Some(task)),
            drop_guard: DropGuard { inner: stop },
            name,
            send_visitor: visitor_send,
        }
    }
}
