use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    pin::pin,
    sync::{Arc, atomic::AtomicBool},
    task::Poll,
};

use color_eyre::Result;
use futures_intrusive::sync::{ManualResetEvent, WaitForEventFuture};
pub use futures_util::{Stream, StreamExt};
use pin_project_lite::pin_project;
use spawn::Spawner;
pub use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
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
    #[must_use]
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
    sender: UnboundedSender<Result<W::F>>,
    spawner: Spawner,
    cancel: Cancellation,
}

impl<A, W: Wrapper<A>> TaskSubmitter<A, W> {
    pub const fn as_ref(&self) -> TaskSubmitterRef<'_, A, W> {
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
    sender: &'r UnboundedSender<Result<W::F>>,
    spawner: &'r Spawner,
    cancel: &'r Cancellation,
}

impl<A, W: Wrapper<A> + Copy> Copy for TaskSubmitterRef<'_, A, W> {}

impl<A, W: Wrapper<A> + Clone> Clone for TaskSubmitterRef<'_, A, W> {
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
    pub struct Cancelled<'c,F>{
        #[pin]
        pub f:F,
        #[pin]
        pub cancel: WaitForEventFuture<'c>,
    }
}

impl<F: Future<Output = ()>> Future for Cancelled<'_, F> {
    type Output = ();

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let s = self.project();
        if s.cancel.poll(cx) == Poll::Ready(()) {
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
    receiver: UnboundedReceiver<Result<T>>,
    _cancel: DropGuard,
}

impl<T> DerefMut for EventReceiver<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.receiver
    }
}

impl<T> Deref for EventReceiver<T> {
    type Target = UnboundedReceiver<Result<T>>;

    fn deref(&self) -> &Self::Target {
        &self.receiver
    }
}

#[must_use]
pub fn new_task_pair<T: Send + 'static>(
    spawner: Spawner,
) -> (TaskSubmitter<T, IdWrapper>, EventReceiver<T>) {
    let (sender, receiver) = unbounded_channel();
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
    pub const fn wrap_with<AN, WN: Wrapper<AN, F = A>>(
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

    pub const fn wrapper(&self) -> W {
        self.wrapper
    }

    pub const fn sender(&self) -> &UnboundedSender<Result<W::F>> {
        self.sender
    }

    pub const fn cancel_token(&self) -> &Cancellation {
        self.cancel
    }
}

impl<A: Send + 'static, W: Wrapper<A>> TaskSubmitterRef<'_, A, W> {
    pub fn erased(&self) -> impl ErasedSubmitter<A> {
        ErasedSubmitterImpl {
            wrapper: self.wrapper,
            sender: self.sender.clone(),
        }
    }

    #[track_caller]
    pub fn spawn_task(
        &self,
        fut: impl Future<Output = Result<A>> + Send + 'static,
        span: Span,
        name: &'static str,
    ) {
        let wrapper = self.wrapper;
        let sender = self.sender.clone();
        let cancel = self.cancel.clone();
        self.spawner.spawn(
            async move {
                let inner = async {
                    let _ = sender.send(fut.await.map(|v| wrapper.wrap(v)));
                };
                Cancelled {
                    f: inner,
                    cancel: cancel.cancelled(),
                }
                .await;
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
        let sender = self.sender.clone();
        let cancel = self.cancel.clone();
        self.spawner.spawn(
            async move {
                let inner = async {
                    let _ = sender.send(Ok(wrapper.wrap(fut.await)));
                };
                Cancelled {
                    f: inner,
                    cancel: cancel.cancelled(),
                }
                .await;
            },
            span,
            name,
        );
    }

    #[track_caller]
    pub fn spawn_stream(
        &self,
        stream: impl Stream<Item = Result<A>> + Send + 'static,
        span: Span,
        name: &'static str,
    ) {
        let wrapper = self.wrapper;
        let sender = self.sender.clone();
        let cancel = self.cancel.clone();
        self.spawner.spawn(
            async move {
                let inner = async {
                    let mut stream = pin!(stream);
                    while let Some(v) = stream.next().await {
                        if sender.send(v.map(|v| wrapper.wrap(v))).is_err() {
                            break;
                        }
                    }
                };
                Cancelled {
                    f: inner,
                    cancel: cancel.cancelled(),
                }
                .await;
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
        let sender = self.sender.clone();
        let cancel = self.cancel.clone();
        self.spawner.spawn(
            async move {
                let inner = async {
                    match fut.await {
                        Ok(v) => {
                            let _ = sender.send(Ok(wrapper.wrap(v)));
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
                .await;
            },
            span,
            name,
        );
    }

    #[track_caller]
    pub fn spawn_value(&self, val: Result<A>) {
        let _ = self.sender.send(val.map(|v| self.wrapper.wrap(v)));
    }

    #[track_caller]
    pub fn spawn_value_infallible(&self, val: A) {
        let _ = self.sender.send(Ok(self.wrapper.wrap(val)));
    }
}

impl<A, W: Wrapper<A>> Deref for TaskSubmitterRef<'_, A, W> {
    type Target = Spawner;

    fn deref(&self) -> &Self::Target {
        self.spawner
    }
}

pub trait ErasedSubmitter<A: 'static>: Send + Sync + Debug + 'static {
    #[track_caller]
    fn spawn_value(&self, val: Result<A>);
    #[track_caller]
    fn spawn_value_infallible(&self, val: A);
}

struct ErasedSubmitterImpl<A: 'static, W: Wrapper<A>> {
    wrapper: W,
    sender: UnboundedSender<Result<W::F>>,
}

impl<A: 'static, W: Wrapper<A>> Debug for ErasedSubmitterImpl<A, W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ErasedSubmitterImpl")
            .finish_non_exhaustive()
    }
}

impl<A: 'static, W: Wrapper<A>> ErasedSubmitter<A> for ErasedSubmitterImpl<A, W> {
    fn spawn_value(&self, val: Result<A>) {
        let _ = self.sender.send(val.map(|v| self.wrapper.wrap(v)));
    }

    fn spawn_value_infallible(&self, val: A) {
        let _ = self.sender.send(Ok(self.wrapper.wrap(val)));
    }
}
