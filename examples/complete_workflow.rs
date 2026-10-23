//! Complete LSP Bridge workflow example.
//!
//! This example demonstrates a full workflow with a real LSP server,
//! including server registration, document operations, and cleanup.

use lsp_bridge::{LspBridge, LspServerConfig};
use lsp_types::Position;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use tokio::fs;
use tracing::{Level, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    info!("🚀 Starting complete LSP Bridge workflow example");

    // Create a temporary workspace for testing
    let temp_dir = setup_test_workspace().await?;
    let workspace_path = temp_dir.path().to_path_buf();

    // Create LSP Bridge
    let mut bridge = LspBridge::new();
    info!("✅ LSP Bridge created successfully");

    // Check if rust-analyzer is available
    if !is_rust_analyzer_available().await {
        warn!("⚠️ rust-analyzer not found, using mock configuration");
        demonstrate_mock_workflow(&mut bridge).await?;
    } else {
        info!("🦀 rust-analyzer found, demonstrating real workflow");
        demonstrate_real_workflow(&mut bridge, workspace_path).await?;
    }

    info!("🎉 Complete workflow example finished successfully");
    Ok(())
}

/// Set up a temporary Rust workspace for testing
async fn setup_test_workspace() -> Result<TempDir, Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let workspace_path = temp_dir.path();

    // Create Cargo.toml
    let cargo_toml = r#"
[package]
name = "example-workspace"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
"#;
    fs::write(workspace_path.join("Cargo.toml"), cargo_toml).await?;

    // Create src directory and main.rs
    fs::create_dir_all(workspace_path.join("src")).await?;
    let main_rs = r#"
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
}

impl User {
    pub fn new(id: u64, name: String, email: String) -> Self {
        Self { id, name, email }
    }
    
    pub fn display_name(&self) -> String {
        format!("{} <{}>", self.name, self.email)
    }
    
    pub fn is_valid(&self) -> bool {
        !self.name.is_empty() && self.email.contains('@')
    }
}

#[tokio::main]
async fn main() {
    let user = User::new(
        1,
        "Alice Johnson".to_string(),
        "alice@example.com".to_string()
    );
    
    println!("User: {}", user.display_name());
    println!("Valid: {}", user.is_valid());
    
    // Serialize to JSON
    let json = serde_json::to_string_pretty(&user).unwrap();
    println!("JSON: {}", json);
}
"#;
    fs::write(workspace_path.join("src").join("main.rs"), main_rs).await?;

    info!("📁 Test workspace created at: {}", workspace_path.display());
    Ok(temp_dir)
}

/// Check if rust-analyzer is available on the system
async fn is_rust_analyzer_available() -> bool {
    tokio::process::Command::new("rust-analyzer")
        .arg("--version")
        .output()
        .await
        .is_ok()
}

/// Demonstrate workflow with a real rust-analyzer server
async fn demonstrate_real_workflow(
    bridge: &mut LspBridge,
    workspace_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("🔧 Configuring rust-analyzer server");

    // Configure rust-analyzer
    let config = LspServerConfig::new()
        .command("rust-analyzer")
        .root_path(workspace_path.clone())
        .working_directory(workspace_path.clone())
        .startup_timeout(Duration::from_secs(30))
        .request_timeout(Duration::from_secs(10));

    // Register and start the server
    let server_id = bridge.register_server("rust-analyzer", config).await?;
    info!("📝 Registered server with ID: {}", server_id);

    info!("🚀 Starting rust-analyzer server...");
    bridge.start_server(&server_id).await?;
    info!("✅ Server started successfully");

    // Allow server to initialize
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Open the main.rs file
    let file_path = workspace_path.join("src").join("main.rs");
    let file_uri = format!("file://{}", file_path.display());
    let file_content = fs::read_to_string(&file_path).await?;

    info!("📖 Opening document: {}", file_uri);
    bridge
        .open_document(&server_id, &file_uri, &file_content)
        .await?;

    // Allow processing time
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Test various LSP features
    info!("🔍 Testing LSP features:");

    // 1. Hover information
    info!("  • Testing hover on 'User' struct");
    match bridge
        .get_hover(&server_id, &file_uri, Position::new(4, 12))
        .await
    {
        Ok(Some(_hover)) => {
            info!("    ✅ Hover successful: got hover information");
        }
        Ok(None) => info!("    ℹ️ No hover information available"),
        Err(e) => warn!("    ⚠️ Hover failed: {}", e),
    }

    // 2. Completions
    info!("  • Testing completions in impl block");
    match bridge
        .get_completions(&server_id, &file_uri, Position::new(15, 20))
        .await
    {
        Ok(completions) => {
            info!("    ✅ Found {} completion items", completions.len());
            for (i, item) in completions.iter().take(3).enumerate() {
                info!(
                    "      {}. {} ({})",
                    i + 1,
                    item.label,
                    item.kind
                        .map_or("unknown".to_string(), |k| format!("{k:?}"))
                );
            }
        }
        Err(e) => warn!("    ⚠️ Completions failed: {}", e),
    }

    // 3. Go to definition
    info!("  • Testing go-to-definition on method call");
    match bridge
        .go_to_definition(&server_id, &file_uri, Position::new(32, 25))
        .await
    {
        Ok(Some(location)) => {
            info!(
                "    ✅ Found definition at: {:?}:{}",
                location.uri, location.range.start.line
            );
        }
        Ok(None) => info!("    ℹ️ No definition found"),
        Err(e) => warn!("    ⚠️ Go-to-definition failed: {}", e),
    }

    // 4. Document formatting
    info!("  • Testing document formatting");
    match bridge.format_document(&server_id, &file_uri).await {
        Ok(edits) => {
            info!("    ✅ Formatting returned {} text edits", edits.len());
        }
        Err(e) => warn!("    ⚠️ Formatting failed: {}", e),
    }

    // Close the document
    info!("📄 Closing document");
    bridge.close_document(&server_id, &file_uri).await?;

    // Shutdown the server
    info!("🛑 Shutting down server");
    bridge.stop_server(&server_id).await?;
    info!("✅ Server shutdown complete");

    Ok(())
}

/// Demonstrate workflow with mock configuration (when rust-analyzer is not available)
async fn demonstrate_mock_workflow(
    bridge: &mut LspBridge,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("🎭 Demonstrating mock workflow (no real LSP server)");

    // Configure a mock server (won't actually start but demonstrates configuration)
    let config = LspServerConfig::new()
        .command("mock-lsp-server")
        .args(vec!["--stdio".to_string(), "--verbose".to_string()])
        .root_path(PathBuf::from("/tmp/mock-project"))
        .startup_timeout(Duration::from_secs(5))
        .request_timeout(Duration::from_secs(3));

    // Register the server (this will work)
    let server_id = bridge.register_server("mock-server", config).await?;
    info!("📝 Registered mock server with ID: {}", server_id);

    // Show server information
    info!("📊 Bridge statistics:");
    info!("  • Server count: {}", bridge.server_count());
    info!(
        "  • Has server '{}': {}",
        server_id,
        bridge.has_server(&server_id)
    );
    info!("  • All servers: {:?}", bridge.list_servers());

    // Attempting to start would fail, so we just demonstrate the configuration
    info!("⚠️ Would attempt to start server, but mock-lsp-server doesn't exist");

    // Show what a real workflow would look like
    info!("📋 In a real scenario, you would:");
    info!("  1. bridge.start_server(&server_id).await?");
    info!("  2. bridge.open_document(&server_id, uri, content).await?");
    info!("  3. bridge.get_completions(&server_id, uri, position).await?");
    info!("  4. bridge.get_hover(&server_id, uri, position).await?");
    info!("  5. bridge.stop_server(&server_id).await?");

    Ok(())
}
