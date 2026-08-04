use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use futures_util::{FutureExt, future::BoxFuture};
use jellyhaj_widgets_core::{
    Result,
    async_task::{UnboundedReceiver, UnboundedSender},
};
use keybinds::KeybindEvents;
use parking_lot::RwLock;
use ratatui::DefaultTerminal;
use tokio::select;
use tracing::{debug, info, instrument, warn};

use crate::{
    state::{Navigation, NextScreen},
    term::run_without,
    widgets::{
        RunResult, WidgetCreator,
        list::{
            ListAccessToken, ListEntry, StateEntry, inspect_list, prepend_element, remove_element,
        },
        shaded::{
            render::{RenderStopRes, render_widget, render_widget_stop},
            widget::Erased,
        },
        suspended::SuspendedInner,
    },
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
    pub fn push(&self, widget: Erased, widget_creator: WidgetCreator) {
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

pub async fn render_loop(
    initial: NextScreen,
    widget_creator: WidgetCreator,
    state: &StateStack,
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    external: &mut UnboundedReceiver<NextScreen>,
) {
    let mut top = Some(initial);
    loop {
        let mut widget = if let Some(top) = top.take() {
            debug!("running top next screen");
            widget_creator(top)
        } else {
            match state.pop() {
                StateValue::Suspended(suspended) => {
                    debug!("resuming suspended widget: {}", suspended.name);
                    match suspended.get_widget().await {
                        RunResult::Cont(erased_widget) => erased_widget,
                        RunResult::Empty => continue,
                        RunResult::Exit => break,
                    }
                }
                StateValue::Empty => {
                    info!("stack is now empty");
                    break;
                }
                StateValue::WithoutTui(without_tui) => {
                    if let Err(e) = run_without(without_tui).await {
                        widget_creator(NextScreen::Error(e))
                    } else {
                        continue;
                    }
                }
            }
        };
        select! {
            nav = render_widget(widget.as_mut(), events, term).map(Navigation::from) => {
                match nav
                {
                    Navigation::Push(next) => {
                        state.push(widget, widget_creator.clone());
                        top = Some(next);
                    }
                    Navigation::PopContext => {
                        match render_widget_stop::<_>(widget.as_mut(), events, term).await {
                            RenderStopRes::Ok => {}
                            RenderStopRes::Exit => break,
                        }
                    }
                    Navigation::Replace(next) => {
                        match render_widget_stop(widget.as_mut(), events, term).await {
                            RenderStopRes::Ok => top = Some(next),
                            RenderStopRes::Exit => break,
                        }
                    }
                    Navigation::Exit => break,
                    Navigation::PushWithoutTui(without_tui) => {
                        if let Err(e) = run_without(without_tui).await {
                            top = Some(NextScreen::Error(e));
                        }
                    }
                }
            }
            next = external.recv() => {
                if let Some(next) = next{
                    state.push(widget, widget_creator.clone());
                    top = Some(next);
                }else{
                    warn!("external widget queue is closed, exit");
                    break
                }
            }
        }
    }
}

pub struct WidgetPusher {
    inner: UnboundedSender<NextScreen>,
}
impl DerefMut for WidgetPusher {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Deref for WidgetPusher {
    type Target = UnboundedSender<NextScreen>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
