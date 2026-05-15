mod pool;
mod spawner;

pub use pool::{Pool, run_with_spawner};
pub use spawner::Spawner;
use tokio::task::JoinHandle;
pub use tokio_util::sync::CancellationToken;
pub use tracing;

pub fn spawn_future<T: Send + 'static>(
    f: impl Future<Output = T> + Send + 'static,
    name: &'static str,
) -> JoinHandle<T> {
    #[cfg(tokio_unstable)]
    {
        tokio::task::Builder::new()
            .name(name)
            .spawn(f)
            .expect("spawning future should not fail")
    }
    #[cfg(not(tokio_unstable))]
    {
        let _ = name;
        tokio::task::spawn(f)
    }
}
