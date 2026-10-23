//! Document operations example showing LSP features like completion, hover, and formatting.

use lsp_bridge::{LspBridge, LspServerConfig};
use lsp_types::Position;
use std::time::Duration;
use tracing::{Level, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Starting LSP Bridge document operations example");

    // Create LSP Bridge
    let mut bridge = LspBridge::new();

    // Configure Rust analyzer (example - would need real server for actual operations)
    let config = LspServerConfig::new()
        .command("rust-analyzer")
        .root_path("/tmp/rust-project")
        .workspace_folder("/tmp/rust-project")
        .startup_timeout(Duration::from_secs(30))
        .request_timeout(Duration::from_secs(10))
        .initialization_options(serde_json::json!({
            "checkOnSave": {
                "command": "clippy"
            }
        }));

    // Register server
    let server_id = bridge.register_server("rust", config).await?;
    info!("Registered Rust analyzer server: {}", server_id);

    // Example document content
    let document_uri = "file:///tmp/rust-project/src/main.rs";
    let document_content = r#"use std::collections::HashMap;

fn main() {
    let mut map = HashMap::new();
    map.insert("hello", "world");
    map.insert("foo", "bar");
    
    for (key, value) in &map {
        println!("{}: {}", key, value);
    }
    
    let result = calculate_sum(10, 20);
    println!("Sum: {}", result);
}

fn calculate_sum(a: i32, b: i32) -> i32 {
    a + b
}

struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    
    fn distance_from_origin(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}
"#;

    println!("Document URI: {document_uri}");
    println!(
        "Document content ({} lines):",
        document_content.lines().count()
    );
    println!("{document_content}");

    // Demonstrate various document operations (would work with real server)
    println!("\n=== Document Operations (Demo) ===");

    // 1. Open document
    println!("1. Opening document...");
    // bridge.open_document(&server_id, document_uri, document_content).await?;

    // 2. Completion at different positions
    let completion_positions = vec![
        (5, 8),   // After "map."
        (11, 20), // After "calculate_"
        (23, 14), // After "self."
    ];

    for (line, character) in completion_positions {
        let _position = Position { line, character };
        println!("2. Getting completions at line {line}, character {character}...");

        // With real server:
        // let completions = bridge.get_completions(&server_id, document_uri, position).await?;
        // println!("   Found {} completions", completions.len());

        // Demo output
        println!("   Would find completions like: insert, get, contains_key, etc.");
    }

    // 3. Hover information
    let hover_positions = vec![
        (3, 15),  // Over "HashMap"
        (11, 25), // Over "calculate_sum"
        (23, 8),  // Over "distance_from_origin"
    ];

    for (line, character) in hover_positions {
        let _position = Position { line, character };
        println!("3. Getting hover info at line {line}, character {character}...");

        // With real server:
        // let hover = bridge.get_hover(&server_id, document_uri, position).await?;
        // if let Some(info) = hover {
        //     println!("   Hover: {:?}", info.contents);
        // }

        // Demo output
        println!("   Would show type information and documentation");
    }

    // 4. Go to definition
    let definition_positions = vec![
        (11, 17), // "calculate_sum" call
        (19, 5),  // "Point" usage
    ];

    for (line, character) in definition_positions {
        let _position = Position { line, character };
        println!("4. Going to definition at line {line}, character {character}...");

        // With real server:
        // let definition = bridge.go_to_definition(&server_id, document_uri, position).await?;
        // if let Some(location) = definition {
        //     println!("   Definition at: {}:{}", location.uri, location.range.start.line);
        // }

        // Demo output
        println!("   Would navigate to function/struct definition");
    }

    // 5. Find references
    let reference_position = Position {
        line: 15,
        character: 3,
    }; // "calculate_sum" definition
    println!(
        "5. Finding references for symbol at line {}, character {}...",
        reference_position.line, reference_position.character
    );

    // With real server:
    // let references = bridge.find_references(&server_id, document_uri, reference_position).await?;
    // println!("   Found {} references", references.len());

    // Demo output
    println!("   Would find all usages of calculate_sum function");

    // 6. Document formatting
    println!("6. Formatting document...");

    // With real server:
    // let formatted = bridge.format_document(&server_id, document_uri).await?;
    // println!("   Formatted document length: {} bytes", formatted.len());

    // Demo output
    println!("   Would apply rustfmt formatting to the document");

    // 7. Get diagnostics
    println!("7. Getting diagnostics...");

    // With real server:
    // let diagnostics = bridge.get_diagnostics(&server_id, document_uri)?;
    // for diagnostic in diagnostics {
    //     println!("   {}: {}", diagnostic.severity.unwrap_or_default(), diagnostic.message);
    // }

    // Demo output
    println!("   Would show any compiler errors, warnings, or lints");

    // 8. Document symbols
    println!("8. Getting document symbols...");
    println!(
        "   Would show: main(), calculate_sum(), Point struct, Point::new(), Point::distance_from_origin()"
    );

    println!("\nDocument operations example completed!");
    println!("This demonstrates the rich LSP features available through LSP Bridge.");

    Ok(())
}
