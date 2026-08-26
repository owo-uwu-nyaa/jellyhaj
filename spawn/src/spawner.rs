use std::{
    cell::Cell,
    rc::Rc,
    task::{Poll, Waker},
};

use tracing::{Instrument, Span, warn};

pub struct SpawnerInner {
    in_flight: Cell<usize>,
    waker: Cell<Option<Waker>>,
}

impl SpawnerInner {
    pub(crate) const fn new() -> Self {
        Self {
            in_flight: Cell::new(0),
            waker: Cell::new(None),
        }
    }
}

pub struct PoolClosed {
    inner: Rc<SpawnerInner>,
}

impl PoolClosed {
    pub(crate) const fn new(inner: Rc<SpawnerInner>) -> Self {
        Self { inner }
    }
}

impl Future for PoolClosed {
    type Output = ();

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.inner.in_flight.get() == 0 {
            Poll::Ready(())
        } else {
            this.inner.waker.set(Some(cx.waker().clone()));
            Poll::Pending
        }
    }
}

impl Drop for PoolClosed {
    ///Safety: `Self` is `Unpin`
    fn drop(&mut self) {
        self.inner.waker.set(None);
    }
}

struct InFlight<F> {
    f: F,
    inner: Rc<SpawnerInner>,
}

impl<F: Future> Future for InFlight<F> {
    type Output = F::Output;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        unsafe { self.map_unchecked_mut(|v| &mut v.f) }.poll(cx)
    }
}

impl<F> Drop for InFlight<F> {
    // SAFETY: `inner` is aöways projected without pin
    fn drop(&mut self) {
        self.inner.in_flight.update(|c| {
            c.checked_sub(1)
                .expect("Count should never drop below 0! Did the constructor increment in_flight?")
        });
        if self.inner.in_flight.get() == 0
            && let Some(waker) = self.inner.waker.replace(None)
        {
            waker.wake();
        }
    }
}

#[derive(Clone)]
pub struct Spawner {
    inner: Rc<SpawnerInner>,
}

impl Spawner {
    pub(crate) const fn new(inner: Rc<SpawnerInner>) -> Self {
        Self { inner }
    }

    #[track_caller]
    fn spawn_bare(&self, fut: impl Future<Output = ()> + 'static, name: &'static str) {
        self.inner.in_flight.update(|v| v + 1);
        let fut = InFlight {
            f: fut,
            inner: self.inner.clone(),
        };
        let _handle = crate::spawn_future(fut, name);
    }
    #[track_caller]
    pub fn spawn(&self, fut: impl Future<Output = ()> + 'static, span: Span, name: &'static str) {
        self.spawn_bare(fut.instrument(span), name);
    }
    #[track_caller]
    pub fn spawn_res<T>(
        &self,
        fut: impl Future<Output = color_eyre::Result<T>> + 'static,
        span: Span,
        name: &'static str,
    ) {
        self.spawn(
            async move {
                if let Err(e) = fut.await {
                    warn!("error returned from task: {e:?}");
                }
            },
            span,
            name,
        );
    }
}
