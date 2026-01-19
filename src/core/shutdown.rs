use std::sync::OnceLock;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::log_debug;

// This handles graceful shutdown of background tasks

pub struct ShutdownManager {
    token: CancellationToken,
    tasks: Arc<Mutex<Vec<(&'static str, JoinHandle<()>)>>>,
}

impl ShutdownManager {
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
            tasks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }

    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }

    pub async fn register_task(&self, name: &'static str, handle: JoinHandle<()>) {
        let mut tasks = self.tasks.lock().await;
        tasks.push((name, handle));
    }

    pub async fn spawn<F>(&self, name: &'static str, future: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let token = self.token.clone();
        let handle = tokio::spawn(async move {
            tokio::select! {
                biased;
                _ = token.cancelled() => {
                    // Task cancelled by shutdown
                }
                _ = future => {
                    // Task completed normally
                }
            }
        });
        self.register_task(name, handle).await;
    }

    pub async fn spawn_loop<F, Fut>(&self, name: &'static str, mut loop_fn: F)
    where
        F: FnMut() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        let token = self.token.clone();
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    biased;
                    _ = token.cancelled() => {
                        break;
                    }
                    _ = loop_fn() => {
                        // continue
                    }
                }
            }
        });
        self.register_task(name, handle).await;
    }

    pub async fn shutdown(&self) {
        log_debug!("[ShutdownManager] Initiating graceful shutdown...");
        self.token.cancel();

        let tasks_to_wait = {
            let mut tasks = self.tasks.lock().await;
            tasks.drain(..).collect::<Vec<_>>()
        };

        for (name, handle) in tasks_to_wait {
            log_debug!("[ShutdownManager] Waiting for task: {}", name);
            match tokio::time::timeout(std::time::Duration::from_secs(5), handle).await {
                Ok(Ok(())) => log_debug!("[ShutdownManager] Task '{}' completed", name),
                Ok(Err(e)) => log_debug!("[ShutdownManager] Task '{}' panicked: {}", name, e),
                Err(_) => log_debug!("[ShutdownManager] Task '{}' timed out", name),
            }
        }

        log_debug!("[ShutdownManager] Shutdown complete");
    }

    pub async fn task_count(&self) -> usize {
        self.tasks.lock().await.len()
    }
}

impl Default for ShutdownManager {
    fn default() -> Self { Self::new() }
}

static SHUTDOWN_MANAGER: OnceLock<ShutdownManager> = OnceLock::new();
pub fn get_shutdown_manager() -> &'static ShutdownManager {
    SHUTDOWN_MANAGER.get_or_init(ShutdownManager::new)
}
