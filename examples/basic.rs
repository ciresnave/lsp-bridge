//! Basic LSP Bridge example demonstrating server setup and document operations.

use lsp_bridge::{LspBridge, LspServerConfig};
use lsp_types::Position;
use std::time::Duration;
use tracing::{Level, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Starting LSP Bridge basic example");

    // Create LSP Bridge
    let mut bridge = LspBridge::new();

    // Configure a language server (using echo as a mock server for demo)
    let config = LspServerConfig::new()
        .command("echo")
        .arg("LSP server simulation")
        .startup_timeout(Duration::from_secs(10))
        .request_timeout(Duration::from_secs(5));

    // Register the server
    let server_id = bridge.register_server("demo", config).await?;
    info!("Registered server: {}", server_id);

    // Note: In a real scenario, you would start the server like this:
    // bridge.start_server(&server_id).await?;
    // bridge.wait_server_ready(&server_id).await?;

    // For this demo, we'll just show the configuration
    info!("Server configuration completed");

    // Example document operations (would work with a real LSP server)
    let document_uri = "file:///tmp/example.rs";
    let content = r#"
fn main() {
    println!("Hello, LSP Bridge!");
    let x = 42;
    println!("The answer is {}", x);
}
"#;

    info!("Document URI: {}", document_uri);
    info!("Document content length: {} bytes", content.len());

    // Show what operations would be available with a real server
    let position = Position {
        line: 2,
        character: 15,
    };
    info!(
        "Example position for operations: line {}, character {}",
        position.line, position.character
    );

    println!("Basic LSP Bridge example completed successfully!");
    println!("With a real language server, you could:");
    println!("- Open documents: bridge.open_document()");
    println!("- Get completions: bridge.get_completions()");
    println!("- Get hover info: bridge.get_hover()");
    println!("- Go to definition: bridge.go_to_definition()");
    println!("- Format documents: bridge.format_document()");
    println!("- And much more!");

    Ok(())
}
