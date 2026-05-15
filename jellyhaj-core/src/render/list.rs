use std::{
    cell::UnsafeCell,
    ptr::null,
    sync::{Arc, Weak},
};

use tracing::instrument;

use crate::render::StateValue;

#[instrument(skip_all, level = "trace")]
pub unsafe fn remove_element(entry: &StateEntry, token: &mut ListAccessToken) {
    tracing::trace!("removing elelment");
    if let (Some(next), Some(prev)) = {
        let entry = unsafe { entry.get_list_mut(token) };
        (
            entry.next.take(),
            entry.prev.take().as_ref().and_then(Weak::upgrade),
        )
    } {
        unsafe { next.get_list_mut(token) }.prev = Some(Arc::downgrade(&prev));
        unsafe { prev.get_list_mut(token) }.next = Some(next);
        unsafe { inspect_list(&prev, token) };
    }
}

#[instrument(skip_all, level = "trace")]
pub unsafe fn replace_element(
    entry: &Arc<StateEntry>,
    new_entry: &Arc<StateEntry>,
    token: &mut ListAccessToken,
) {
    tracing::trace!("replacing element");
    if let (Some(next), Some(prev)) = {
        let entry = unsafe { entry.get_list_mut(token) };
        (
            entry.next.take(),
            entry.prev.take().as_ref().and_then(Weak::upgrade),
        )
    } {
        unsafe { next.get_list_mut(token) }.prev = Some(Arc::downgrade(new_entry));
        let new = unsafe { new_entry.get_list_mut(token) };
        new.next = Some(next);
        new.prev = Some(Arc::downgrade(&prev));
        unsafe { prev.get_list_mut(token) }.next = Some(new_entry.clone());
    }
    unsafe { inspect_list(new_entry, token) };
}

#[instrument(skip_all, level = "trace")]
pub unsafe fn append_element(
    entry: &Arc<StateEntry>,
    new_entry: Arc<StateEntry>,
    token: &mut ListAccessToken,
) {
    tracing::trace!("appending element");
    if let Some(next) = { unsafe { entry.get_list_mut(token) }.next.take() } {
        unsafe { next.get_list_mut(token) }.prev = Some(Arc::downgrade(&new_entry));
        let new = unsafe { new_entry.get_list_mut(token) };
        new.next = Some(next);
        new.prev = Some(Arc::downgrade(entry));
        unsafe { entry.get_list_mut(token) }.next = Some(new_entry);
    }
    unsafe { inspect_list(entry, token) };
}

#[instrument(skip_all, level = "trace")]
pub unsafe fn prepend_element(
    entry: &Arc<StateEntry>,
    new_entry: Arc<StateEntry>,
    token: &mut ListAccessToken,
) {
    unsafe { inspect_list(entry, token) };
    tracing::trace!("prepending element");
    if let Some(prev) = {
        unsafe { entry.get_list_mut(token) }
            .prev
            .take()
            .and_then(|p| p.upgrade())
    } {
        unsafe { entry.get_list_mut(token) }.prev = Some(Arc::downgrade(&new_entry));
        let new = unsafe { new_entry.get_list_mut(token) };
        new.next = Some(entry.clone());
        new.prev = Some(Arc::downgrade(&prev));
        unsafe { prev.get_list_mut(token) }.next = Some(new_entry);
    }
    unsafe { inspect_list(entry, token) };
}

pub unsafe fn inspect_list(start: &Arc<StateEntry>, token: &ListAccessToken) {
    let span = tracing::trace_span!("inspect_list");
    let _entered = span.enter();
    let mut next_entry = Some(start);
    if !span.is_disabled() {
        while let Some(cur) = next_entry {
            let kind = match &cur.value {
                StateValue::Suspended(_) => "Suspended",
                StateValue::Empty => "Empty",
                StateValue::WithoutTui(_) => "WithoutTui",
            };
            let entry = unsafe { cur.get_list(token) };
            let prev = entry.prev.as_ref().map_or(null(), Weak::as_ptr);
            let next = entry.next.as_ref().map_or(null(), Arc::as_ptr);
            tracing::trace!(name: "list-entry", kind = kind, prev = ?prev, next = ?next);
            next_entry = entry.next.as_ref();
            if let Some(e) = next_entry
                && Arc::ptr_eq(e, start)
            {
                break;
            }
        }
    }
}

pub struct ListAccessToken {
    pub(super) _evil: (),
}

pub struct ListEntry {
    pub next: Option<Arc<StateEntry>>,
    pub prev: Option<Weak<StateEntry>>,
}

pub struct StateEntry {
    pub list: UnsafeCell<ListEntry>,
    pub value: StateValue,
}

unsafe impl Sync for StateEntry {}
unsafe impl Send for StateEntry {}

impl StateEntry {
    /// # Safety
    /// the `ListAccessToken` must be from this list
    pub unsafe fn get_list<'a>(&'a self, _token: &'a ListAccessToken) -> &'a ListEntry {
        unsafe { &*self.list.get() }
    }
    /// # Safety
    /// the `ListAccessToken` must be from this list
    pub unsafe fn get_list_mut<'a>(&'a self, _token: &'a mut ListAccessToken) -> &'a mut ListEntry {
        unsafe { &mut *self.list.get() }
    }

    pub const fn new(val: StateValue) -> Self {
        Self {
            list: UnsafeCell::new(ListEntry {
                next: None,
                prev: None,
            }),
            value: val,
        }
    }
}
