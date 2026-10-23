//! Multi-server example demonstrating coordination of multiple LSP servers.

use lsp_bridge::{LspBridge, LspServerConfig};
use std::time::Duration;
use tracing::{Level, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Starting LSP Bridge multi-server example");

    // Create LSP Bridge
    let mut bridge = LspBridge::new();

    // Configure multiple language servers
    let rust_config = LspServerConfig::new()
        .command("rust-analyzer")
        .root_path("/path/to/rust/project")
        .workspace_folder("/path/to/rust/project")
        .startup_timeout(Duration::from_secs(30))
        .initialization_options(serde_json::json!({
            "checkOnSave": {
                "command": "clippy"
            },
            "cargo": {
                "buildScripts": {
                    "enable": true
                }
            }
        }));

    let python_config = LspServerConfig::new()
        .command("pylsp")
        .root_path("/path/to/python/project")
        .workspace_folder("/path/to/python/project")
        .startup_timeout(Duration::from_secs(20))
        .initialization_options(serde_json::json!({
            "plugins": {
                "pycodestyle": {"enabled": false},
                "mccabe": {"enabled": false},
                "pyflakes": {"enabled": false},
                "flake8": {
                    "enabled": true,
                    "maxLineLength": 88
                }
            }
        }));

    let typescript_config = LspServerConfig::new()
        .command("typescript-language-server")
        .args(["--stdio"])
        .root_path("/path/to/ts/project")
        .workspace_folder("/path/to/ts/project")
        .startup_timeout(Duration::from_secs(25))
        .initialization_options(serde_json::json!({
            "preferences": {
                "disableSuggestions": false,
                "quotePreference": "double"
            }
        }));

    // Register all servers
    let rust_server = bridge.register_server("rust", rust_config).await?;
    let python_server = bridge.register_server("python", python_config).await?;
    let typescript_server = bridge
        .register_server("typescript", typescript_config)
        .await?;

    info!("Registered servers:");
    info!("  - Rust: {}", rust_server);
    info!("  - Python: {}", python_server);
    info!("  - TypeScript: {}", typescript_server);

    // Show server count
    println!("Total servers registered: {}", bridge.server_count());

    // List all servers
    let servers = bridge.list_servers();
    println!("Server IDs: {servers:?}");

    // Demonstrate server selection for different file types
    let file_mappings = vec![
        ("file:///project/src/main.rs", &rust_server),
        ("file:///project/main.py", &python_server),
        ("file:///project/app.ts", &typescript_server),
        ("file:///project/lib.rs", &rust_server),
        ("file:///project/utils.py", &python_server),
        ("file:///project/index.ts", &typescript_server),
    ];

    println!("\nFile to server mappings:");
    for (file, server) in file_mappings {
        println!("  {file} -> {server}");
    }

    // In a real application, you would:
    // 1. Start all servers
    // 2. Wait for them to be ready
    // 3. Open documents in appropriate servers
    // 4. Handle requests based on file type

    println!("\nMulti-server coordination example completed!");
    println!("This demonstrates how to manage multiple language servers");
    println!("for different programming languages in a single application.");

    Ok(())
}
