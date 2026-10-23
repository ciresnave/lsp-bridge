//! Advanced LSP Bridge Usage Example
//!
//! This example demonstrates more complex scenarios and advanced features
//! including handling diagnostics, custom server configurations, and error handling.

use lsp_bridge::{LspBridge, LspServerConfig};
use lsp_types::{CompletionParams, Position, TextDocumentIdentifier, Uri};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{Level, error, info};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

/// Initialize logging with tracing
fn setup_logging() {
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::DEBUG.into()))
        .with_target(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");
}

/// Example function to handle server capabilities
async fn explore_capabilities(bridge: &LspBridge, server_id: &str) -> lsp_bridge::Result<()> {
    // Get server capabilities
    let caps = match bridge.server_capabilities(server_id).await {
        Ok(caps) => caps,
        Err(e) => {
            info!("Failed to get server capabilities: {}", e);
            return Ok(());
        }
    };

    // Print out supported features
    info!("Server capabilities for {}:", server_id);
    info!("  Completion: {}", caps.supports_completion());
    info!("  Hover: {}", caps.supports_hover());
    info!("  Go to definition: {}", caps.supports_definition());
    info!(
        "  Document formatting: {}",
        caps.supports_document_formatting()
    );
    info!("  References: {}", caps.supports_references());
    info!("  Workspace symbols: {}", caps.supports_workspace_symbol());

    Ok(())
}

/// Example of advanced completion handling
async fn get_advanced_completions(
    bridge: &LspBridge,
    server_id: &str,
    uri: &str,
    position: Position,
) -> lsp_bridge::Result<()> {
    // Create completion params with full options
    let _params = CompletionParams {
        text_document_position: lsp_types::TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: Uri::from_str(uri).expect("Invalid URI"),
            },
            position,
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: Some(lsp_types::CompletionContext {
            trigger_kind: lsp_types::CompletionTriggerKind::TRIGGER_CHARACTER,
            trigger_character: Some(".".to_string()),
        }),
    };

    // Get completions
    let completions = match bridge.get_completions(server_id, uri, position).await {
        Ok(result) => result,
        Err(e) => {
            error!("Completion error: {}", e);
            return Err(e);
        }
    };

    // Process completions
    info!("Received {} completion items", completions.len());
    for (i, item) in completions.iter().take(5).enumerate() {
        info!(
            "  {}. {} ({})",
            i + 1,
            item.label,
            item.kind
                .map_or("unknown".to_string(), |k| format!("{k:?}"))
        );
    }
    if completions.len() > 5 {
        info!("  ... and {} more", completions.len() - 5);
    }

    Ok(())
}

/// Create custom server configuration based on language
fn create_server_config(language: &str, root_path: &Path) -> Option<(String, LspServerConfig)> {
    match language {
        "rust" => {
            // Rust Analyzer configuration
            let config = LspServerConfig::new()
                .command("rust-analyzer")
                .root_path(root_path)
                .initialization_options(serde_json::json!({
                    "checkOnSave": { "command": "clippy" },
                    "procMacro": { "enable": true },
                    "cargo": { "allFeatures": true }
                }));
            Some(("rust".to_string(), config))
        }
        "typescript" | "javascript" => {
            // TypeScript Language Server configuration
            let config = LspServerConfig::new()
                .command("typescript-language-server")
                .args(vec!["--stdio".to_string()])
                .root_path(root_path)
                .initialization_options(serde_json::json!({
                    "preferences": {
                        "importModuleSpecifierPreference": "relative"
                    }
                }));
            Some(("ts".to_string(), config))
        }
        "python" => {
            // Python Language Server (pyright) configuration
            let config = LspServerConfig::new()
                .command("pyright-langserver")
                .args(vec!["--stdio".to_string()])
                .root_path(root_path)
                .initialization_options(serde_json::json!({
                    "python": {
                        "analysis": {
                            "autoSearchPaths": true,
                            "useLibraryCodeForTypes": true,
                            "diagnosticMode": "workspace"
                        }
                    }
                }));
            Some(("python".to_string(), config))
        }
        _ => None,
    }
}

/// Example of error handling and recovery
async fn error_handling_example(bridge: &mut LspBridge, server_id: &str) -> lsp_bridge::Result<()> {
    // 1. Handle server crash and restart
    info!("Testing server crash recovery...");

    // Force server to restart (simulating a crash)
    bridge.restart_server(server_id).await?;
    info!("Server restarted successfully");

    // 2. Handle timeout for slow requests
    info!("Testing request timeout handling...");
    match timeout(
        Duration::from_secs(2),
        bridge.server_capabilities(server_id),
    )
    .await
    {
        Ok(result) => match result {
            Ok(_) => info!("Got server capabilities within timeout"),
            Err(e) => error!("Error getting capabilities: {}", e),
        },
        Err(_) => {
            error!("Request timed out");
            // Force a restart in case of timeout
            bridge.restart_server(server_id).await?;
        }
    }

    // 3. Handle invalid requests
    info!("Testing invalid request handling...");
    let invalid_uri = "invalid:uri";
    match bridge.open_document(server_id, invalid_uri, "").await {
        Ok(_) => info!("Document opened successfully (unexpected)"),
        Err(e) => {
            info!("Expected error occurred: {}", e);
            // The error is expected, so we recover and continue
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup logging
    setup_logging();

    // Get current directory as root path
    let root_path = std::env::current_dir()?;
    info!("Using root path: {}", root_path.display());

    // Create bridge
    let mut bridge = LspBridge::new();

    // Configure and start servers
    let language = "rust"; // Can be changed to "typescript" or "python"

    if let Some((id, config)) = create_server_config(language, &root_path) {
        info!("Registering {} server...", language);

        // Register server
        let server_id = bridge.register_server(id.clone(), config).await?;

        // Start server
        info!("Starting server...");
        bridge.start_server(&server_id).await?;

        // Wait for server to initialize
        info!("Waiting for server to initialize...");
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Explore capabilities
        explore_capabilities(&bridge, &server_id).await?;

        // Test error handling and recovery
        error_handling_example(&mut bridge, &server_id).await?;

        // Create a test file
        let file_uri = format!(
            "file://{}/test.{}",
            root_path.display(),
            if language == "rust" {
                "rs"
            } else if language == "python" {
                "py"
            } else {
                "ts"
            }
        );

        // Sample content based on language
        let content = match language {
            "rust" => {
                "fn main() {\n    println!(\"Hello, world!\");\n}\n\nstruct Test {\n    field: i32\n}\n"
            }
            "python" => {
                "def main():\n    print(\"Hello, world!\")\n\nclass Test:\n    def __init__(self):\n        self.field = 42\n"
            }
            _ => {
                "function main() {\n    console.log(\"Hello, world!\");\n}\n\nclass Test {\n    field: number = 42;\n}\n"
            }
        };

        // Open document
        info!("Opening document {}...", file_uri);
        bridge.open_document(&server_id, &file_uri, content).await?;

        // Get completions at a specific position
        let position = Position {
            line: 1,
            character: 14,
        };
        get_advanced_completions(&bridge, &server_id, &file_uri, position).await?;

        // Clean up - close document
        info!("Closing document...");
        bridge.close_document(&server_id, &file_uri).await?;

        // Shutdown server
        info!("Shutting down server...");
        bridge.stop_server(&server_id).await?;
    } else {
        error!("Unsupported language: {}", language);
    }

    info!("Example completed successfully");
    Ok(())
}
