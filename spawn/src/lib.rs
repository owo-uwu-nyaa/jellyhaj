mod spawner;

use color_eyre::eyre::Context;
pub use spawner::Spawner;
use std::{pin::pin, rc::Rc, time::Duration};
use tokio::{select, sync::mpsc::UnboundedReceiver, task::JoinHandle};
pub use tracing;
use tracing::{Instrument, Span, info, warn};

use crate::spawner::{PoolClosed, SpawnerInner};

pub fn spawn_future<T: 'static>(
    f: impl Future<Output = T> + 'static,
    name: &'static str,
) -> JoinHandle<T> {
    #[cfg(tokio_unstable)]
    {
        tokio::task::Builder::new()
            .name(name)
            .spawn_local(f)
            .expect("spawning future should not fail")
    }
    #[cfg(not(tokio_unstable))]
    {
        let _ = name;
        tokio::task::spawn_local(f)
    }
}

async fn run_inner(
    inner: JoinHandle<color_eyre::Result<()>>,
    mut panics: UnboundedReceiver<color_eyre::Report>,
    pool_closed: spawner::PoolClosed,
) -> color_eyre::Result<()> {
    let mut inner = pin!(inner);
    let mut cancel = pin!(tokio::signal::ctrl_c());
    let mut queue_empty = false;
    let res = loop {
        select! {
            res = &mut inner => {
                break res.context("executing main task").flatten()
            }
            res = panics.recv(), if ! queue_empty => {
                if let Some(res) = res{
                    break Err(res);
                }
                queue_empty = true;
            }
            res = &mut cancel => {
                info!("interrupt received");
                break res.context("")
            }
        }
    };
    select! {
        () = pool_closed => {
            res
        }
        () = tokio::time::sleep(Duration::from_secs(10)) => {
            warn!("timeout reached");
            res
        }
    }
}

pub fn run_with_spawner<F: Future<Output = color_eyre::Result<()>> + 'static>(
    f: impl FnOnce(Spawner) -> F,
    span: Span,
    name: &'static str,
    panics: UnboundedReceiver<color_eyre::Report>,
) -> color_eyre::Result<()> {
    let rt = tokio::runtime::LocalRuntime::new()?;
    let inner = Rc::new(SpawnerInner::new());

    let handle = {
        let _guard = rt.enter();
        spawn_future(f(Spawner::new(inner.clone())).instrument(span), name)
    };
    let res = rt.block_on(run_inner(handle, panics, PoolClosed::new(inner)));
    rt.shutdown_timeout(Duration::from_secs(1));
    res
}
