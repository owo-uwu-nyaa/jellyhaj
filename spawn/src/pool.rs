use std::task::{Poll, ready};

use crate::{Spawner, spawner::JoinSetCallback};
use pin_project_lite::pin_project;
use tokio::{sync::mpsc::UnboundedReceiver, task::JoinSet};
use tokio_util::sync::{CancellationToken, WaitForCancellationFutureOwned};
use tracing::{Instrument, Span};

pin_project! {
    pub struct Pool<T> where T: Send {
        recv: UnboundedReceiver<JoinSetCallback>,
        pool: JoinSet<()>,
        closed: bool,
        cancellation: CancellationToken,
        #[pin]
        cancellation_fut : WaitForCancellationFutureOwned,
        #[pin]
        res: tokio::sync::oneshot::Receiver<T>
    }
}

impl<T: Send> Future for Pool<T> {
    type Output = Option<T>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut this = self.as_mut().project();
        loop {
            if *this.closed {
                loop {
                    match this.pool.poll_join_next(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(None) => {
                            break;
                        }
                        Poll::Ready(Some(_)) => {}
                    }
                }
                let res = ready!(this.res.poll(cx));
                break Poll::Ready(res.ok());
            } else if this.cancellation_fut.as_mut().poll(cx).is_ready() {
                this.pool.abort_all();
                *this.closed = true;
            } else {
                let empty = loop {
                    let pool_res = this.pool.poll_join_next(cx);
                    match pool_res {
                        Poll::Pending => break false,
                        Poll::Ready(None) => break true,
                        Poll::Ready(Some(Ok(()))) => {}
                        Poll::Ready(Some(Err(e))) => {
                            if e.is_panic() {
                                this.pool.abort_all();
                                *this.closed = true;
                                this.cancellation.cancel();
                                return self.poll(cx);
                            }
                        }
                    }
                };
                while let Poll::Ready(Some(job)) = this.recv.poll_recv(cx) {
                    job(this.pool)
                }
                if empty && this.pool.is_empty() {
                    *this.closed = true;
                } else {
                    return Poll::Pending;
                }
            }
        }
    }
}

pub fn run_with_spawner<T: Send + 'static, F: Future<Output = T> + Send + 'static>(
    f: impl FnOnce(Spawner) -> F,
    cancel: CancellationToken,
    span: Span,
) -> Pool<T> {
    let (send, recv) = tokio::sync::mpsc::unbounded_channel();
    let mut pool = JoinSet::new();
    let spawn = Spawner { sender: send };
    let f = f(spawn).instrument(span);
    let (send, res) = tokio::sync::oneshot::channel();
    pool.spawn(async move {
        let _ = send.send(f.await);
    });
    let fut = cancel.clone().cancelled_owned();
    Pool {
        recv,
        pool,
        closed: false,
        cancellation: cancel,
        cancellation_fut: fut,
        res,
    }
}
