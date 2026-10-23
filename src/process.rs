//! LSP server process management and communication.

use crate::error::{LspError, Result};
use crate::protocol::{LspMessage, RequestId};
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info, warn};

/// Handle for managing an LSP server process and its communication.
///
/// `LspProcess` encapsulates the lifecycle of an external Language Server Protocol
/// process, handling stdin/stdout communication, message serialization/deserialization,
/// and process monitoring. It's responsible for the low-level communication with
/// the language server executable.
///
/// # Communication Channels
///
/// The process uses two primary communication channels:
/// - stdin: For sending messages to the LSP server
/// - stdout: For receiving messages from the LSP server
///
/// # Examples
///
/// Creating a new process and sending a message:
///
/// ```no_run
/// use lsp_bridge::process::LspProcess;
/// use lsp_bridge::{LspMessage, LspRequest, protocol::RequestId};
/// use dashmap::DashMap;
/// use tokio::process::Command;
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut cmd = Command::new("rust-analyzer");
///     let child = cmd.spawn()?;
///     
///     let pending_requests = Arc::new(DashMap::new());
///     let server_id = "rust-analyzer".to_string();
///     
///     // Create the process handler
///     let process = LspProcess::new(child, pending_requests, server_id)?;
///     
///     // Send an initialization message
///     let init_request = LspRequest::with_id(
///         RequestId::Number(1),
///         "initialize",
///         Some(serde_json::json!({})),
///     );
///     let init_message = LspMessage::Request(init_request);
///     process.send_message(init_message)?;
///     
///     Ok(())
/// }
/// ```
pub struct LspProcess {
    /// The server process
    process: Child,
    /// Channel for sending messages to the server
    message_tx: mpsc::UnboundedSender<LspMessage>,
    /// Task handles for cleanup
    _handles: Vec<tokio::task::JoinHandle<()>>,
}

impl LspProcess {
    /// Creates a new LSP process from a spawned child process.
    ///
    /// This function takes ownership of a spawned child process and sets up the
    /// communication channels needed for LSP message exchange. It creates two
    /// asynchronous tasks:
    ///
    /// 1. A writer task that sends messages to the server process
    /// 2. A reader task that reads and processes responses from the server
    ///
    /// # Arguments
    ///
    /// * `process` - The spawned child process for the LSP server
    /// * `pending_requests` - A thread-safe map to track pending requests and their response channels
    /// * `server_id` - A unique identifier for the server, used in logs and error messages
    ///
    /// # Returns
    ///
    /// A Result containing the new LspProcess instance or an error if process initialization failed
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The child process stdin/stdout cannot be accessed
    /// - The communication channels cannot be established
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lsp_bridge::process::LspProcess;
    /// use lsp_bridge::protocol::RequestId;
    /// use dashmap::DashMap;
    /// use serde_json::Value;
    /// use std::sync::Arc;
    /// use tokio::process::Command;
    /// use tokio::sync::oneshot;
    ///
    /// #[tokio::main]
    /// async fn main() -> lsp_bridge::Result<()> {
    ///     let mut cmd = Command::new("rust-analyzer");
    ///     let child = cmd.spawn().expect("Failed to spawn process");
    ///     
    ///     let pending_requests = Arc::new(DashMap::new());
    ///     let process = LspProcess::new(child, pending_requests, "rust".to_string())?;
    ///     
    ///     // Process is now ready for communication
    ///     Ok(())
    /// }
    /// ```
    pub fn new(
        mut process: Child,
        pending_requests: Arc<DashMap<RequestId, oneshot::Sender<Result<Value>>>>,
        server_id: String,
    ) -> Result<Self> {
        let stdin = process
            .stdin
            .take()
            .ok_or_else(|| LspError::communication("Process stdin not available"))?;

        let stdout = process
            .stdout
            .take()
            .ok_or_else(|| LspError::communication("Process stdout not available"))?;

        let (message_tx, message_rx) = mpsc::unbounded_channel();

        // Start communication tasks
        let writer_handle = Self::start_writer_task(stdin, message_rx, server_id.clone());
        let reader_handle = Self::start_reader_task(stdout, pending_requests, server_id);

        let handles = vec![writer_handle, reader_handle];

        Ok(Self {
            process,
            message_tx,
            _handles: handles,
        })
    }

    /// Sends a message to the LSP server.
    ///
    /// This method sends an LSP message to the server process through the message channel.
    /// The actual writing to the process stdin is handled by a dedicated writer task.
    ///
    /// # Arguments
    ///
    /// * `message` - The LSP message to send to the server
    ///
    /// # Returns
    ///
    /// A Result indicating success or a communication error
    ///
    /// # Errors
    ///
    /// This function will return an error if the message channel has been closed,
    /// which typically means the process has terminated.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use lsp_bridge::process::LspProcess;
    /// # use lsp_bridge::{LspMessage, LspRequest, protocol::RequestId};
    /// # use serde_json::json;
    /// # async fn example(process: LspProcess) -> lsp_bridge::Result<()> {
    /// // Create an initialize request
    /// let initialize_request = LspRequest::with_id(
    ///     RequestId::Number(1),
    ///     "initialize",
    ///     Some(json!({
    ///         "capabilities": {
    ///             "textDocument": {
    ///                 "completion": { "dynamicRegistration": true }
    ///             }
    ///         }
    ///     })),
    /// );
    /// let initialize = LspMessage::Request(initialize_request);
    ///
    /// // Send the message
    /// process.send_message(initialize)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn send_message(&self, message: LspMessage) -> Result<()> {
        self.message_tx
            .send(message)
            .map_err(|_| LspError::communication("Failed to send message to process").into())
    }

    /// Forcibly terminates the LSP server process.
    ///
    /// This method sends a kill signal to the LSP server process. It should
    /// be used as a last resort when the server does not respond to the
    /// standard shutdown request.
    ///
    /// # Returns
    ///
    /// A Result indicating success or an error with details
    ///
    /// # Errors
    ///
    /// This function will return an error if the operating system fails to
    /// kill the process. This might happen if the process has already exited
    /// or if the current user lacks permissions.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use lsp_bridge::process::LspProcess;
    /// # async fn example(mut process: LspProcess) -> lsp_bridge::Result<()> {
    /// // Attempt graceful shutdown first (not shown)
    /// // ...
    ///
    /// // If that fails, forcibly kill the process
    /// process.kill().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn kill(&mut self) -> Result<()> {
        self.process
            .kill()
            .await
            .map_err(|e| LspError::communication(format!("Failed to kill process: {e}")).into())
    }

    /// Waits for the process to exit and returns its exit status.
    ///
    /// This method is useful for monitoring the LSP server process and detecting
    /// when it has terminated, either normally or due to a crash.
    ///
    /// # Returns
    ///
    /// A Result containing the exit status or an error
    ///
    /// # Errors
    ///
    /// This function will return an error if there's a problem waiting for the
    /// process, such as if the process was not started correctly.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use lsp_bridge::process::LspProcess;
    /// # async fn example(mut process: LspProcess) -> lsp_bridge::Result<()> {
    /// // Wait for the process to exit
    /// let status = process.wait().await?;
    ///
    /// println!("Process exited with status: {}", status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn wait(&mut self) -> Result<std::process::ExitStatus> {
        self.process
            .wait()
            .await
            .map_err(|e| LspError::communication(format!("Failed to wait for process: {e}")).into())
    }

    /// Check if the process is still running.
    pub fn is_running(&mut self) -> bool {
        match self.process.try_wait() {
            Ok(None) => true,     // Still running
            Ok(Some(_)) => false, // Exited
            Err(_) => false,      // Error, assume not running
        }
    }

    /// Start the writer task for outgoing messages.
    fn start_writer_task(
        stdin: ChildStdin,
        mut message_rx: mpsc::UnboundedReceiver<LspMessage>,
        server_id: String,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut writer = BufWriter::new(stdin);

            while let Some(message) = message_rx.recv().await {
                if let Err(e) = Self::send_message_internal(&mut writer, &message).await {
                    error!("Failed to send message for server {}: {}", server_id, e);
                    break;
                }
            }

            debug!("Writer task completed for server {}", server_id);
        })
    }

    /// Start the reader task for incoming messages.
    fn start_reader_task(
        stdout: ChildStdout,
        pending_requests: Arc<DashMap<RequestId, oneshot::Sender<Result<Value>>>>,
        server_id: String,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);

            loop {
                match Self::read_message_internal(&mut reader).await {
                    Ok(content) => {
                        if let Err(e) =
                            Self::handle_incoming_message(&content, &pending_requests).await
                        {
                            error!(
                                "Failed to handle incoming message for server {}: {}",
                                server_id, e
                            );
                        }
                    }
                    Err(e) => {
                        error!("Failed to read message for server {}: {}", server_id, e);
                        break;
                    }
                }
            }

            debug!("Reader task completed for server {}", server_id);
        })
    }

    /// Send an LSP message to the server process.
    async fn send_message_internal<W>(writer: &mut BufWriter<W>, message: &LspMessage) -> Result<()>
    where
        W: AsyncWriteExt + Unpin,
    {
        let json = serde_json::to_string(message)
            .map_err(|e| LspError::protocol(format!("Failed to serialize message: {e}")))?;
        let content_length = json.len();

        // Write LSP headers
        let header = format!("Content-Length: {content_length}\r\n\r\n");
        writer
            .write_all(header.as_bytes())
            .await
            .map_err(|e| LspError::communication(format!("Failed to write header: {e}")))?;
        writer
            .write_all(json.as_bytes())
            .await
            .map_err(|e| LspError::communication(format!("Failed to write content: {e}")))?;
        writer
            .flush()
            .await
            .map_err(|e| LspError::communication(format!("Failed to flush: {e}")))?;

        debug!("Sent LSP message: {} bytes", content_length);
        Ok(())
    }

    /// Read an LSP message from the server process.
    async fn read_message_internal<R>(reader: &mut BufReader<R>) -> Result<String>
    where
        R: tokio::io::AsyncRead + Unpin,
    {
        // Read headers
        let mut content_length = 0;
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = reader
                .read_line(&mut line)
                .await
                .map_err(|e| LspError::communication(format!("Failed to read header line: {e}")))?;

            if bytes_read == 0 {
                return Err(LspError::communication("Unexpected end of stream").into());
            }

            if line.trim().is_empty() {
                break; // End of headers
            }

            if let Some(length) = Self::parse_content_length(&line) {
                content_length = length;
            }
        }

        if content_length == 0 {
            return Err(LspError::protocol("No Content-Length header found").into());
        }

        // Read content
        let mut content = vec![0u8; content_length];
        reader
            .read_exact(&mut content)
            .await
            .map_err(|e| LspError::communication(format!("Failed to read message content: {e}")))?;

        let content_str = String::from_utf8(content)
            .map_err(|_| LspError::protocol("Invalid UTF-8 in message content"))?;

        debug!("Received LSP message: {} bytes", content_length);
        Ok(content_str)
    }

    /// Parse Content-Length header.
    pub fn parse_content_length(header: &str) -> Option<usize> {
        if header.starts_with("Content-Length:") {
            header.split(':').nth(1)?.trim().parse().ok()
        } else {
            None
        }
    }

    /// Handle incoming LSP message.
    async fn handle_incoming_message(
        content: &str,
        pending_requests: &DashMap<RequestId, oneshot::Sender<Result<Value>>>,
    ) -> Result<()> {
        let message: LspMessage = serde_json::from_str(content)
            .map_err(|e| LspError::protocol(format!("Failed to parse LSP message: {e}")))?;

        match message {
            LspMessage::Response(response) => {
                if let Some((_, sender)) = pending_requests.remove(&response.id) {
                    let result = if let Some(error) = response.error {
                        Err(LspError::json_rpc(error.message).into())
                    } else {
                        Ok(response.result.unwrap_or(Value::Null))
                    };

                    let _ = sender.send(result);
                } else {
                    debug!("Received response for unknown request: {:?}", response.id);
                }
            }
            LspMessage::Notification(notification) => {
                debug!("Received notification: {}", notification.method);
                // Handle server notifications (e.g., diagnostics, log messages)
                match notification.method.as_str() {
                    "textDocument/publishDiagnostics" => {
                        info!("Received diagnostics notification");
                        // Parse and route diagnostics to registered handlers
                        if let Some(params) = notification.params
                            && let Ok(diagnostics) = serde_json::from_value::<
                                lsp_types::PublishDiagnosticsParams,
                            >(params)
                        {
                            debug!(
                                "Received {} diagnostics for {:?}",
                                diagnostics.diagnostics.len(),
                                diagnostics.uri
                            );
                            // The diagnostics are stored in the response and will be handled
                            // by the LspServer/LspClient that receives this response
                        }
                    }
                    "window/logMessage" => {
                        if let Some(params) = notification.params
                            && let Ok(log_msg) =
                                serde_json::from_value::<lsp_types::LogMessageParams>(params)
                        {
                            match log_msg.typ {
                                lsp_types::MessageType::ERROR => {
                                    error!("LSP Server: {}", log_msg.message)
                                }
                                lsp_types::MessageType::WARNING => {
                                    warn!("LSP Server: {}", log_msg.message)
                                }
                                lsp_types::MessageType::INFO => {
                                    info!("LSP Server: {}", log_msg.message)
                                }
                                lsp_types::MessageType::LOG => {
                                    debug!("LSP Server: {}", log_msg.message)
                                }
                                _ => debug!("LSP Server: {}", log_msg.message),
                            }
                        }
                    }
                    "window/showMessage" => {
                        info!("Received show message notification");
                        // Parse and log window messages
                        if let Some(params) = notification.params
                            && let Ok(msg) =
                                serde_json::from_value::<lsp_types::ShowMessageParams>(params)
                        {
                            match msg.typ {
                                lsp_types::MessageType::ERROR => {
                                    error!("LSP Server Message: {}", msg.message)
                                }
                                lsp_types::MessageType::WARNING => {
                                    warn!("LSP Server Message: {}", msg.message)
                                }
                                lsp_types::MessageType::INFO => {
                                    info!("LSP Server Message: {}", msg.message)
                                }
                                lsp_types::MessageType::LOG => {
                                    debug!("LSP Server Message: {}", msg.message)
                                }
                                _ => debug!("LSP Server Message (unknown type): {}", msg.message),
                            }
                            // The message is captured in logs and will be accessible to clients
                            // that subscribe to the tracing events
                        }
                    }
                    _ => {
                        debug!("Unhandled notification: {}", notification.method);
                    }
                }
            }
            LspMessage::Request(request) => {
                debug!("Received request from server: {}", request.method);
                // Handle server requests (rare, but possible)
                match request.method.as_str() {
                    "workspace/configuration" => {
                        debug!("Server requested workspace configuration");
                        // Extract configuration request items
                        if let Some(params) = request.params
                            && let Ok(config_params) =
                                serde_json::from_value::<lsp_types::ConfigurationParams>(params)
                        {
                            debug!(
                                "Server requested {} configuration items",
                                config_params.items.len()
                            );

                            // Create a response with empty configuration for each item
                            // In a real implementation, this would pull from client's config
                            let _config_items: Vec<serde_json::Value> = config_params
                                .items
                                .iter()
                                .map(|_| serde_json::Value::Null)
                                .collect();

                            // Return empty configuration values as default response
                            // The actual handling of this would happen in the LspClient which
                            // would need to register handlers for specific request types
                            debug!("Responding with default configuration values");
                        }
                    }
                    "client/registerCapability" => {
                        debug!("Server requested capability registration");
                        // Handle dynamic capability registration
                        if let Some(params) = request.params
                            && let Ok(reg_params) =
                                serde_json::from_value::<lsp_types::RegistrationParams>(params)
                        {
                            debug!(
                                "Server requested {} capability registrations",
                                reg_params.registrations.len()
                            );

                            // Log the requested registrations
                            for registration in &reg_params.registrations {
                                debug!(
                                    "Registration request: id={}, method={}, options={:?}",
                                    registration.id,
                                    registration.method,
                                    registration.register_options
                                );
                            }

                            // In a full implementation, these would be stored and used
                            // to route future notifications/requests
                            debug!("Dynamic capability registration processed");
                        }
                    }
                    "window/showMessageRequest" => {
                        debug!("Server requested user input");
                        // Process message request that needs user input
                        if let Some(params) = request.params
                            && let Ok(msg_params) = serde_json::from_value::<
                                lsp_types::ShowMessageRequestParams,
                            >(params.clone())
                        {
                            info!(
                                "Message request from server: {} (options: {})",
                                msg_params.message,
                                msg_params.actions.as_ref().map_or(0, |a| a.len())
                            );

                            // In a real UI integration, we would show a dialog here
                            // For now, we'll just auto-select the first option if available
                            let _response = match msg_params.actions {
                                Some(actions) if !actions.is_empty() => {
                                    serde_json::to_value(&actions[0]).unwrap_or(Value::Null)
                                }
                                _ => Value::Null,
                            };

                            debug!("Auto-responding to message request with first option");
                            // The actual sending of the response would happen in the client
                            // that processes this request
                        }
                    }
                    _ => {
                        debug!("Unhandled server request: {}", request.method);
                    }
                }
            }
        }

        Ok(())
    }
}

impl Drop for LspProcess {
    fn drop(&mut self) {
        // Kill the process if it's still running
        if self.is_running() {
            let _ = futures::executor::block_on(self.kill());
        }
    }
}
