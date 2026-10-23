//! Main LSP Bridge interface coordinating client-server communication.

use crate::client::LspClient;
use crate::config::LspServerConfig;
use crate::error::{LspError, Result};
use crate::protocol::{DocumentState, ServerCapabilities};
use crate::server::{LspServer, LspServerManager, ServerId, ServerState};
use async_trait::async_trait;
use lsp_types::{self, notification::Notification, request::Request, *};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Main LSP Bridge interface that coordinates all LSP operations.
///
/// This is the primary entry point for using the LSP Bridge library.
/// It manages multiple LSP servers, handles document synchronization,
/// and provides a unified interface for LSP operations.
pub struct LspBridge {
    client: Arc<LspClient>,
    server_configs: Arc<RwLock<HashMap<ServerId, LspServerConfig>>>,
}

impl LspBridge {
    /// Create a new LSP Bridge instance.
    pub fn new() -> Self {
        Self {
            client: Arc::new(LspClient::new()),
            server_configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new LSP server with the bridge.
    ///
    /// # Arguments
    ///
    /// * `server_id` - Unique identifier for the server
    /// * `config` - Configuration for the server
    ///
    /// # Returns
    ///
    /// The server ID that can be used for subsequent operations.
    pub async fn register_server<I: Into<String>>(
        &mut self,
        server_id: I,
        config: LspServerConfig,
    ) -> Result<ServerId> {
        let server_id: String = server_id.into();

        // Validate configuration
        config.validate()?;

        // Store configuration
        // Explicitly call String::clone for rust-analyzer
        let server_id_for_config: String = String::clone(&server_id);
        self.server_configs
            .write()
            .await
            .insert(server_id_for_config, config.clone());

        // Create server instance - explicit clone for rust-analyzer
        let server_id_for_server: String = String::clone(&server_id);
        let server = LspServer::new(server_id_for_server, config);

        // Register with client - explicit clone for rust-analyzer
        let server_id_for_client: String = String::clone(&server_id);
        self.client
            .register_server(server_id_for_client, server)
            .await?;

        info!("Registered LSP server: {0}", server_id);
        Ok(server_id)
    }

    /// Unregister an LSP server from the bridge.
    pub async fn unregister_server(&mut self, server_id: &str) -> Result<()> {
        // Remove configuration
        self.server_configs.write().await.remove(server_id);

        // Unregister from client
        self.client.unregister_server(server_id).await?;

        info!("Unregistered LSP server: {0}", server_id);
        Ok(())
    }

    /// Start an LSP server.
    pub async fn start_server(&mut self, server_id: &str) -> Result<()> {
        if let Some(server) = self.client.get_server(server_id) {
            server.write().await.start().await?;
            info!("Started LSP server: {0}", server_id);
            Ok(())
        } else {
            Err(LspError::server_not_found(server_id).into())
        }
    }

    /// Stop an LSP server.
    pub async fn stop_server(&mut self, server_id: &str) -> Result<()> {
        if let Some(server) = self.client.get_server(server_id) {
            server.write().await.stop().await?;
            info!("Stopped LSP server: {0}", server_id);
            Ok(())
        } else {
            Err(LspError::server_not_found(server_id).into())
        }
    }

    /// Restart an LSP server.
    pub async fn restart_server(&mut self, server_id: &str) -> Result<()> {
        if let Some(server) = self.client.get_server(server_id) {
            server.write().await.restart().await?;
            info!("Restarted LSP server: {0}", server_id);
            Ok(())
        } else {
            Err(LspError::server_not_found(server_id).into())
        }
    }

    /// Get the state of an LSP server.
    pub async fn server_state(&self, server_id: &str) -> Result<ServerState> {
        if let Some(server) = self.client.get_server(server_id) {
            Ok(server.read().await.state().await)
        } else {
            Err(LspError::server_not_found(server_id).into())
        }
    }

    /// Get the capabilities of an LSP server.
    pub async fn server_capabilities(&self, server_id: &str) -> Result<ServerCapabilities> {
        if let Some(server) = self.client.get_server(server_id) {
            if let Some(capabilities) = server.read().await.capabilities().await {
                Ok(capabilities)
            } else {
                Err(LspError::custom("Server capabilities not available").into())
            }
        } else {
            Err(LspError::server_not_found(server_id).into())
        }
    }

    /// Set the active server for operations that don't specify a server.
    pub async fn set_active_server(&self, server_id: Option<ServerId>) -> Result<()> {
        self.client.set_active_server(server_id).await
    }

    /// Get the active server ID.
    pub async fn active_server(&self) -> Option<ServerId> {
        self.client.active_server().await
    }

    /// Wait for a server to become ready.
    pub async fn wait_server_ready(&self, server_id: &str) -> Result<()> {
        self.client.wait_server_ready(server_id, 30).await
    }

    /// Open a document in an LSP server.
    pub async fn open_document(&self, server_id: &str, uri: &str, content: &str) -> Result<()> {
        // Determine language ID from URI
        let language_id = self.detect_language_id(uri);

        self.client
            .open_document(server_id, uri, content, &language_id)
            .await
    }

    /// Close a document in an LSP server.
    pub async fn close_document(&self, server_id: &str, uri: &str) -> Result<()> {
        self.client.close_document(server_id, uri).await
    }

    /// Update document content.
    pub async fn update_document(&self, server_id: &str, uri: &str, content: &str) -> Result<()> {
        self.client.update_document(server_id, uri, content).await
    }

    /// Request completions for a document position.
    pub async fn get_completions(
        &self,
        server_id: &str,
        uri: &str,
        position: Position,
    ) -> Result<Vec<CompletionItem>> {
        self.client.get_completions(server_id, uri, position).await
    }

    /// Get hover information for a document position.
    pub async fn get_hover(
        &self,
        server_id: &str,
        uri: &str,
        position: Position,
    ) -> Result<Option<Hover>> {
        self.client.get_hover(server_id, uri, position).await
    }

    /// Go to definition for a document position.
    pub async fn go_to_definition(
        &self,
        server_id: &str,
        uri: &str,
        position: Position,
    ) -> Result<Option<Location>> {
        let response = self
            .client
            .go_to_definition(server_id, uri, position)
            .await?;

        match response {
            Some(GotoDefinitionResponse::Scalar(location)) => Ok(Some(location)),
            Some(GotoDefinitionResponse::Array(locations)) => Ok(locations.into_iter().next()),
            Some(GotoDefinitionResponse::Link(links)) => {
                Ok(links.into_iter().next().map(|link| Location {
                    uri: link.target_uri,
                    range: link.target_selection_range,
                }))
            }
            None => Ok(None),
        }
    }

    /// Find references for a document position.
    pub async fn find_references(
        &self,
        server_id: &str,
        uri: &str,
        position: Position,
    ) -> Result<Vec<Location>> {
        self.client
            .find_references(server_id, uri, position, false)
            .await
    }

    /// Format a document.
    pub async fn format_document(&self, server_id: &str, uri: &str) -> Result<Vec<TextEdit>> {
        let options = FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            properties: HashMap::new(),
            trim_trailing_whitespace: Some(true),
            insert_final_newline: Some(true),
            trim_final_newlines: Some(true),
        };

        self.client.format_document(server_id, uri, options).await
    }

    /// Get diagnostics for a document.
    pub fn get_diagnostics(&self, server_id: &str, uri: &str) -> Result<Vec<Diagnostic>> {
        // Verify server exists
        if self.client.get_server(server_id).is_none() {
            return Err(LspError::server_not_found(server_id).into());
        }

        Ok(self.client.get_diagnostics(uri))
    }

    /// Send a custom request to a server.
    pub async fn request<R: Request>(&self, server_id: &str, params: R::Params) -> Result<R::Result>
    where
        R::Params: serde::Serialize,
        R::Result: serde::de::DeserializeOwned,
    {
        if let Some(server) = self.client.get_server(server_id) {
            let response = server
                .read()
                .await
                .request(R::METHOD.to_string(), Some(serde_json::to_value(params)?))
                .await?;

            let result: R::Result = serde_json::from_value(response)?;
            Ok(result)
        } else {
            Err(LspError::server_not_found(server_id).into())
        }
    }

    /// Send a custom notification to a server.
    pub async fn notify<N: Notification>(&self, server_id: &str, params: N::Params) -> Result<()>
    where
        N::Params: serde::Serialize,
    {
        if let Some(server) = self.client.get_server(server_id) {
            server
                .read()
                .await
                .notify(N::METHOD.to_string(), Some(serde_json::to_value(params)?))
                .await?;
            Ok(())
        } else {
            Err(LspError::server_not_found(server_id).into())
        }
    }

    /// Get document state.
    pub fn get_document_state(&self, uri: &str) -> Option<DocumentState> {
        self.client.get_document_state(uri)
    }

    /// List all open documents.
    pub fn list_documents(&self) -> Vec<String> {
        self.client.list_documents()
    }

    /// Get the number of registered servers (for testing/debugging).
    pub fn server_count(&self) -> usize {
        self.client.server_count()
    }

    /// Check if a server is registered (for testing/debugging).
    pub fn has_server(&self, server_id: &str) -> bool {
        self.client.get_server(server_id).is_some()
    }

    /// List all registered server IDs (for testing/debugging).
    pub fn list_servers(&self) -> Vec<String> {
        self.client.list_servers()
    }

    /// Get type definition for a symbol.
    pub async fn get_type_definition(
        &self,
        server_id: &str,
        uri: &str,
        position: Position,
    ) -> Result<Option<Location>> {
        let response = self
            .client
            .get_type_definition(server_id, uri, position)
            .await?;

        match response {
            Some(GotoDefinitionResponse::Scalar(location)) => Ok(Some(location)),
            Some(GotoDefinitionResponse::Array(locations)) => Ok(locations.into_iter().next()),
            Some(GotoDefinitionResponse::Link(links)) => {
                Ok(links.into_iter().next().map(|link| Location {
                    uri: link.target_uri,
                    range: link.target_selection_range,
                }))
            }
            None => Ok(None),
        }
    }

    /// Get implementation for a symbol.
    pub async fn get_implementation(
        &self,
        server_id: &str,
        uri: &str,
        position: Position,
    ) -> Result<Option<Location>> {
        let response = self
            .client
            .get_implementation(server_id, uri, position)
            .await?;

        match response {
            Some(GotoDefinitionResponse::Scalar(location)) => Ok(Some(location)),
            Some(GotoDefinitionResponse::Array(locations)) => Ok(locations.into_iter().next()),
            Some(GotoDefinitionResponse::Link(links)) => {
                Ok(links.into_iter().next().map(|link| Location {
                    uri: link.target_uri,
                    range: link.target_selection_range,
                }))
            }
            None => Ok(None),
        }
    }

    /// Get document highlights for a position.
    pub async fn get_document_highlights(
        &self,
        server_id: &str,
        uri: &str,
        position: Position,
    ) -> Result<Vec<DocumentHighlight>> {
        self.client
            .get_document_highlights(server_id, uri, position)
            .await
            .map(|highlights| highlights.unwrap_or_default())
    }

    /// Get code lens for a document.
    pub async fn get_code_lens(&self, server_id: &str, uri: &str) -> Result<Vec<CodeLens>> {
        self.client
            .get_code_lens(server_id, uri)
            .await
            .map(|lens| lens.unwrap_or_default())
    }

    /// Resolve a code lens.
    pub async fn resolve_code_lens(
        &self,
        server_id: &str,
        code_lens: CodeLens,
    ) -> Result<CodeLens> {
        self.client.resolve_code_lens(server_id, code_lens).await
    }

    /// Get document links.
    pub async fn get_document_links(
        &self,
        server_id: &str,
        uri: &str,
    ) -> Result<Vec<DocumentLink>> {
        self.client
            .get_document_links(server_id, uri)
            .await
            .map(|links| links.unwrap_or_default())
    }

    /// Resolve a document link.
    pub async fn resolve_document_link(
        &self,
        server_id: &str,
        link: DocumentLink,
    ) -> Result<DocumentLink> {
        self.client.resolve_document_link(server_id, link).await
    }

    /// Get document colors.
    pub async fn get_document_colors(
        &self,
        server_id: &str,
        uri: &str,
    ) -> Result<Vec<ColorInformation>> {
        self.client.get_document_colors(server_id, uri).await
    }

    /// Get color presentations.
    pub async fn get_color_presentations(
        &self,
        server_id: &str,
        uri: &str,
        color: Color,
        range: Range,
    ) -> Result<Vec<ColorPresentation>> {
        self.client
            .get_color_presentations(server_id, uri, color, range)
            .await
    }

    /// Format document range.
    pub async fn format_document_range(
        &self,
        server_id: &str,
        uri: &str,
        range: Range,
    ) -> Result<Vec<TextEdit>> {
        let options = FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            properties: HashMap::new(),
            trim_trailing_whitespace: Some(true),
            insert_final_newline: Some(true),
            trim_final_newlines: Some(true),
        };

        self.client
            .format_document_range(server_id, uri, range, options)
            .await
    }

    /// Format document on type.
    pub async fn format_document_on_type(
        &self,
        server_id: &str,
        uri: &str,
        position: Position,
        ch: String,
    ) -> Result<Vec<TextEdit>> {
        let options = FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            properties: HashMap::new(),
            trim_trailing_whitespace: Some(true),
            insert_final_newline: Some(true),
            trim_final_newlines: Some(true),
        };

        self.client
            .format_document_on_type(server_id, uri, position, ch, options)
            .await
    }

    /// Get folding ranges.
    pub async fn get_folding_ranges(
        &self,
        server_id: &str,
        uri: &str,
    ) -> Result<Vec<FoldingRange>> {
        self.client
            .get_folding_ranges(server_id, uri)
            .await
            .map(|ranges| ranges.unwrap_or_default())
    }

    /// Get selection ranges.
    pub async fn get_selection_ranges(
        &self,
        server_id: &str,
        uri: &str,
        positions: Vec<Position>,
    ) -> Result<Vec<SelectionRange>> {
        self.client
            .get_selection_ranges(server_id, uri, positions)
            .await
            .map(|ranges| ranges.unwrap_or_default())
    }

    /// Execute a command.
    pub async fn execute_command(
        &self,
        server_id: &str,
        command: String,
        arguments: Option<Vec<serde_json::Value>>,
    ) -> Result<Option<serde_json::Value>> {
        self.client
            .execute_command(server_id, command, arguments)
            .await
    }

    /// Prepare call hierarchy.
    pub async fn prepare_call_hierarchy(
        &self,
        server_id: &str,
        uri: &str,
        position: Position,
    ) -> Result<Vec<CallHierarchyItem>> {
        self.client
            .prepare_call_hierarchy(server_id, uri, position)
            .await
            .map(|items| items.unwrap_or_default())
    }

    /// Get incoming calls.
    pub async fn get_incoming_calls(
        &self,
        server_id: &str,
        item: CallHierarchyItem,
    ) -> Result<Vec<CallHierarchyIncomingCall>> {
        self.client
            .get_incoming_calls(server_id, item)
            .await
            .map(|calls| calls.unwrap_or_default())
    }

    /// Get outgoing calls.
    pub async fn get_outgoing_calls(
        &self,
        server_id: &str,
        item: CallHierarchyItem,
    ) -> Result<Vec<CallHierarchyOutgoingCall>> {
        self.client
            .get_outgoing_calls(server_id, item)
            .await
            .map(|calls| calls.unwrap_or_default())
    }

    /// Get semantic tokens (full).
    pub async fn get_semantic_tokens_full(
        &self,
        server_id: &str,
        uri: &str,
    ) -> Result<Option<SemanticTokens>> {
        self.client.get_semantic_tokens_full(server_id, uri).await
    }

    /// Get semantic tokens (delta).
    pub async fn get_semantic_tokens_delta(
        &self,
        server_id: &str,
        uri: &str,
        previous_result_id: String,
    ) -> Result<Option<SemanticTokensResult>> {
        self.client
            .get_semantic_tokens_delta(server_id, uri, previous_result_id)
            .await
    }

    /// Get semantic tokens (range).
    pub async fn get_semantic_tokens_range(
        &self,
        server_id: &str,
        uri: &str,
        range: Range,
    ) -> Result<Option<SemanticTokens>> {
        self.client
            .get_semantic_tokens_range(server_id, uri, range)
            .await
    }

    /// Get inlay hints.
    pub async fn get_inlay_hints(
        &self,
        server_id: &str,
        uri: &str,
        range: Range,
    ) -> Result<Vec<InlayHint>> {
        self.client
            .get_inlay_hints(server_id, uri, range)
            .await
            .map(|hints| hints.unwrap_or_default())
    }

    /// Resolve inlay hint.
    pub async fn resolve_inlay_hint(&self, server_id: &str, hint: InlayHint) -> Result<InlayHint> {
        self.client.resolve_inlay_hint(server_id, hint).await
    }

    /// Get inline values.
    pub async fn get_inline_values(
        &self,
        server_id: &str,
        uri: &str,
        range: Range,
        context: InlineValueContext,
    ) -> Result<Vec<InlineValue>> {
        self.client
            .get_inline_values(server_id, uri, range, context)
            .await
            .map(|values| values.unwrap_or_default())
    }

    /// Get monikers.
    pub async fn get_monikers(
        &self,
        server_id: &str,
        uri: &str,
        position: Position,
    ) -> Result<Vec<Moniker>> {
        self.client
            .get_monikers(server_id, uri, position)
            .await
            .map(|monikers| monikers.unwrap_or_default())
    }

    /// Resolve completion item.
    pub async fn resolve_completion_item(
        &self,
        server_id: &str,
        item: CompletionItem,
    ) -> Result<CompletionItem> {
        self.client.resolve_completion_item(server_id, item).await
    }

    /// Shutdown all servers and cleanup.
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down LSP Bridge");

        let servers = self.list_servers();
        for server_id in servers {
            if let Err(e) = self.stop_server(&server_id).await {
                warn!("Error stopping server {0}: {1}", server_id, e);
            }
        }

        self.server_configs.write().await.clear();

        info!("LSP Bridge shutdown complete");
        Ok(())
    }

    /// Detect language ID from file URI.
    fn detect_language_id(&self, uri: &str) -> String {
        // Extract file extension and map to language ID
        if let Some(extension) = uri.split('.').next_back() {
            match extension {
                "rs" => "rust",
                "py" => "python",
                "js" => "javascript",
                "ts" => "typescript",
                "json" => "json",
                "yaml" | "yml" => "yaml",
                "toml" => "toml",
                "md" => "markdown",
                "html" => "html",
                "css" => "css",
                "cpp" | "cc" | "cxx" => "cpp",
                "c" => "c",
                "h" | "hpp" => "c",
                "java" => "java",
                "go" => "go",
                "php" => "php",
                "rb" => "ruby",
                "swift" => "swift",
                "kt" => "kotlin",
                "cs" => "csharp",
                _ => "plaintext",
            }
            .to_string()
        } else {
            "plaintext".to_string()
        }
    }

    /// Get signature help at a position.
    pub async fn get_signature_help(
        &self,
        server_id: &str,
        uri: &str,
        position: Position,
    ) -> Result<Option<SignatureHelp>> {
        self.client
            .get_signature_help(server_id, uri, position)
            .await
    }

    /// Get document symbols.
    pub async fn get_document_symbols(
        &self,
        server_id: &str,
        uri: &str,
    ) -> Result<Vec<DocumentSymbol>> {
        self.client.get_document_symbols(server_id, uri).await
    }

    /// Get workspace symbols.
    pub async fn get_workspace_symbols(
        &self,
        server_id: &str,
        query: &str,
    ) -> Result<Vec<SymbolInformation>> {
        self.client.get_workspace_symbols(server_id, query).await
    }

    /// Get code actions for a range.
    pub async fn get_code_actions(
        &self,
        server_id: &str,
        uri: &str,
        range: Range,
        context: CodeActionContext,
    ) -> Result<Vec<CodeActionOrCommand>> {
        self.client
            .get_code_actions(server_id, uri, range, context)
            .await
    }

    /// Rename a symbol.
    pub async fn rename_symbol(
        &self,
        server_id: &str,
        uri: &str,
        position: Position,
        new_name: &str,
    ) -> Result<Option<WorkspaceEdit>> {
        self.client
            .rename_symbol(server_id, uri, position, new_name)
            .await
    }

    /// Apply text edits to content and return the result.
    pub fn apply_text_edits(&self, content: &str, edits: &[TextEdit]) -> Result<String> {
        crate::utils::document::apply_edits(content, edits)
    }
}

#[async_trait]
impl LspServerManager for LspBridge {
    async fn start_server(&mut self, server_id: &str) -> Result<()> {
        self.start_server(server_id).await
    }

    async fn stop_server(&mut self, server_id: &str) -> Result<()> {
        self.stop_server(server_id).await
    }

    async fn restart_server(&mut self, server_id: &str) -> Result<()> {
        self.restart_server(server_id).await
    }

    async fn server_state(&self, server_id: &str) -> Result<ServerState> {
        self.server_state(server_id).await
    }

    async fn server_capabilities(&self, server_id: &str) -> Result<ServerCapabilities> {
        self.server_capabilities(server_id).await
    }

    async fn server_request(
        &self,
        server_id: &str,
        method: String,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        if let Some(server) = self.client.get_server(server_id) {
            server.read().await.request(method, params).await
        } else {
            Err(LspError::server_not_found(server_id).into())
        }
    }

    async fn server_notify(
        &self,
        server_id: &str,
        method: String,
        params: Option<serde_json::Value>,
    ) -> Result<()> {
        if let Some(server) = self.client.get_server(server_id) {
            server.read().await.notify(method, params).await
        } else {
            Err(LspError::server_not_found(server_id).into())
        }
    }
}

impl Default for LspBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_bridge_creation() {
        let bridge = LspBridge::new();
        assert_eq!(bridge.server_count(), 0);
    }

    #[tokio::test]
    async fn test_server_registration() {
        let mut bridge = LspBridge::new();
        let config = LspServerConfig::new()
            .command("test-server")
            .startup_timeout(Duration::from_secs(5));

        let server_id = bridge.register_server("test", config).await.unwrap();
        assert_eq!(server_id, "test");
        assert_eq!(bridge.server_count(), 1);
    }

    #[test]
    fn test_language_detection() {
        let bridge = LspBridge::new();

        assert_eq!(bridge.detect_language_id("file:///test.rs"), "rust");
        assert_eq!(bridge.detect_language_id("file:///test.py"), "python");
        assert_eq!(bridge.detect_language_id("file:///test.js"), "javascript");
        assert_eq!(
            bridge.detect_language_id("file:///test.unknown"),
            "plaintext"
        );
    }

    #[test]
    fn test_text_edits() {
        let bridge = LspBridge::new();
        let content = "line 1\nline 2\nline 3";

        let edits = vec![TextEdit {
            range: Range {
                start: Position {
                    line: 1,
                    character: 0,
                },
                end: Position {
                    line: 1,
                    character: 6,
                },
            },
            new_text: "modified".to_string(),
        }];

        let result = bridge.apply_text_edits(content, &edits).unwrap();
        assert_eq!(result, "line 1\nmodified\nline 3");
    }
}
