use std::{
    ops::Deref,
    pin::{Pin, pin},
    sync::{Arc, atomic::AtomicBool},
    task::Poll,
};

use color_eyre::Result;
pub use futures_channel::mpsc::SendError;
use futures_channel::mpsc::{Receiver, Sender, channel};
use futures_intrusive::sync::{ManualResetEvent, WaitForEventFuture};
pub use futures_util::{Sink, SinkExt, Stream, StreamExt};
use pin_project_lite::pin_project;
use spawn::Spawner;
use tracing::Span;

struct CancellationInner {
    event: ManualResetEvent,
    cancelled: AtomicBool,
}

#[derive(Clone)]
pub struct Cancellation {
    inner: Arc<CancellationInner>,
}

impl Cancellation {
    pub fn is_cancelled(&self) -> bool {
        self.inner
            .cancelled
            .load(std::sync::atomic::Ordering::Relaxed)
    }
    pub fn cancel(&self) {
        self.inner
            .cancelled
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.inner.event.set();
    }
    pub fn cancelled(&self) -> WaitForEventFuture<'_> {
        self.inner.event.wait()
    }
}

struct DropGuard {
    inner: Cancellation,
}

impl Drop for DropGuard {
    fn drop(&mut self) {
        self.inner.cancel();
    }
}

pub trait Wrapper<C>: Clone + Copy + Send + Sync + 'static {
    type F: Send + 'static;
    fn wrap(&self, val: C) -> Self::F;
}

impl<A, R: Send + 'static, F: Clone + Copy + Send + Sync + 'static + Fn(A) -> R> Wrapper<A> for F {
    type F = R;
    fn wrap(&self, val: A) -> Self::F {
        self(val)
    }
}

pub struct TaskSubmitter<A, W: Wrapper<A>> {
    wrapper: W,
    sender: Sender<Result<W::F>>,
    spawner: Spawner,
    cancel: Cancellation,
}

impl<A, W: Wrapper<A>> TaskSubmitter<A, W> {
    pub fn as_ref(&self) -> TaskSubmitterRef<'_, A, W> {
        TaskSubmitterRef {
            wrapper: self.wrapper,
            sender: &self.sender,
            spawner: &self.spawner,
            cancel: &self.cancel,
        }
    }
}

pub struct TaskSubmitterRef<'r, A, W: Wrapper<A>> {
    wrapper: W,
    sender: &'r Sender<Result<W::F>>,
    spawner: &'r Spawner,
    cancel: &'r Cancellation,
}

impl<'r, A, W: Wrapper<A> + Copy> Copy for TaskSubmitterRef<'r, A, W> {}

impl<'r, A, W: Wrapper<A> + Clone> Clone for TaskSubmitterRef<'r, A, W> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, W: Wrapper<A>> Clone for TaskSubmitter<A, W> {
    fn clone(&self) -> Self {
        Self {
            wrapper: self.wrapper,
            sender: self.sender.clone(),
            spawner: self.spawner.clone(),
            cancel: self.cancel.clone(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Wrapped<W1, W2> {
    w1: W1,
    w2: W2,
}

impl<W1, W2, C> Wrapper<C> for Wrapped<W1, W2>
where
    W1: Wrapper<C>,
    W2: Wrapper<W1::F>,
{
    type F = W2::F;

    fn wrap(&self, val: C) -> Self::F {
        self.w2.wrap(self.w1.wrap(val))
    }
}

pin_project! {
    struct Cancelled<'c,F>{
        #[pin]
        f:F,
        #[pin]
        cancel: WaitForEventFuture<'c>,
    }
}

impl<'c, F: Future<Output = ()>> Future for Cancelled<'c, F> {
    type Output = ();

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let s = self.project();
        if let Poll::Ready(()) = s.cancel.poll(cx) {
            Poll::Ready(())
        } else {
            s.f.poll(cx)
        }
    }
}

#[derive(Clone, Copy)]
pub struct IdWrapper;

impl<T: Send + 'static> Wrapper<T> for IdWrapper {
    type F = T;

    fn wrap(&self, val: T) -> Self::F {
        val
    }
}

pub struct EventReceiver<T> {
    receiver: Receiver<Result<T>>,
    _cancel: DropGuard,
}

impl<T> Stream for EventReceiver<T> {
    type Item = Result<T>;
    #[inline]
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().receiver).poll_next(cx)
    }
}

pub fn new_task_pair<T: Send + 'static>(
    spawner: Spawner,
) -> (TaskSubmitter<T, IdWrapper>, EventReceiver<T>) {
    let (sender, receiver) = channel(16);
    let cancel = Cancellation {
        inner: Arc::new(CancellationInner {
            event: ManualResetEvent::new(false),
            cancelled: AtomicBool::new(false),
        }),
    };
    let receiver = EventReceiver {
        receiver,
        _cancel: DropGuard {
            inner: cancel.clone(),
        },
    };
    (
        TaskSubmitter {
            wrapper: IdWrapper,
            sender,
            spawner,
            cancel,
        },
        receiver,
    )
}

impl<'r, A, W: Wrapper<A>> TaskSubmitterRef<'r, A, W> {
    pub fn wrap_with<AN, WN: Wrapper<AN, F = A>>(
        self,
        wrapper: WN,
    ) -> TaskSubmitterRef<'r, AN, Wrapped<WN, W>> {
        TaskSubmitterRef {
            wrapper: Wrapped {
                w1: wrapper,
                w2: self.wrapper,
            },
            sender: self.sender,
            spawner: self.spawner,
            cancel: self.cancel,
        }
    }

    pub fn wrapper(&self) -> W {
        self.wrapper
    }

    pub fn sender(&self) -> &Sender<Result<W::F>> {
        self.sender
    }

    pub fn cancel_token(&self) -> &Cancellation {
        self.cancel
    }

    #[track_caller]
    pub fn spawn_task(
        &self,
        fut: impl Future<Output = Result<A>> + Send + 'static,
        span: Span,
        name: &'static str,
    ) {
        let wrapper = self.wrapper;
        let mut sender = self.sender.clone();
        let cancel = self.cancel.clone();
        self.spawner.spawn(
            async move {
                let inner = async {
                    let _ = sender.feed(fut.await.map(|v| wrapper.wrap(v))).await;
                };
                Cancelled {
                    f: inner,
                    cancel: cancel.cancelled(),
                }
                .await
            },
            span,
            name,
        );
    }

    #[track_caller]
    pub fn spawn_task_infallible(
        &self,
        fut: impl Future<Output = A> + Send + 'static,
        span: Span,
        name: &'static str,
    ) {
        let wrapper = self.wrapper;
        let mut sender = self.sender.clone();
        let cancel = self.cancel.clone();
        self.spawner.spawn(
            async move {
                let inner = async {
                    let _ = sender.feed(Ok(wrapper.wrap(fut.await))).await;
                };
                Cancelled {
                    f: inner,
                    cancel: cancel.cancelled(),
                }
                .await
            },
            span,
            name,
        );
    }
}

impl<'r, A: Send, W: Wrapper<A>> TaskSubmitterRef<'r, A, W> {
    #[track_caller]
    pub fn spawn_stream(
        &self,
        stream: impl Stream<Item = Result<A>> + Send + 'static,
        span: Span,
        name: &'static str,
    ) {
        let wrapper = self.wrapper;
        let mut sender = self.sender.clone();
        let cancel = self.cancel.clone();
        self.spawner.spawn(
            async move {
                let inner = async {
                    let mut stream = pin!(stream);
                    while let Some(v) = stream.next().await {
                        if sender.feed(v.map(|v| wrapper.wrap(v))).await.is_err() {
                            break;
                        }
                    }
                };
                Cancelled {
                    f: inner,
                    cancel: cancel.cancelled(),
                }
                .await
            },
            span,
            name,
        );
    }

    #[track_caller]
    pub fn spawn_task_suppressed_error(
        &self,
        fut: impl Future<Output = Result<A>> + Send + 'static,
        span: Span,
        name: &'static str,
    ) {
        let wrapper = self.wrapper;
        let mut sender = self.sender.clone();
        let cancel = self.cancel.clone();
        self.spawner.spawn(
            async move {
                let inner = async {
                    match fut.await {
                        Ok(v) => {
                            let _ = sender.feed(Ok(wrapper.wrap(v))).await;
                        }
                        Err(e) => {
                            tracing::error!("task returned suppressed error:\n{e:?}");
                        }
                    }
                };
                Cancelled {
                    f: inner,
                    cancel: cancel.cancelled(),
                }
                .await
            },
            span,
            name,
        );
    }
}

impl<'r, A, W: Wrapper<A>> Deref for TaskSubmitterRef<'r, A, W> {
    type Target = Spawner;

    fn deref(&self) -> &Self::Target {
        self.spawner
    }
}
