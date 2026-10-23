//! LSP protocol implementation and utilities.

use crate::error::{LspError, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use lsp_types::*;

/// LSP request identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    Number(i64),
    String(String),
}

impl RequestId {
    /// Generate a new unique request ID.
    pub fn new() -> Self {
        Self::String(Uuid::new_v4().to_string())
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<i64> for RequestId {
    fn from(id: i64) -> Self {
        Self::Number(id)
    }
}

impl From<String> for RequestId {
    fn from(id: String) -> Self {
        Self::String(id)
    }
}

impl From<&str> for RequestId {
    fn from(id: &str) -> Self {
        Self::String(id.to_string())
    }
}

/// LSP JSON-RPC message types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LspMessage {
    Request(LspRequest),
    Response(LspResponse),
    Notification(LspNotification),
}

/// LSP request message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspRequest {
    pub jsonrpc: String,
    pub id: RequestId,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

impl LspRequest {
    /// Create a new LSP request.
    pub fn new<M: Into<String>>(method: M, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: RequestId::new(),
            method: method.into(),
            params,
        }
    }

    /// Create a new LSP request with a specific ID.
    pub fn with_id<M: Into<String>>(
        id: RequestId,
        method: M,
        params: Option<serde_json::Value>,
    ) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

/// LSP response message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspResponse {
    pub jsonrpc: String,
    pub id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<LspResponseError>,
}

impl LspResponse {
    /// Create a successful response.
    pub fn success(id: RequestId, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response.
    pub fn error(id: RequestId, error: LspResponseError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// LSP notification message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

impl LspNotification {
    /// Create a new LSP notification.
    pub fn new<M: Into<String>>(method: M, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
        }
    }
}

/// LSP response error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspResponseError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl LspResponseError {
    /// Create a new response error.
    pub fn new<M: Into<String>>(code: i32, message: M) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Create a response error with data.
    pub fn with_data<M: Into<String>>(code: i32, message: M, data: serde_json::Value) -> Self {
        Self {
            code,
            message: message.into(),
            data: Some(data),
        }
    }
}

/// Standard LSP error codes.
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    pub const SERVER_ERROR_START: i32 = -32099;
    pub const SERVER_ERROR_END: i32 = -32000;
    pub const SERVER_NOT_INITIALIZED: i32 = -32002;
    pub const UNKNOWN_ERROR_CODE: i32 = -32001;
    pub const REQUEST_FAILED: i32 = -32803;
    pub const SERVER_CANCELLED: i32 = -32802;
    pub const CONTENT_MODIFIED: i32 = -32801;
    pub const REQUEST_CANCELLED: i32 = -32800;
}

/// LSP server capabilities wrapper.
#[derive(Debug, Clone)]
pub struct ServerCapabilities {
    pub inner: lsp_types::ServerCapabilities,
}

impl ServerCapabilities {
    /// Create new server capabilities.
    pub fn new(capabilities: lsp_types::ServerCapabilities) -> Self {
        Self {
            inner: capabilities,
        }
    }

    /// Check if the server supports text document synchronization.
    pub fn supports_text_document_sync(&self) -> bool {
        self.inner.text_document_sync.is_some()
    }

    /// Check if the server supports completion.
    pub fn supports_completion(&self) -> bool {
        self.inner.completion_provider.is_some()
    }

    /// Check if the server supports hover.
    pub fn supports_hover(&self) -> bool {
        self.inner.hover_provider.is_some()
    }

    /// Check if the server supports signature help.
    pub fn supports_signature_help(&self) -> bool {
        self.inner.signature_help_provider.is_some()
    }

    /// Check if the server supports go to definition.
    pub fn supports_definition(&self) -> bool {
        self.inner.definition_provider.is_some()
    }

    /// Check if the server supports go to type definition.
    pub fn supports_type_definition(&self) -> bool {
        self.inner.type_definition_provider.is_some()
    }

    /// Check if the server supports go to implementation.
    pub fn supports_implementation(&self) -> bool {
        self.inner.implementation_provider.is_some()
    }

    /// Check if the server supports find references.
    pub fn supports_references(&self) -> bool {
        self.inner.references_provider.is_some()
    }

    /// Check if the server supports document highlighting.
    pub fn supports_document_highlight(&self) -> bool {
        self.inner.document_highlight_provider.is_some()
    }

    /// Check if the server supports document symbols.
    pub fn supports_document_symbol(&self) -> bool {
        self.inner.document_symbol_provider.is_some()
    }

    /// Check if the server supports code actions.
    pub fn supports_code_action(&self) -> bool {
        self.inner.code_action_provider.is_some()
    }

    /// Check if the server supports code lens.
    pub fn supports_code_lens(&self) -> bool {
        self.inner.code_lens_provider.is_some()
    }

    /// Check if the server supports document formatting.
    pub fn supports_document_formatting(&self) -> bool {
        self.inner.document_formatting_provider.is_some()
    }

    /// Check if the server supports document range formatting.
    pub fn supports_document_range_formatting(&self) -> bool {
        self.inner.document_range_formatting_provider.is_some()
    }

    /// Check if the server supports document on type formatting.
    pub fn supports_document_on_type_formatting(&self) -> bool {
        self.inner.document_on_type_formatting_provider.is_some()
    }

    /// Check if the server supports rename.
    pub fn supports_rename(&self) -> bool {
        self.inner.rename_provider.is_some()
    }

    /// Check if the server supports workspace symbols.
    pub fn supports_workspace_symbol(&self) -> bool {
        self.inner.workspace_symbol_provider.is_some()
    }

    /// Check if the server supports execute command.
    pub fn supports_execute_command(&self) -> bool {
        self.inner.execute_command_provider.is_some()
    }

    /// Get completion trigger characters.
    pub fn completion_trigger_characters(&self) -> Vec<String> {
        self.inner
            .completion_provider
            .as_ref()
            .and_then(|cp| cp.trigger_characters.clone())
            .unwrap_or_default()
    }

    /// Get signature help trigger characters.
    pub fn signature_help_trigger_characters(&self) -> Vec<String> {
        self.inner
            .signature_help_provider
            .as_ref()
            .and_then(|shp| shp.trigger_characters.clone())
            .unwrap_or_default()
    }
}

/// Document state tracking.
#[derive(Debug, Clone)]
pub struct DocumentState {
    pub uri: String,
    pub version: i32,
    pub content: String,
    pub language_id: String,
    pub diagnostics: Vec<Diagnostic>,
}

impl DocumentState {
    /// Create a new document state.
    pub fn new<U, C, L>(uri: U, version: i32, content: C, language_id: L) -> Self
    where
        U: Into<String>,
        C: Into<String>,
        L: Into<String>,
    {
        Self {
            uri: uri.into(),
            version,
            content: content.into(),
            language_id: language_id.into(),
            diagnostics: Vec::new(),
        }
    }

    /// Update the document content.
    pub fn update_content<C: Into<String>>(&mut self, content: C) {
        self.content = content.into();
        self.version += 1;
    }

    /// Update diagnostics for the document.
    pub fn update_diagnostics(&mut self, diagnostics: Vec<Diagnostic>) {
        self.diagnostics = diagnostics;
    }

    /// Get the document text in a specific range.
    pub fn get_text_range(&self, range: &Range) -> Result<String> {
        let lines: Vec<&str> = self.content.lines().collect();

        if range.start.line as usize >= lines.len() || range.end.line as usize >= lines.len() {
            return Err(LspError::custom("Range out of bounds").into());
        }

        if range.start.line == range.end.line {
            let line = lines[range.start.line as usize];
            let start_char = range.start.character as usize;
            let end_char = range.end.character as usize;

            if start_char > line.len() || end_char > line.len() {
                return Err(LspError::custom("Character position out of bounds").into());
            }

            return Ok(line[start_char..end_char].to_string());
        }

        let mut result = String::new();
        for (i, line) in lines.iter().enumerate() {
            let line_num = i as u32;
            if line_num >= range.start.line && line_num <= range.end.line {
                if line_num == range.start.line {
                    let start_char = range.start.character as usize;
                    if start_char <= line.len() {
                        result.push_str(&line[start_char..]);
                    }
                } else if line_num == range.end.line {
                    let end_char = range.end.character as usize;
                    if end_char <= line.len() {
                        result.push_str(&line[..end_char]);
                    }
                } else {
                    result.push_str(line);
                }

                if line_num < range.end.line {
                    result.push('\n');
                }
            }
        }

        Ok(result)
    }
}

/// Utility functions for LSP protocol.
pub mod utils {
    use super::*;

    /// Convert a file path to an LSP URI.
    pub fn path_to_uri<P: AsRef<std::path::Path>>(path: P) -> String {
        let path = path.as_ref();
        format!("file://{}", path.display())
    }

    /// Convert an LSP URI to a file path.
    pub fn uri_to_path(uri: &str) -> Result<std::path::PathBuf> {
        if let Ok(url) = url::Url::parse(uri)
            && url.scheme() == "file"
            && let Ok(path) = url.to_file_path()
        {
            return Ok(path);
        }
        Err(LspError::invalid_uri(uri).into())
    }

    /// Create a position from line and character.
    pub fn position(line: u32, character: u32) -> Position {
        Position { line, character }
    }

    /// Create a range from start and end positions.
    pub fn range(start: Position, end: Position) -> Range {
        Range { start, end }
    }

    /// Create a range from line and character coordinates.
    pub fn range_from_coords(
        start_line: u32,
        start_char: u32,
        end_line: u32,
        end_char: u32,
    ) -> Range {
        Range {
            start: Position {
                line: start_line,
                character: start_char,
            },
            end: Position {
                line: end_line,
                character: end_char,
            },
        }
    }

    /// Create a text document identifier.
    pub fn text_document_identifier(uri: Uri) -> TextDocumentIdentifier {
        TextDocumentIdentifier { uri }
    }

    /// Create a versioned text document identifier.
    pub fn versioned_text_document_identifier(
        uri: Uri,
        version: i32,
    ) -> VersionedTextDocumentIdentifier {
        VersionedTextDocumentIdentifier { uri, version }
    }

    /// Create a text document item.
    pub fn text_document_item<L, T>(
        uri: Uri,
        language_id: L,
        version: i32,
        text: T,
    ) -> TextDocumentItem
    where
        L: Into<String>,
        T: Into<String>,
    {
        TextDocumentItem {
            uri,
            language_id: language_id.into(),
            version,
            text: text.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_generation() {
        let id1 = RequestId::new();
        let id2 = RequestId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_lsp_request_creation() {
        let request = LspRequest::new("textDocument/completion", None);
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.method, "textDocument/completion");
        assert!(request.params.is_none());
    }

    #[test]
    fn test_lsp_response_creation() {
        let id = RequestId::new();
        let response = LspResponse::success(id.clone(), serde_json::json!({"result": "test"}));
        assert_eq!(response.id, id);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_document_state() {
        let mut doc = DocumentState::new("file:///test.rs", 1, "fn main() {}", "rust");
        doc.update_content("fn main() { println!(\"Hello\"); }");
        assert_eq!(doc.version, 2);
        assert!(doc.content.contains("println!"));
    }

    #[test]
    fn test_path_uri_conversion() {
        // Use a platform-appropriate path
        let path = if cfg!(windows) {
            std::path::Path::new("C:\\tmp\\test.rs")
        } else {
            std::path::Path::new("/tmp/test.rs")
        };

        let uri = utils::path_to_uri(path);
        assert!(uri.starts_with("file://"));

        let back_path = utils::uri_to_path(&uri).unwrap();
        assert_eq!(back_path, path);
    }
}
