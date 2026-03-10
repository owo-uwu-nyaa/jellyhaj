use std::{
    sync::Arc,
    task::{Poll, ready},
};

use crate::Spawner;
use parking_lot::Mutex;
use pin_project_lite::pin_project;
use tokio::task::JoinSet;
use tokio_util::sync::{CancellationToken, WaitForCancellationFutureOwned};
use tracing::Span;

pin_project! {
    pub struct Pool<T> where T: Send {
        pool: Arc<Mutex<JoinSet<()>>>,
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
        'poll: loop {
            if *this.closed {
                loop {
                    match this.pool.lock().poll_join_next(cx) {
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
                this.pool.lock().abort_all();
                *this.closed = true;
            } else {
                let mut pool = this.pool.lock();
                loop {
                    let pool_res = pool.poll_join_next(cx);
                    match pool_res {
                        Poll::Pending => break,
                        Poll::Ready(None) => {
                            *this.closed = true;
                            this.cancellation.cancel();
                            continue 'poll;
                        }
                        Poll::Ready(Some(Ok(()))) => {}
                        Poll::Ready(Some(Err(e))) => {
                            if e.is_panic() {
                                this.pool.lock().abort_all();
                                *this.closed = true;
                                this.cancellation.cancel();
                                continue 'poll;
                            }
                        }
                    }
                }
                return Poll::Pending;
            }
        }
    }
}

#[track_caller]
pub fn run_with_spawner<T: Send + 'static, F: Future<Output = T> + Send + 'static>(
    f: impl FnOnce(Spawner) -> F,
    cancel: CancellationToken,
    span: Span,
    name: &'static str,
) -> Pool<T> {
    let pool = Arc::new(Mutex::new(JoinSet::new()));
    let spawn = Spawner { pool: pool.clone() };
    let (send, res) = tokio::sync::oneshot::channel();
    let f = f(spawn.clone());
    spawn.spawn(
        async move {
            let _ = send.send(f.await);
        },
        span,
        name,
    );
    let fut = cancel.clone().cancelled_owned();
    Pool {
        pool,
        closed: false,
        cancellation: cancel,
        cancellation_fut: fut,
        res,
    }
}
