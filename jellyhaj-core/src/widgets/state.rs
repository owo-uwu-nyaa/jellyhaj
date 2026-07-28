use std::{cell::UnsafeCell, sync::Arc};

use futures_util::future::BoxFuture;
use jellyhaj_widgets_core::Result;
use parking_lot::RwLock;
use tracing::instrument;

use crate::widgets::{
    ShadedErased, WidgetCreator,
    list::{ListAccessToken, ListEntry, StateEntry, inspect_list, prepend_element, remove_element},
    suspended::SuspendedInner,
};

pub enum StateValue {
    Suspended(SuspendedInner),
    Empty,
    WithoutTui(BoxFuture<'static, Result<()>>),
}

pub struct StateStack {
    lock: Arc<RwLock<ListAccessToken>>,
    list: Arc<StateEntry>,
}

impl Drop for StateStack {
    // break reference cycles, isolate all entries
    #[instrument(skip_all, level = "trace", name = "StateStack::drop()")]
    fn drop(&mut self) {
        let mut guard = self.lock.write();
        let mut entry = self.list.clone();
        unsafe { inspect_list(&entry, &guard) };
        while let Some(new_entry) = {
            let entry = unsafe { entry.get_list_mut(&mut guard) };
            entry.prev = None;
            entry.next.take()
        } {
            entry = new_entry;
        }
    }
}

impl StateStack {
    #[instrument(skip_all, level = "trace", name = "StateStack::new()")]
    pub fn new() -> Self {
        tracing::trace!("new state stack");
        let list = Arc::new(StateEntry {
            list: UnsafeCell::new(ListEntry {
                next: None,
                prev: None,
            }),
            value: StateValue::Empty,
        });
        //initialize, this is the ownly copy so this is save
        unsafe {
            let entry = &mut *list.list.get();
            entry.next = Some(list.clone());
            entry.prev = Some(Arc::downgrade(&list));
        }

        let token = ListAccessToken { _evil: () };
        unsafe { inspect_list(&list, &token) };
        Self {
            lock: Arc::new(RwLock::new(token)),
            list,
        }
    }
    pub fn push(&self, widget: ShadedErased, widget_creator: WidgetCreator) {
        let mut token = self.lock.write();
        unsafe {
            prepend_element(
                &self.list,
                Arc::new_cyclic(|this| {
                    StateEntry::new(StateValue::Suspended(SuspendedInner::new(
                        widget,
                        this.clone(),
                        widget_creator,
                        self.lock.clone(),
                    )))
                }),
                &mut token,
            );
        }
    }
    #[must_use]
    pub fn pop(&self) -> StateValue {
        let mut token = self.lock.write();
        let entry = unsafe { self.list.get_list(&token) }
            .prev
            .as_ref()
            .expect("previous should be set while the list is live")
            .upgrade()
            .expect("previous should not be dropped");
        if Arc::ptr_eq(&self.list, &entry) {
            StateValue::Empty
        } else {
            unsafe {
                remove_element(&entry, &mut token);
            }
            unsafe { inspect_list(&self.list, &token) };
            Arc::into_inner(entry)
                .expect("should not currently be owned")
                .value
        }
    }

    #[instrument(skip_all, level = "trace")]
    pub fn visit(&self, mut visitor: impl FnMut(&StateValue)) {
        let token = self.lock.read();
        tracing::trace!("visiting states");
        unsafe { inspect_list(&self.list, &token) };
        let head = &self.list;
        let mut current = head;
        loop {
            current = unsafe { current.get_list(&token) }
                .next
                .as_ref()
                .expect("next should be set while the list is live");
            if Arc::ptr_eq(current, head) {
                break;
            }
            visitor(&current.value);
        }
    }
}

impl Default for StateStack {
    fn default() -> Self {
        Self::new()
    }
}
