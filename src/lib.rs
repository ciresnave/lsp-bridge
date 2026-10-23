//! # LSP Bridge
//!
//! A comprehensive Rust library that provides a bridge between Language Server Protocol (LSP)
//! servers and clients. It simplifies the integration of LSP capabilities into applications,
//! tools, and IDEs by handling the complexity of protocol communication, lifecycle management,
//! and feature negotiation.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Your Application                         │
//! └─────────────────────┬───────────────────────────────────────┘
//!                       │
//! ┌─────────────────────▼───────────────────────────────────────┐
//! │                   LspBridge                                 │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐          │
//! │  │   Config    │ │   Monitor   │ │   Registry  │          │
//! │  │   Manager   │ │   Health    │ │   Servers   │          │
//! │  └─────────────┘ └─────────────┘ └─────────────┘          │
//! └─────────────────────┬───────────────────────────────────────┘
//!                       │
//! ┌─────────────────────▼───────────────────────────────────────┐
//! │                   LspClient                                 │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐          │
//! │  │   Message   │ │   Server    │ │   Active    │          │
//! │  │   Router    │ │   Pool      │ │   Tracker   │          │
//! │  └─────────────┘ └─────────────┘ └─────────────┘          │
//! └─────────────────────┬───────────────────────────────────────┘
//!                       │
//! ┌─────────────────────▼───────────────────────────────────────┐
//! │                   LspServer                                 │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐          │
//! │  │   State     │ │   Request   │ │   Process   │          │
//! │  │   Manager   │ │   Handler   │ │   Manager   │          │
//! │  └─────────────┘ └─────────────┘ └─────────────┘          │
//! └─────────────────────┬───────────────────────────────────────┘
//!                       │
//! ┌─────────────────────▼───────────────────────────────────────┐
//! │              External LSP Servers                           │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐          │
//! │  │rust-analyzer│ │typescript-ls│ │    pylsp    │    ...   │
//! │  └─────────────┘ └─────────────┘ └─────────────┘          │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Features
//!
//! - Complete LSP protocol implementation (version 3.17+)
//! - Server lifecycle management (startup, shutdown, crash recovery)
//! - Asynchronous communication handling with tokio
//! - Automatic server capability detection and negotiation
//! - Request/notification routing and multiplexing
//! - Support for custom LSP extensions
//! - Language-specific configuration
//! - Progress reporting and cancellation
//! - Document synchronization
//! - Multi-server coordination
//! - Built-in monitoring and observability
//! - Thread-safe concurrent operations
//!
//! ## Quick Start
//!
//! ### Basic Setup
//!
//! ```rust,no_run
//! use lsp_bridge::{LspBridge, LspServerConfig};
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a new bridge
//!     let mut bridge = LspBridge::new();
//!     
//!     // Configure rust-analyzer server
//!     let config = LspServerConfig::new()
//!         .command("rust-analyzer")
//!         .root_path(PathBuf::from("/path/to/project"));
//!     
//!     // Register and start the server
//!     let server_id = bridge.register_server("rust-analyzer", config).await?;
//!     bridge.start_server(&server_id).await?;
//!     
//!     // Open a document
//!     bridge.open_document(
//!         &server_id,
//!         "file:///path/to/file.rs",
//!         &std::fs::read_to_string("/path/to/file.rs")?
//!     ).await?;
//!     
//!     // Get completions
//!     let completions = bridge.get_completions(
//!         &server_id,
//!         "file:///path/to/file.rs",
//!         lsp_types::Position::new(10, 5)
//!     ).await?;
//!     
//!     println!("Found {} completions", completions.len());
//!     
//!     // Clean shutdown
//!     bridge.stop_server(&server_id).await?;
//!     Ok(())
//! }
//! ```
//!
//! ### Multi-Server Setup
//!
//! ```rust,no_run
//! use lsp_bridge::{LspBridge, LspServerConfig};
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut bridge = LspBridge::new();
//!     
//!     // Configure multiple servers
//!     let workspace = PathBuf::from("/path/to/mixed/project");
//!     
//!     // Rust server
//!     let rust_config = LspServerConfig::new()
//!         .command("rust-analyzer")
//!         .root_path(workspace.clone());
//!     let rust_id = bridge.register_server("rust", rust_config).await?;
//!     
//!     // TypeScript server  
//!     let ts_config = LspServerConfig::new()
//!         .command("typescript-language-server")
//!         .args(vec!["--stdio".to_string()])
//!         .root_path(workspace.clone());
//!     let ts_id = bridge.register_server("typescript", ts_config).await?;
//!     
//!     // Start both servers
//!     bridge.start_server(&rust_id).await?;
//!     bridge.start_server(&ts_id).await?;
//!     
//!     // Work with different file types
//!     bridge.open_document(&rust_id, "file:///project/src/main.rs", "fn main() {}").await?;
//!     bridge.open_document(&ts_id, "file:///project/index.ts", "console.log('hi')").await?;
//!     
//!     // Get language-specific features
//!     let rust_hover = bridge.get_hover(&rust_id, "file:///project/src/main.rs",
//!                                       lsp_types::Position::new(0, 3)).await?;
//!     let ts_completions = bridge.get_completions(&ts_id, "file:///project/index.ts",
//!                                                  lsp_types::Position::new(0, 8)).await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! The LSP Bridge is structured around several core components:
//!
//! - [`LspBridge`]: Main coordination interface managing multiple servers
//! - [`LspClient`]: Client-side protocol implementation
//! - [`LspServer`]: Individual server lifecycle and communication
//! - [`LspServerConfig`]: Server configuration and validation
//! - Protocol types: Type-safe LSP message handling
//!
//! ## Error Handling
//!
//! All operations return [`Result<T, LspBridgeError>`](error::LspBridgeError) which provides
//! detailed error context and recovery information:
//!
//! ```rust,no_run
//! use lsp_bridge::{LspBridge, error::LspBridgeError};
//!
//! async fn handle_errors() {
//!     let mut bridge = LspBridge::new();
//!     
//!     match bridge.start_server("non-existent").await {
//!         Ok(_) => println!("Server started"),
//!         Err(LspBridgeError::Lsp(err)) => {
//!             println!("LSP error: {}", err);
//!             // Handle LSP-specific errors
//!         },
//!         Err(LspBridgeError::Io(err)) => {
//!             println!("I/O error: {}", err);
//!             // Handle I/O errors  
//!         },
//!         Err(err) => {
//!             println!("Other error: {}", err);
//!         }
//!     }
//! }
//! ```
//!
//! ## Thread Safety
//!
//! All operations are thread-safe and can be used from multiple async tasks:
//!
//! ```rust,no_run
//! use lsp_bridge::LspBridge;
//! use std::sync::Arc;
//! use tokio::task;
//!
//! async fn concurrent_operations() {
//!     let bridge = Arc::new(LspBridge::new());
//!     
//!     let bridge1 = bridge.clone();
//!     let bridge2 = bridge.clone();
//!     
//!     let task1 = task::spawn(async move {
//!         // Work with bridge1
//!     });
//!     
//!     let task2 = task::spawn(async move {
//!         // Work with bridge2  
//!     });
//!     
//!     let _ = tokio::try_join!(task1, task2);
//! }
//! ```
//! # Example Usage
//!
//! ```no_run
//! use lsp_bridge::{LspBridge, LspServerConfig};
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Configure the LSP server
//!     let config = LspServerConfig::new()
//!         .command("rust-analyzer")
//!         .root_path(PathBuf::from("/path/to/project"));
//!     
//!     // Create and start the bridge
//!     let mut bridge = LspBridge::new();
//!     let server_id = bridge.register_server("rust", config).await?;
//!     bridge.start_server(&server_id).await?;
//!     
//!     // Use LSP features
//!     let document_uri = "file:///path/to/project/src/main.rs";
//!     bridge.open_document(&server_id, document_uri, "fn main() {}").await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! # Performance
//!
//! The LSP Bridge is designed for high performance with:
//! - Async/await throughout for non-blocking operations
//! - Efficient message serialization (~370ns for typical messages)
//! - Memory-efficient data structures with DashMap
//! - Connection pooling and resource management
//!
//! ## License
//!
//! Licensed under either of Apache License, Version 2.0 or MIT license at your option.

pub mod bridge;
pub mod client;
pub mod config;
pub mod error;
pub mod hardening;
pub mod monitoring;
pub mod process;
pub mod protocol;
pub mod server;
pub mod utils; // Add process module

// Re-export main types for convenience
pub use bridge::LspBridge;
pub use client::LspClient;
pub use config::{LspClientCapabilities, LspServerConfig};
pub use error::{LspBridgeError, LspError, Result};
pub use hardening::{HardeningConfig, HardeningManager, ResourceLimits};
pub use monitoring::{HealthStatus, MetricsCollector, MetricsSnapshot};
pub use protocol::{LspMessage, LspNotification, LspRequest, LspResponse};
pub use server::{LspServer, LspServerManager};

/// Prelude module for common imports
pub mod prelude {
    pub use crate::{
        LspBridge, LspClientCapabilities, LspError, LspServer, LspServerConfig, Result,
        protocol::*, server::ServerState,
    };
    pub use lsp_types::{
        ClientCapabilities, CodeActionParams, CompletionItem, CompletionParams, Diagnostic,
        DocumentFormattingParams, GotoDefinitionParams, HoverParams, InitializeParams, Location,
        PartialResultParams, Position, Range, ReferenceParams, RenameParams, ServerCapabilities,
        TextDocumentIdentifier, TextDocumentPositionParams, Uri, WorkDoneProgressParams,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_creation() {
        let bridge = LspBridge::new();
        assert_eq!(bridge.server_count(), 0);
    }
}
