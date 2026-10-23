//! LSP server lifecycle management and communication.

use crate::config::LspServerConfig;
use crate::error::{LspError, Result};
use crate::process::LspProcess;
use crate::protocol::{LspMessage, LspNotification, LspRequest, RequestId, ServerCapabilities};
use async_trait::async_trait;
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;
use tokio::process::Command;
use tokio::sync::{RwLock, oneshot};
use tokio::time::timeout;
use tracing::{info, warn};

/// Server identifier type.
pub type ServerId = String;

/// LSP server state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerState {
    /// Server is not started
    Stopped,
    /// Server is starting up
    Starting,
    /// Server is initializing
    Initializing,
    /// Server is ready to accept requests
    Ready,
    /// Server is shutting down
    ShuttingDown,
    /// Server has crashed or stopped unexpectedly
    Crashed,
}

/// LSP server instance manager.
pub struct LspServer {
    id: ServerId,
    config: LspServerConfig,
    state: Arc<RwLock<ServerState>>,
    capabilities: Arc<RwLock<Option<ServerCapabilities>>>,
    process: Arc<RwLock<Option<LspProcess>>>,
    pending_requests: Arc<DashMap<RequestId, oneshot::Sender<Result<Value>>>>,
    restart_count: Arc<RwLock<u32>>,
    last_restart: Arc<RwLock<Option<Instant>>>,
}

impl LspServer {
    /// Create a new LSP server instance.
    pub fn new<I: Into<String>>(id: I, config: LspServerConfig) -> Self {
        Self {
            id: id.into(),
            config,
            state: Arc::new(RwLock::new(ServerState::Stopped)),
            capabilities: Arc::new(RwLock::new(None)),
            process: Arc::new(RwLock::new(None)),
            pending_requests: Arc::new(DashMap::new()),
            restart_count: Arc::new(RwLock::new(0)),
            last_restart: Arc::new(RwLock::new(None)),
        }
    }

    /// Get the server ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the server configuration.
    pub fn config(&self) -> &LspServerConfig {
        &self.config
    }

    /// Get the current server state.
    pub async fn state(&self) -> ServerState {
        self.state.read().await.clone()
    }

    /// Get the server capabilities.
    pub async fn capabilities(&self) -> Option<ServerCapabilities> {
        self.capabilities.read().await.clone()
    }

    /// Check if the server is ready.
    pub async fn is_ready(&self) -> bool {
        *self.state.read().await == ServerState::Ready
    }

    /// Start the LSP server.
    pub async fn start(&mut self) -> Result<()> {
        // Validate configuration
        self.config.validate()?;

        // Check current state
        let current_state = self.state.read().await.clone();
        if current_state != ServerState::Stopped && current_state != ServerState::Crashed {
            return Err(LspError::server_startup(format!(
                "Server {0} is not in a startable state: {1:?}",
                self.id, current_state
            ))
            .into());
        }

        info!("Starting LSP server: {0}", self.id);
        *self.state.write().await = ServerState::Starting;

        // Start the server process
        let child_process = self.spawn_process().await?;
        let lsp_process = LspProcess::new(
            child_process,
            self.pending_requests.clone(),
            self.id.clone(),
        )?;

        *self.process.write().await = Some(lsp_process);

        // Initialize the server
        self.initialize().await?;

        *self.state.write().await = ServerState::Ready;
        info!("LSP server {0} is ready", self.id);

        Ok(())
    }

    /// Stop the LSP server.
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping LSP server: {0}", self.id);
        *self.state.write().await = ServerState::ShuttingDown;

        // Send shutdown request
        if let Err(e) = self.shutdown().await {
            warn!("Error during server shutdown: {0}", e);
        }

        // Kill the process if it's still running
        if let Some(mut process) = self.process.write().await.take()
            && let Err(e) = process.kill().await
        {
            warn!("Error killing server process: {0}", e);
        }

        *self.state.write().await = ServerState::Stopped;
        *self.capabilities.write().await = None;
        self.pending_requests.clear();

        info!("LSP server {0} stopped", self.id);
        Ok(())
    }

    /// Restart the LSP server.
    pub async fn restart(&mut self) -> Result<()> {
        info!("Restarting LSP server: {0}", self.id);

        // Check restart limits
        let restart_count = *self.restart_count.read().await;
        if restart_count >= self.config.max_restart_attempts {
            return Err(LspError::server_startup(format!(
                "Maximum restart attempts ({0}) exceeded for server {1}",
                self.config.max_restart_attempts, self.id
            ))
            .into());
        }

        // Check restart delay
        if let Some(last_restart) = *self.last_restart.read().await {
            let elapsed = last_restart.elapsed();
            if elapsed < self.config.restart_delay {
                let remaining = self.config.restart_delay - elapsed;
                tokio::time::sleep(remaining).await;
            }
        }

        // Stop and start
        let _ = self.stop().await;
        self.start().await?;

        // Update restart tracking
        *self.restart_count.write().await += 1;
        *self.last_restart.write().await = Some(Instant::now());

        Ok(())
    }

    /// Send a request to the server.
    pub async fn request(&self, method: String, params: Option<Value>) -> Result<Value> {
        let state = self.state.read().await.clone();
        if state != ServerState::Ready && state != ServerState::Initializing {
            return Err(LspError::server_not_found(&self.id).into());
        }

        let request = LspRequest::new(method, params);
        let request_id = request.id.clone();

        let (tx, rx) = oneshot::channel();
        self.pending_requests.insert(request_id.clone(), tx);

        // Send the request
        if let Some(process) = &*self.process.read().await {
            process.send_message(LspMessage::Request(request))?;
        } else {
            return Err(LspError::communication("No process available").into());
        }

        // Wait for response with timeout
        match timeout(self.config.request_timeout, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(LspError::communication("Request channel closed").into()),
            Err(_) => {
                self.pending_requests.remove(&request_id);
                Err(LspError::timeout(self.config.request_timeout.as_millis() as u64).into())
            }
        }
    }

    /// Send a notification to the server.
    pub async fn notify(&self, method: String, params: Option<Value>) -> Result<()> {
        let state = self.state.read().await.clone();
        if state != ServerState::Ready && state != ServerState::Initializing {
            return Err(LspError::server_not_found(&self.id).into());
        }

        let notification = LspNotification::new(method, params);

        if let Some(process) = &*self.process.read().await {
            process.send_message(LspMessage::Notification(notification))?;
        } else {
            return Err(LspError::communication("No process available").into());
        }

        Ok(())
    }

    /// Spawn the server process.
    async fn spawn_process(&self) -> Result<tokio::process::Child> {
        let mut command = Command::new(&self.config.command);
        command.args(&self.config.args);

        if let Some(work_dir) = &self.config.working_directory {
            command.current_dir(work_dir);
        }

        for (key, value) in &self.config.environment {
            command.env(key, value);
        }

        command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        command
            .spawn()
            .map_err(|e| LspError::server_startup(format!("Failed to spawn process: {e}")).into())
    }

    /// Initialize the server.
    async fn initialize(&self) -> Result<()> {
        *self.state.write().await = ServerState::Initializing;

        let init_params = self.build_initialize_params();
        let response = self
            .request("initialize".to_string(), Some(init_params))
            .await?;

        // Parse server capabilities
        if let Some(result) = response.get("capabilities") {
            let capabilities: lsp_types::ServerCapabilities =
                serde_json::from_value(result.clone())?;
            *self.capabilities.write().await = Some(ServerCapabilities::new(capabilities));
        }

        // Send initialized notification
        self.notify("initialized".to_string(), Some(serde_json::json!({})))
            .await?;

        Ok(())
    }

    /// Build initialize parameters.
    fn build_initialize_params(&self) -> Value {
        let mut params = serde_json::json!({
            "processId": std::process::id(),
            "clientInfo": {
                "name": "lsp-bridge",
                "version": env!("CARGO_PKG_VERSION")
            },
            "capabilities": self.config.client_capabilities,
            "trace": self.config.trace,
            "workspaceFolders": self.config.workspace_folders.iter()
                .map(|path| serde_json::json!({
                    "uri": crate::utils::uri::from_path(path),
                    "name": path.file_name().and_then(|n| n.to_str()).unwrap_or("workspace")
                }))
                .collect::<Vec<_>>()
        });

        if let Some(root_path) = &self.config.root_path {
            params["rootPath"] = serde_json::json!(root_path);
            params["rootUri"] = serde_json::json!(crate::utils::uri::from_path(root_path));
        }

        if let Some(init_options) = &self.config.initialization_options {
            params["initializationOptions"] = init_options.clone();
        }

        params
    }

    /// Shutdown the server.
    async fn shutdown(&self) -> Result<()> {
        // Only send shutdown requests if the server is properly initialized
        let state = self.state.read().await.clone();
        if matches!(state, ServerState::Ready) {
            // Send shutdown request with timeout (ignore errors during shutdown)
            let shutdown_future = async {
                let _ = self.request("shutdown".to_string(), None).await;
                let _ = self.notify("exit".to_string(), None).await;
            };

            // Use a short timeout to prevent hanging during shutdown
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), shutdown_future).await;
        }

        Ok(())
    }
}

/// LSP server manager trait for extensibility.
#[async_trait]
pub trait LspServerManager {
    /// Start a server.
    async fn start_server(&mut self, server_id: &str) -> Result<()>;

    /// Stop a server.
    async fn stop_server(&mut self, server_id: &str) -> Result<()>;

    /// Restart a server.
    async fn restart_server(&mut self, server_id: &str) -> Result<()>;

    /// Get server state.
    async fn server_state(&self, server_id: &str) -> Result<ServerState>;

    /// Get server capabilities.
    async fn server_capabilities(&self, server_id: &str) -> Result<ServerCapabilities>;

    /// Send request to server.
    async fn server_request(
        &self,
        server_id: &str,
        method: String,
        params: Option<Value>,
    ) -> Result<Value>;

    /// Send notification to server.
    async fn server_notify(
        &self,
        server_id: &str,
        method: String,
        params: Option<Value>,
    ) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LspServerConfig;
    use std::time::Duration;

    #[tokio::test]
    async fn test_server_creation() {
        let config = LspServerConfig::new()
            .command("echo")
            .startup_timeout(Duration::from_secs(5));

        let server = LspServer::new("test", config);
        assert_eq!(server.id(), "test");
        assert_eq!(server.state().await, ServerState::Stopped);
    }

    /// A directory name with a space: `file://` + `display()` leaves the space unescaped, and on
    /// Windows produces `file://C:\...` (no authority separator), which is not a URI.
    fn spaced_dir() -> std::path::PathBuf {
        std::env::temp_dir().join("lsp bridge root")
    }

    #[test]
    fn initialize_params_root_uri_is_a_valid_file_uri() {
        let dir = spaced_dir();
        let config = LspServerConfig::new().command("echo").root_path(&dir);
        let params = LspServer::new("t", config).build_initialize_params();

        let uri = params["rootUri"].as_str().expect("rootUri");
        let parsed = url::Url::parse(uri).expect("rootUri parses as a URL");
        assert_eq!(parsed.scheme(), "file");
        assert!(!uri.contains(' '), "unescaped space in {uri}");
        assert_eq!(parsed.to_file_path().expect("file path"), dir);
        if cfg!(windows) {
            assert!(
                uri.starts_with("file:///"),
                "Windows URI needs three slashes: {uri}"
            );
        }
    }

    #[test]
    fn initialize_params_workspace_folder_uri_is_a_valid_file_uri() {
        let dir = spaced_dir();
        let config = LspServerConfig::new()
            .command("echo")
            .workspace_folder(&dir);
        let params = LspServer::new("t", config).build_initialize_params();

        let folder = &params["workspaceFolders"][0];
        let uri = folder["uri"].as_str().expect("uri");
        assert!(
            uri.starts_with("file:///") && !uri.contains(' ') && !uri.contains('\\'),
            "{uri}"
        );
        let parsed = url::Url::parse(uri).expect("parses");
        assert_eq!(parsed.to_file_path().expect("file path"), dir);
        assert_eq!(folder["name"], "lsp bridge root");
    }

    #[test]
    fn test_parse_content_length() {
        use crate::process::LspProcess;
        let header = "Content-Length: 123\r\n";
        assert_eq!(LspProcess::parse_content_length(header), Some(123));

        let invalid_header = "Invalid: header\r\n";
        assert_eq!(LspProcess::parse_content_length(invalid_header), None);
    }

    #[tokio::test]
    async fn test_server_state_transitions() {
        let config = LspServerConfig::new().command("echo");
        let server = LspServer::new("test", config);

        assert_eq!(server.state().await, ServerState::Stopped);

        // Note: We can't actually test start() without a real LSP server
        // These would be integration tests
    }
}
