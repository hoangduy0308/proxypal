use async_trait::async_trait;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::process::{Child, Command};

#[async_trait]
pub trait ProxyProcessManager: Send + Sync {
    /// Start CLIProxyAPI with the given config file
    async fn start(&self, config_path: &Path, port: u16) -> anyhow::Result<u32>;

    /// Stop the running CLIProxyAPI process
    async fn stop(&self) -> anyhow::Result<()>;

    /// Check if CLIProxyAPI is currently running
    fn is_running(&self) -> bool;

    /// Get PID of running process (if any)
    fn pid(&self) -> Option<u32>;

    /// Get uptime in seconds (if running)
    fn uptime_seconds(&self) -> Option<u64>;

    /// For downcasting in tests
    fn as_any(&self) -> &dyn std::any::Any;
}

pub struct LocalProxyProcessManager {
    child: Arc<Mutex<Option<Child>>>,
    started_at: Arc<Mutex<Option<Instant>>>,
    pub binary_path: String,
}

impl LocalProxyProcessManager {
    pub fn new(binary_path: String) -> Self {
        Self {
            child: Arc::new(Mutex::new(None)),
            started_at: Arc::new(Mutex::new(None)),
            binary_path,
        }
    }

    pub fn from_env() -> Self {
        let binary_path =
            std::env::var("CLIPROXY_BINARY_PATH").unwrap_or_else(|_| "cliproxyapi".to_string());
        Self::new(binary_path)
    }
}

#[async_trait]
impl ProxyProcessManager for LocalProxyProcessManager {
    async fn start(&self, config_path: &Path, _port: u16) -> anyhow::Result<u32> {
        if self.is_running() {
            anyhow::bail!("Proxy is already running");
        }

        let child = Command::new(&self.binary_path)
            .arg("--config")
            .arg(config_path)
            .spawn()?;

        let pid = child.id().unwrap_or(0);

        *self.child.lock().unwrap() = Some(child);
        *self.started_at.lock().unwrap() = Some(Instant::now());

        Ok(pid)
    }

    async fn stop(&self) -> anyhow::Result<()> {
        let child_opt = self.child.lock().unwrap().take();
        if let Some(mut child) = child_opt {
            child.kill().await?;
        }
        *self.started_at.lock().unwrap() = None;
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.child.lock().unwrap().is_some()
    }

    fn pid(&self) -> Option<u32> {
        self.child.lock().unwrap().as_ref().and_then(|c| c.id())
    }

    fn uptime_seconds(&self) -> Option<u64> {
        self.started_at
            .lock()
            .unwrap()
            .map(|t| t.elapsed().as_secs())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Default)]
pub struct MockProxyProcessManager {
    pub is_running: Mutex<bool>,
    pub pid: Mutex<Option<u32>>,
    pub start_result: Mutex<Option<anyhow::Result<u32>>>,
    pub call_log: Mutex<Vec<String>>,
}

impl MockProxyProcessManager {
    pub fn set_running(&self, running: bool, pid: u32) {
        *self.is_running.lock().unwrap() = running;
        *self.pid.lock().unwrap() = if running { Some(pid) } else { None };
    }
}

#[async_trait]
impl ProxyProcessManager for MockProxyProcessManager {
    async fn start(&self, config_path: &Path, port: u16) -> anyhow::Result<u32> {
        self.call_log
            .lock()
            .unwrap()
            .push(format!("start:{}:{}", config_path.display(), port));

        if let Some(result) = self.start_result.lock().unwrap().take() {
            return result;
        }

        *self.is_running.lock().unwrap() = true;
        *self.pid.lock().unwrap() = Some(12345);
        Ok(12345)
    }

    async fn stop(&self) -> anyhow::Result<()> {
        self.call_log.lock().unwrap().push("stop".to_string());
        *self.is_running.lock().unwrap() = false;
        *self.pid.lock().unwrap() = None;
        Ok(())
    }

    fn is_running(&self) -> bool {
        *self.is_running.lock().unwrap()
    }

    fn pid(&self) -> Option<u32> {
        *self.pid.lock().unwrap()
    }

    fn uptime_seconds(&self) -> Option<u64> {
        if self.is_running() {
            Some(120)
        } else {
            None
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_mock_process_manager_start() {
        let mock = MockProxyProcessManager::default();
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.yaml");
        std::fs::write(&config_path, "port: 8317").unwrap();

        let pid = mock.start(&config_path, 8317).await.unwrap();

        assert!(mock.is_running());
        assert_eq!(mock.pid(), Some(pid));

        let calls = mock.call_log.lock().unwrap();
        assert!(calls[0].starts_with("start:"));
    }

    #[tokio::test]
    async fn test_mock_process_manager_stop() {
        let mock = MockProxyProcessManager::default();
        mock.set_running(true, 12345);

        mock.stop().await.unwrap();

        assert!(!mock.is_running());
        assert_eq!(mock.pid(), None);
    }

    #[tokio::test]
    async fn test_mock_process_manager_already_running_error() {
        let mock = MockProxyProcessManager::default();
        mock.set_running(true, 12345);

        *mock.start_result.lock().unwrap() = Some(Err(anyhow::anyhow!("Proxy is already running")));

        let result = mock.start(Path::new("/tmp/config.yaml"), 8317).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_local_process_manager_from_env() {
        std::env::remove_var("CLIPROXY_BINARY_PATH");
        let manager = LocalProxyProcessManager::from_env();
        assert_eq!(manager.binary_path, "cliproxyapi");
    }

    #[test]
    fn test_local_process_manager_new() {
        let manager = LocalProxyProcessManager::new("/usr/bin/cliproxyapi".to_string());
        assert_eq!(manager.binary_path, "/usr/bin/cliproxyapi");
        assert!(!manager.is_running());
        assert_eq!(manager.pid(), None);
        assert_eq!(manager.uptime_seconds(), None);
    }

    #[tokio::test]
    async fn test_mock_uptime_when_running() {
        let mock = MockProxyProcessManager::default();
        mock.set_running(true, 12345);
        assert_eq!(mock.uptime_seconds(), Some(120));
    }

    #[tokio::test]
    async fn test_mock_uptime_when_stopped() {
        let mock = MockProxyProcessManager::default();
        assert_eq!(mock.uptime_seconds(), None);
    }
}
