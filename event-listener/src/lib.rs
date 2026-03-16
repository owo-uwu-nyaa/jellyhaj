use std::{
    collections::{HashMap, hash_map::Entry},
    pin::pin,
    sync::Arc,
    task::{Poll, ready},
};

use color_eyre::Result;
use futures_intrusive::sync::ManualResetEvent;
use jellyfin::{
    JellyfinClient,
    socket::{ChangedUserData, JellyfinMessage, LibraryChanged, RefreshProgress, UserDataChanged},
};
use jellyhaj_async_task::{Cancellation, Stream, StreamExt, TaskSubmitterRef, Wrapper};
use parking_lot::{Mutex, lock_api::MutexGuard};
use spawn::Spawner;
use tracing::{debug, info_span, instrument};

trait InterestInner<T> {
    fn poll_send(
        &self,
        cx: &mut std::task::Context<'_>,
        val: &mut Option<T>,
    ) -> std::task::Poll<()>;
}

struct InterestInnerImpl<T, W: Wrapper<T>> {
    wrapper: W,
    sender: Mutex<futures_channel::mpsc::Sender<Result<W::F>>>,
}

impl<T, W: Wrapper<T>> InterestInner<T> for InterestInnerImpl<T, W> {
    fn poll_send(
        &self,
        cx: &mut std::task::Context<'_>,
        val: &mut Option<T>,
    ) -> std::task::Poll<()> {
        let _ = ready!(self.sender.lock().poll_ready(cx));
        let _ = self.sender.lock().start_send(Ok(self
            .wrapper
            .wrap(val.take().expect("should not be empty"))));
        Poll::Ready(())
    }
}

#[derive(Clone)]
pub struct Interest<T> {
    inner: Arc<dyn InterestInner<T> + Send + Sync>,
    cancelled: Cancellation,
}

impl<T: Send + 'static> Interest<T> {
    pub fn new<W: Wrapper<T>>(submitter: TaskSubmitterRef<'_, T, W>) -> Self {
        Self {
            inner: Arc::new(InterestInnerImpl {
                wrapper: submitter.wrapper(),
                sender: Mutex::new(submitter.sender().clone()),
            }),
            cancelled: submitter.cancel_token().clone(),
        }
    }

    async fn send(&self, val: T) {
        let mut val = Some(val);
        std::future::poll_fn(|cx| self.inner.poll_send(cx, &mut val)).await;
    }
}

fn clean_vec<T>(v: &mut Vec<Interest<T>>) -> bool {
    v.retain(|i| !i.cancelled.is_cancelled());
    !v.is_empty()
}

fn clean_hash_map<K, T>(m: &mut HashMap<K, Vec<Interest<T>>>) -> bool {
    m.retain(|_, v| clean_vec(v));
    !m.is_empty()
}

#[derive(Debug, Clone)]
pub struct RefrshProgressInterest {
    pub item_id: String,
    pub progress: f32,
}

const CLEAN_INTERVAL: u8 = 128;

fn register<T: Send + 'static, W: Wrapper<T>>(
    map: &mut HashMap<String, Vec<Interest<T>>>,
    item_id: String,
    submitter: TaskSubmitterRef<'_, T, W>,
) {
    let interests = match map.entry(item_id) {
        Entry::Occupied(occupied) => occupied.into_mut(),
        Entry::Vacant(vacant) => {
            let interests = Vec::with_capacity(1);
            vacant.insert(interests)
        }
    };
    interests.push(Interest::new(submitter));
}

pub struct Interests {
    refresh_progress: HashMap<String, Vec<Interest<RefrshProgressInterest>>>,
    changed_user_data: HashMap<String, Vec<Interest<ChangedUserData>>>,
    folder_modified: HashMap<String, Vec<Interest<String>>>,
    item_updated: HashMap<String, Vec<Interest<String>>>,
    item_removed: HashMap<String, Vec<Interest<String>>>,
    clean_counter: u8,
}

impl Interests {
    fn clean(&mut self) {
        let (v, o) = self.clean_counter.overflowing_sub(1);
        self.clean_counter = v;
        if o {
            self.clean_counter = CLEAN_INTERVAL;
            clean_hash_map(&mut self.refresh_progress);
            clean_hash_map(&mut self.changed_user_data);
            clean_hash_map(&mut self.folder_modified);
            clean_hash_map(&mut self.item_updated);
            clean_hash_map(&mut self.item_removed);
        }
    }
    pub fn register_refrsh_progress(
        &mut self,
        item_id: String,
        submitter: TaskSubmitterRef<
            '_,
            RefrshProgressInterest,
            impl Wrapper<RefrshProgressInterest>,
        >,
    ) {
        register(&mut self.refresh_progress, item_id, submitter);
    }
    pub fn register_changed_userdata(
        &mut self,
        item_id: String,
        submitter: TaskSubmitterRef<'_, ChangedUserData, impl Wrapper<ChangedUserData>>,
    ) {
        register(&mut self.changed_user_data, item_id, submitter);
    }
    pub fn register_folder_modified(
        &mut self,
        item_id: String,
        submitter: TaskSubmitterRef<'_, String, impl Wrapper<String>>,
    ) {
        register(&mut self.folder_modified, item_id, submitter);
    }
    pub fn register_item_updated(
        &mut self,
        item_id: String,
        submitter: TaskSubmitterRef<'_, String, impl Wrapper<String>>,
    ) {
        register(&mut self.item_updated, item_id, submitter);
    }
    pub fn register_item_removed(
        &mut self,
        item_id: String,
        submitter: TaskSubmitterRef<'_, String, impl Wrapper<String>>,
    ) {
        register(&mut self.item_removed, item_id, submitter);
    }
}

struct CancelSocket {
    inner: Arc<ManualResetEvent>,
}

impl Drop for CancelSocket {
    fn drop(&mut self) {
        self.inner.set();
    }
}

#[derive(Clone)]
pub struct JellyfinEventInterests {
    inner: Arc<Mutex<Interests>>,
    cancel: Arc<CancelSocket>,
}

impl JellyfinEventInterests {
    pub fn get(&self) -> parking_lot::MutexGuard<'_, Interests> {
        let mut guard = self.inner.lock();
        guard.clean();
        guard
    }
    pub fn new(spawn: &Spawner, jellyfin: &JellyfinClient) -> Result<Self> {
        let this = JellyfinEventInterests {
            inner: Arc::new(Mutex::new(Interests {
                refresh_progress: HashMap::new(),
                changed_user_data: HashMap::new(),
                folder_modified: HashMap::new(),
                item_updated: HashMap::new(),
                item_removed: HashMap::new(),
                clean_counter: CLEAN_INTERVAL,
            })),
            cancel: Arc::new(CancelSocket {
                inner: Arc::new(ManualResetEvent::new(false)),
            }),
        };
        let stream = jellyfin.get_socket()?;
        spawn.spawn(
            poll_socket_cancellable(this.inner.clone(), stream, this.cancel.inner.clone()),
            info_span!("poll_jellyfin_socket"),
            "poll_jellyfin_socket",
        );
        Ok(this)
    }
}

async fn poll_socket_cancellable(
    interests: Arc<Mutex<Interests>>,
    stream: impl Stream<Item = JellyfinMessage>,
    cancel: Arc<ManualResetEvent>,
) {
    tokio::select! {
        _ = jellyfin_poll_socket(interests, stream) => {
            debug!("socket closed");
        }
        _ = cancel.wait() => {
            debug!("interests dropped");
        }
    }
}

#[instrument(skip_all)]
async fn jellyfin_poll_socket(
    interests: Arc<Mutex<Interests>>,
    stream: impl Stream<Item = JellyfinMessage>,
) {
    let mut stream = pin!(stream);
    while let Some(message) = stream.next().await {
        debug!("received message {message:?}");
        match message {
            JellyfinMessage::RefreshProgress {
                data: RefreshProgress { item_id, progress },
            } => {
                let vec = if let Ok(mut guard) =
                    MutexGuard::try_map(interests.lock(), |i| i.refresh_progress.get_mut(&item_id))
                    && clean_vec(&mut guard)
                {
                    Vec::clone(&guard)
                } else {
                    continue;
                };
                let message = RefrshProgressInterest { item_id, progress };
                for i in vec {
                    i.send(message.clone()).await
                }
            }
            JellyfinMessage::UserDataChanged {
                data:
                    UserDataChanged {
                        user_data_list,
                        user_id:_,
                    },
            } => {
                for change in user_data_list {
                    let vec = if let Ok(mut guard) = MutexGuard::try_map(interests.lock(), |i| {
                        i.changed_user_data.get_mut(&change.item_id)
                    }) && clean_vec(&mut guard)
                    {
                        Vec::clone(&guard)
                    } else {
                        continue;
                    };
                    for i in vec {
                        i.send(change.clone()).await
                    }
                }
            }
            JellyfinMessage::LibraryChanged {
                data:
                    LibraryChanged {
                        collection_folders,
                        folders_added_to:_,
                        folders_removed_from:_,
                        items_added:_,
                        items_removed,
                        items_updated,
                    },
            } => {
                for folder in collection_folders {
                    let vec = if let Ok(mut guard) = MutexGuard::try_map(interests.lock(), |i| {
                        i.folder_modified.get_mut(&folder)
                    }) && clean_vec(&mut guard)
                    {
                        Vec::clone(&guard)
                    } else {
                        continue;
                    };
                    for i in vec {
                        i.send(folder.clone()).await
                    }
                }
                for removed in items_removed {
                    let vec = if let Ok(mut guard) =
                        MutexGuard::try_map(interests.lock(), |i| i.item_removed.get_mut(&removed))
                        && clean_vec(&mut guard)
                    {
                        Vec::clone(&guard)
                    } else {
                        continue;
                    };
                    for i in vec {
                        i.send(removed.clone()).await
                    }
                }
                for updated in items_updated {
                    let vec = if let Ok(mut guard) =
                        MutexGuard::try_map(interests.lock(), |i| i.item_updated.get_mut(&updated))
                        && clean_vec(&mut guard)
                    {
                        Vec::clone(&guard)
                    } else {
                        continue;
                    };
                    for i in vec {
                        i.send(updated.clone()).await
                    }
                }
            }
            JellyfinMessage::Unknown {
                message_type: _,
                data: _,
            } => {}
        }
    }
}
