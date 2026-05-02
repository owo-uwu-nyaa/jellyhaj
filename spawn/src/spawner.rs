use std::sync::Arc;

use parking_lot::Mutex;
use tokio::task::JoinSet;
use tracing::{Instrument, Span, warn};

#[derive(Clone)]
pub struct Spawner {
    pub(crate) pool: Arc<Mutex<JoinSet<()>>>,
}

impl Spawner {
    #[track_caller]
    fn spawn_bare(&self, fut: impl Future<Output = ()> + Send + 'static, name: &'static str) {
        #[cfg(not(tokio_unstable))]
        let _ = name;
        let mut join_set = self.pool.lock();
        #[cfg(tokio_unstable)]
        join_set
            .build_task()
            .name(name)
            .spawn(fut)
            .expect("spawning future should not fail");
        #[cfg(not(tokio_unstable))]
        join_set.spawn(fut);
    }
    #[track_caller]
    pub fn spawn(
        &self,
        fut: impl Future<Output = ()> + Send + 'static,
        span: Span,
        name: &'static str,
    ) {
        self.spawn_bare(fut.instrument(span), name);
    }
    #[track_caller]
    pub fn spawn_res<T>(
        &self,
        fut: impl Future<Output = color_eyre::Result<T>> + Send + 'static,
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
