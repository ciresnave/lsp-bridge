//! Configuration types and builders for LSP Bridge.

use crate::error::{LspError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Configuration for an LSP server instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspServerConfig {
    /// Command to execute the LSP server
    pub command: String,

    /// Arguments to pass to the server command
    pub args: Vec<String>,

    /// Working directory for the server process
    pub working_directory: Option<PathBuf>,

    /// Environment variables for the server process
    pub environment: HashMap<String, String>,

    /// Root path/URI for the workspace
    pub root_path: Option<PathBuf>,

    /// Workspace folders
    pub workspace_folders: Vec<PathBuf>,

    /// Initialization options to send to the server
    pub initialization_options: Option<serde_json::Value>,

    /// Client capabilities to advertise
    pub client_capabilities: LspClientCapabilities,

    /// Server startup timeout
    pub startup_timeout: Duration,

    /// Request timeout
    pub request_timeout: Duration,

    /// Whether to enable tracing
    pub trace: TraceLevel,

    /// Custom server settings
    pub settings: HashMap<String, serde_json::Value>,

    /// Maximum number of restart attempts
    pub max_restart_attempts: u32,

    /// Restart delay between attempts
    pub restart_delay: Duration,
}

impl Default for LspServerConfig {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            working_directory: None,
            environment: HashMap::new(),
            root_path: None,
            workspace_folders: Vec::new(),
            initialization_options: None,
            client_capabilities: LspClientCapabilities::default(),
            startup_timeout: Duration::from_secs(30),
            request_timeout: Duration::from_secs(30),
            trace: TraceLevel::Off,
            settings: HashMap::new(),
            max_restart_attempts: 3,
            restart_delay: Duration::from_secs(2),
        }
    }
}

impl LspServerConfig {
    /// Create a new server configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the server command
    pub fn command<S: Into<String>>(mut self, command: S) -> Self {
        self.command = command.into();
        self
    }

    /// Add arguments to the server command
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(|s| s.into()));
        self
    }

    /// Add a single argument to the server command
    pub fn arg<S: Into<String>>(mut self, arg: S) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Set the working directory
    pub fn working_directory<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.working_directory = Some(path.as_ref().to_path_buf());
        self
    }

    /// Add an environment variable
    pub fn env<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.environment.insert(key.into(), value.into());
        self
    }

    /// Set multiple environment variables
    pub fn envs<I, K, V>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (key, value) in vars {
            self.environment.insert(key.into(), value.into());
        }
        self
    }

    /// Set the root path
    pub fn root_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.root_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Add workspace folders
    pub fn workspace_folders<I, P>(mut self, folders: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        self.workspace_folders
            .extend(folders.into_iter().map(|p| p.as_ref().to_path_buf()));
        self
    }

    /// Add a single workspace folder
    pub fn workspace_folder<P: AsRef<Path>>(mut self, folder: P) -> Self {
        self.workspace_folders.push(folder.as_ref().to_path_buf());
        self
    }

    /// Set initialization options
    pub fn initialization_options(mut self, options: serde_json::Value) -> Self {
        self.initialization_options = Some(options);
        self
    }

    /// Set client capabilities
    pub fn client_capabilities(mut self, capabilities: LspClientCapabilities) -> Self {
        self.client_capabilities = capabilities;
        self
    }

    /// Set startup timeout
    pub fn startup_timeout(mut self, timeout: Duration) -> Self {
        self.startup_timeout = timeout;
        self
    }

    /// Set request timeout
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Set trace level
    pub fn trace(mut self, level: TraceLevel) -> Self {
        self.trace = level;
        self
    }

    /// Add a server setting
    pub fn setting<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Serialize,
    {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.settings.insert(key.into(), json_value);
        }
        self
    }

    /// Set maximum restart attempts
    pub fn max_restart_attempts(mut self, attempts: u32) -> Self {
        self.max_restart_attempts = attempts;
        self
    }

    /// Set restart delay
    pub fn restart_delay(mut self, delay: Duration) -> Self {
        self.restart_delay = delay;
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.command.is_empty() {
            return Err(LspError::invalid_configuration("Command cannot be empty").into());
        }

        if self.startup_timeout.is_zero() {
            return Err(LspError::invalid_configuration(
                "Startup timeout must be greater than zero",
            )
            .into());
        }

        if self.request_timeout.is_zero() {
            return Err(LspError::invalid_configuration(
                "Request timeout must be greater than zero",
            )
            .into());
        }

        Ok(())
    }
}

/// Client capabilities configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LspClientCapabilities {
    /// Text document capabilities
    pub text_document: TextDocumentClientCapabilities,

    /// Workspace capabilities
    pub workspace: WorkspaceClientCapabilities,

    /// Window capabilities
    pub window: WindowClientCapabilities,

    /// General capabilities
    pub general: GeneralClientCapabilities,

    /// Experimental capabilities
    pub experimental: Option<serde_json::Value>,
}

/// Text document client capabilities.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TextDocumentClientCapabilities {
    pub synchronization: Option<TextDocumentSyncClientCapabilities>,
    pub completion: Option<CompletionClientCapabilities>,
    pub hover: Option<HoverClientCapabilities>,
    pub signature_help: Option<SignatureHelpClientCapabilities>,
    pub declaration: Option<GotoCapability>,
    pub definition: Option<GotoCapability>,
    pub type_definition: Option<GotoCapability>,
    pub implementation: Option<GotoCapability>,
    pub references: Option<ReferenceClientCapabilities>,
    pub document_highlight: Option<DocumentHighlightClientCapabilities>,
    pub document_symbol: Option<DocumentSymbolClientCapabilities>,
    pub code_action: Option<CodeActionClientCapabilities>,
    pub code_lens: Option<CodeLensClientCapabilities>,
    pub document_link: Option<DocumentLinkClientCapabilities>,
    pub color_provider: Option<DocumentColorClientCapabilities>,
    pub formatting: Option<DocumentFormattingClientCapabilities>,
    pub range_formatting: Option<DocumentRangeFormattingClientCapabilities>,
    pub on_type_formatting: Option<DocumentOnTypeFormattingClientCapabilities>,
    pub rename: Option<RenameClientCapabilities>,
    pub folding_range: Option<FoldingRangeClientCapabilities>,
    pub selection_range: Option<SelectionRangeClientCapabilities>,
    pub publish_diagnostics: Option<PublishDiagnosticsClientCapabilities>,
    pub call_hierarchy: Option<CallHierarchyClientCapabilities>,
}

/// Workspace client capabilities.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceClientCapabilities {
    pub apply_edit: Option<bool>,
    pub workspace_edit: Option<WorkspaceEditClientCapabilities>,
    pub did_change_configuration: Option<DidChangeConfigurationClientCapabilities>,
    pub did_change_watched_files: Option<DidChangeWatchedFilesClientCapabilities>,
    pub symbol: Option<WorkspaceSymbolClientCapabilities>,
    pub execute_command: Option<ExecuteCommandClientCapabilities>,
    pub workspace_folders: Option<bool>,
    pub configuration: Option<bool>,
}

/// Window client capabilities.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowClientCapabilities {
    pub work_done_progress: Option<bool>,
    pub show_message: Option<ShowMessageRequestClientCapabilities>,
    pub show_document: Option<ShowDocumentClientCapabilities>,
}

/// General client capabilities.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeneralClientCapabilities {
    pub regular_expressions: Option<RegularExpressionsClientCapabilities>,
    pub markdown: Option<MarkdownClientCapabilities>,
}

/// Trace level for LSP communication.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum TraceLevel {
    #[serde(rename = "off")]
    #[default]
    Off,
    #[serde(rename = "messages")]
    Messages,
    #[serde(rename = "verbose")]
    Verbose,
}

// Placeholder types for client capabilities - these would normally be imported from lsp-types
// but we're defining minimal versions here for the example

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TextDocumentSyncClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompletionClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HoverClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SignatureHelpClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GotoCapability;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReferenceClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentHighlightClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentSymbolClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodeActionClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodeLensClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentLinkClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentColorClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentFormattingClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentRangeFormattingClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentOnTypeFormattingClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RenameClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FoldingRangeClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SelectionRangeClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PublishDiagnosticsClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CallHierarchyClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceEditClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DidChangeConfigurationClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DidChangeWatchedFilesClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceSymbolClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecuteCommandClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShowMessageRequestClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShowDocumentClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegularExpressionsClientCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarkdownClientCapabilities;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_builder() {
        let config = LspServerConfig::new()
            .command("rust-analyzer")
            .arg("--help")
            .root_path("/tmp/project")
            .workspace_folder("/tmp/project/src")
            .startup_timeout(Duration::from_secs(60));

        assert_eq!(config.command, "rust-analyzer");
        assert_eq!(config.args, vec!["--help"]);
        assert_eq!(config.startup_timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_config_validation() {
        let valid_config = LspServerConfig::new().command("test-server");
        assert!(valid_config.validate().is_ok());

        let invalid_config = LspServerConfig::new(); // No command
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_client_capabilities_default() {
        let capabilities = LspClientCapabilities::default();
        assert!(capabilities.experimental.is_none());
    }
}
