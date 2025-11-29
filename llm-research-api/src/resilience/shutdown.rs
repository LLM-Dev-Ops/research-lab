//! Graceful shutdown coordination for the application.
//!
//! This module provides utilities for handling shutdown signals and coordinating
//! graceful shutdown across different components of the application.
//!
//! # Example
//!
//! ```no_run
//! use llm_research_api::resilience::shutdown::{ShutdownSignal, ShutdownCoordinator};
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let signal = ShutdownSignal::new();
//! let coordinator = ShutdownCoordinator::new(Duration::from_secs(30));
//!
//! // Register shutdown hooks
//! coordinator.on_shutdown(|| async {
//!     println!("Shutting down gracefully...");
//! });
//!
//! // Wait for shutdown signal
//! signal.wait().await;
//!
//! // Perform graceful shutdown
//! coordinator.shutdown().await;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, timeout};
use tracing::{debug, info, warn};

/// Shutdown signal handler
#[derive(Clone)]
pub struct ShutdownSignal {
    triggered: Arc<AtomicBool>,
}

impl ShutdownSignal {
    /// Create a new shutdown signal
    pub fn new() -> Self {
        Self {
            triggered: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Wait for a shutdown signal (SIGTERM or SIGINT)
    pub async fn wait(&self) {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                info!("Received SIGINT (Ctrl+C)");
            }
            _ = terminate => {
                info!("Received SIGTERM");
            }
        }

        self.triggered.store(true, Ordering::SeqCst);
    }

    /// Check if shutdown has been triggered
    pub fn is_triggered(&self) -> bool {
        self.triggered.load(Ordering::SeqCst)
    }

    /// Manually trigger shutdown
    pub fn trigger(&self) {
        info!("Manually triggering shutdown");
        self.triggered.store(true, Ordering::SeqCst);
    }
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for components that support graceful shutdown
#[async_trait]
pub trait GracefulShutdown: Send + Sync {
    /// Perform graceful shutdown
    async fn shutdown(&self) -> Result<(), ShutdownError>;

    /// Get the name of this component
    fn name(&self) -> &str;
}

/// Shutdown errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum ShutdownError {
    /// Shutdown timed out
    #[error("Shutdown timed out after {0:?}")]
    Timeout(Duration),

    /// Component shutdown failed
    #[error("Component {component} shutdown failed: {reason}")]
    ComponentFailed { component: String, reason: String },

    /// Already shutting down
    #[error("Shutdown already in progress")]
    AlreadyShuttingDown,
}

/// Shutdown hook function type
type ShutdownHook = Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// Coordinator for graceful shutdown
pub struct ShutdownCoordinator {
    timeout: Duration,
    hooks: Arc<Mutex<Vec<ShutdownHook>>>,
    components: Arc<RwLock<Vec<Arc<dyn GracefulShutdown>>>>,
    active_connections: Arc<AtomicUsize>,
    is_shutting_down: Arc<AtomicBool>,
}

impl ShutdownCoordinator {
    /// Create a new shutdown coordinator
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            hooks: Arc::new(Mutex::new(Vec::new())),
            components: Arc::new(RwLock::new(Vec::new())),
            active_connections: Arc::new(AtomicUsize::new(0)),
            is_shutting_down: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Register a shutdown hook
    ///
    /// Hooks are executed in the order they are registered
    pub fn on_shutdown<F, Fut>(&self, f: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let hook: ShutdownHook = Box::new(move || Box::pin(f()));
        tokio::spawn({
            let hooks = self.hooks.clone();
            async move {
                hooks.lock().await.push(hook);
            }
        });
    }

    /// Register a component for graceful shutdown
    pub async fn register_component(&self, component: Arc<dyn GracefulShutdown>) {
        let mut components = self.components.write().await;
        components.push(component);
    }

    /// Increment active connection count
    pub fn connection_opened(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement active connection count
    pub fn connection_closed(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get current active connection count
    pub fn active_connections(&self) -> usize {
        self.active_connections.load(Ordering::Relaxed)
    }

    /// Check if shutdown is in progress
    pub fn is_shutting_down(&self) -> bool {
        self.is_shutting_down.load(Ordering::Relaxed)
    }

    /// Perform graceful shutdown
    pub async fn shutdown(&self) -> Result<(), ShutdownError> {
        // Check if already shutting down
        if self
            .is_shutting_down
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(ShutdownError::AlreadyShuttingDown);
        }

        info!("Starting graceful shutdown");

        // Execute shutdown hooks
        self.execute_hooks().await;

        // Shutdown components
        self.shutdown_components().await?;

        // Drain connections
        self.drain_connections().await?;

        info!("Graceful shutdown completed");
        Ok(())
    }

    /// Execute all registered shutdown hooks
    async fn execute_hooks(&self) {
        let hooks = self.hooks.lock().await;
        info!("Executing {} shutdown hooks", hooks.len());

        for (i, hook) in hooks.iter().enumerate() {
            debug!("Executing shutdown hook {}", i + 1);
            hook().await;
        }
    }

    /// Shutdown all registered components
    async fn shutdown_components(&self) -> Result<(), ShutdownError> {
        let components = self.components.read().await;
        info!("Shutting down {} components", components.len());

        let mut errors = Vec::new();

        for component in components.iter() {
            let name = component.name();
            debug!("Shutting down component: {}", name);

            match timeout(self.timeout, component.shutdown()).await {
                Ok(Ok(())) => {
                    info!("Component {} shut down successfully", name);
                }
                Ok(Err(e)) => {
                    warn!("Component {} shutdown failed: {}", name, e);
                    errors.push((name.to_string(), e.to_string()));
                }
                Err(_) => {
                    warn!("Component {} shutdown timed out", name);
                    errors.push((
                        name.to_string(),
                        format!("timed out after {:?}", self.timeout),
                    ));
                }
            }
        }

        if !errors.is_empty() {
            let (component, reason) = errors.into_iter().next().unwrap();
            return Err(ShutdownError::ComponentFailed { component, reason });
        }

        Ok(())
    }

    /// Drain active connections with timeout
    async fn drain_connections(&self) -> Result<(), ShutdownError> {
        let start = std::time::Instant::now();
        let check_interval = Duration::from_millis(100);

        info!(
            "Draining {} active connections",
            self.active_connections()
        );

        while self.active_connections() > 0 {
            if start.elapsed() >= self.timeout {
                warn!(
                    "Connection drain timed out with {} active connections",
                    self.active_connections()
                );
                return Err(ShutdownError::Timeout(self.timeout));
            }

            debug!(
                "Waiting for {} connections to close",
                self.active_connections()
            );
            sleep(check_interval).await;
        }

        info!("All connections drained");
        Ok(())
    }

    /// Force shutdown without waiting
    pub async fn force_shutdown(&self) {
        warn!("Forcing immediate shutdown");
        self.is_shutting_down.store(true, Ordering::SeqCst);
    }
}

/// Connection guard that tracks active connections
pub struct ConnectionGuard {
    coordinator: Arc<ShutdownCoordinator>,
}

impl ConnectionGuard {
    /// Create a new connection guard
    pub fn new(coordinator: Arc<ShutdownCoordinator>) -> Option<Self> {
        if coordinator.is_shutting_down() {
            None
        } else {
            coordinator.connection_opened();
            Some(Self { coordinator })
        }
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.coordinator.connection_closed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    struct TestComponent {
        name: String,
        shutdown_delay: Duration,
        should_fail: bool,
    }

    #[async_trait]
    impl GracefulShutdown for TestComponent {
        async fn shutdown(&self) -> Result<(), ShutdownError> {
            sleep(self.shutdown_delay).await;

            if self.should_fail {
                Err(ShutdownError::ComponentFailed {
                    component: self.name.clone(),
                    reason: "simulated failure".to_string(),
                })
            } else {
                Ok(())
            }
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    #[test]
    fn test_shutdown_signal_new() {
        let signal = ShutdownSignal::new();
        assert!(!signal.is_triggered());
    }

    #[test]
    fn test_shutdown_signal_manual_trigger() {
        let signal = ShutdownSignal::new();
        signal.trigger();
        assert!(signal.is_triggered());
    }

    #[tokio::test]
    async fn test_coordinator_connection_tracking() {
        let coordinator = ShutdownCoordinator::new(Duration::from_secs(10));

        assert_eq!(coordinator.active_connections(), 0);

        coordinator.connection_opened();
        assert_eq!(coordinator.active_connections(), 1);

        coordinator.connection_opened();
        assert_eq!(coordinator.active_connections(), 2);

        coordinator.connection_closed();
        assert_eq!(coordinator.active_connections(), 1);

        coordinator.connection_closed();
        assert_eq!(coordinator.active_connections(), 0);
    }

    #[tokio::test]
    async fn test_connection_guard() {
        let coordinator = Arc::new(ShutdownCoordinator::new(Duration::from_secs(10)));

        assert_eq!(coordinator.active_connections(), 0);

        {
            let _guard = ConnectionGuard::new(coordinator.clone()).unwrap();
            assert_eq!(coordinator.active_connections(), 1);
        }

        assert_eq!(coordinator.active_connections(), 0);
    }

    #[tokio::test]
    async fn test_connection_guard_rejects_during_shutdown() {
        let coordinator = Arc::new(ShutdownCoordinator::new(Duration::from_secs(10)));
        coordinator.is_shutting_down.store(true, Ordering::SeqCst);

        let guard = ConnectionGuard::new(coordinator.clone());
        assert!(guard.is_none());
    }

    #[tokio::test]
    async fn test_shutdown_hooks_execution() {
        let coordinator = ShutdownCoordinator::new(Duration::from_secs(10));
        let counter = Arc::new(AtomicU32::new(0));

        let counter1 = counter.clone();
        coordinator.on_shutdown(move || {
            let c = counter1.clone();
            async move {
                c.fetch_add(1, Ordering::Relaxed);
            }
        });

        let counter2 = counter.clone();
        coordinator.on_shutdown(move || {
            let c = counter2.clone();
            async move {
                c.fetch_add(10, Ordering::Relaxed);
            }
        });

        // Give hooks time to register
        sleep(Duration::from_millis(100)).await;

        coordinator.shutdown().await.unwrap();

        assert_eq!(counter.load(Ordering::Relaxed), 11);
    }

    #[tokio::test]
    async fn test_component_registration_and_shutdown() {
        let coordinator = ShutdownCoordinator::new(Duration::from_secs(10));

        let component = Arc::new(TestComponent {
            name: "test".to_string(),
            shutdown_delay: Duration::from_millis(50),
            should_fail: false,
        });

        coordinator.register_component(component).await;

        let result = coordinator.shutdown().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_component_shutdown_failure() {
        let coordinator = ShutdownCoordinator::new(Duration::from_secs(10));

        let component = Arc::new(TestComponent {
            name: "failing".to_string(),
            shutdown_delay: Duration::from_millis(50),
            should_fail: true,
        });

        coordinator.register_component(component).await;

        let result = coordinator.shutdown().await;
        assert!(matches!(result, Err(ShutdownError::ComponentFailed { .. })));
    }

    #[tokio::test]
    async fn test_component_shutdown_timeout() {
        let coordinator = ShutdownCoordinator::new(Duration::from_millis(100));

        let component = Arc::new(TestComponent {
            name: "slow".to_string(),
            shutdown_delay: Duration::from_secs(10),
            should_fail: false,
        });

        coordinator.register_component(component).await;

        let result = coordinator.shutdown().await;
        assert!(matches!(result, Err(ShutdownError::ComponentFailed { .. })));
    }

    #[tokio::test]
    async fn test_connection_drain() {
        let coordinator = Arc::new(ShutdownCoordinator::new(Duration::from_secs(2)));

        coordinator.connection_opened();
        coordinator.connection_opened();

        let coord_clone = coordinator.clone();
        tokio::spawn(async move {
            sleep(Duration::from_millis(200)).await;
            coord_clone.connection_closed();
            sleep(Duration::from_millis(200)).await;
            coord_clone.connection_closed();
        });

        let result = coordinator.shutdown().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_connection_drain_timeout() {
        let coordinator = Arc::new(ShutdownCoordinator::new(Duration::from_millis(100)));

        // Add connections that won't be closed
        coordinator.connection_opened();
        coordinator.connection_opened();

        let result = coordinator.shutdown().await;
        assert!(matches!(result, Err(ShutdownError::Timeout(_))));
    }

    #[tokio::test]
    async fn test_prevents_double_shutdown() {
        let coordinator = Arc::new(ShutdownCoordinator::new(Duration::from_secs(10)));

        let coord1 = coordinator.clone();
        let handle1 = tokio::spawn(async move { coord1.shutdown().await });

        // Give first shutdown a chance to start
        sleep(Duration::from_millis(50)).await;

        let coord2 = coordinator.clone();
        let handle2 = tokio::spawn(async move { coord2.shutdown().await });

        let result1 = handle1.await.unwrap();
        let result2 = handle2.await.unwrap();

        // One should succeed, one should fail
        assert!(
            (result1.is_ok() && matches!(result2, Err(ShutdownError::AlreadyShuttingDown)))
                || (result2.is_ok() && matches!(result1, Err(ShutdownError::AlreadyShuttingDown)))
        );
    }

    #[tokio::test]
    async fn test_force_shutdown() {
        let coordinator = ShutdownCoordinator::new(Duration::from_secs(10));

        assert!(!coordinator.is_shutting_down());

        coordinator.force_shutdown().await;

        assert!(coordinator.is_shutting_down());
    }
}
