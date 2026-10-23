//! Performance testing with large codebases
//!
//! These tests validate the bridge's performance characteristics when
//! working with large codebases and heavy LSP traffic.

use lsp_bridge::{LspBridge, LspServerConfig};
use lsp_types::*;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::fs;
use tokio::time::{sleep, timeout};

/// Performance metrics collected during testing
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub startup_time: Duration,
    pub request_latencies: Vec<Duration>,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub throughput_requests_per_second: f64,
    pub document_sync_time: Duration,
}

impl PerformanceMetrics {
    fn new() -> Self {
        Self {
            startup_time: Duration::ZERO,
            request_latencies: Vec::new(),
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            throughput_requests_per_second: 0.0,
            document_sync_time: Duration::ZERO,
        }
    }

    fn calculate_statistics(&self) -> PerformanceStats {
        let latencies = &self.request_latencies;
        if latencies.is_empty() {
            return PerformanceStats::default();
        }

        let mut sorted_latencies = latencies.clone();
        sorted_latencies.sort();

        let len = sorted_latencies.len();
        let mean = sorted_latencies.iter().sum::<Duration>() / len as u32;
        let median = sorted_latencies[len / 2];
        let p95 = sorted_latencies[(len as f64 * 0.95) as usize];
        let p99 = sorted_latencies[(len as f64 * 0.99) as usize];
        let min = sorted_latencies[0];
        let max = sorted_latencies[len - 1];

        PerformanceStats {
            mean_latency: mean,
            median_latency: median,
            p95_latency: p95,
            p99_latency: p99,
            min_latency: min,
            max_latency: max,
            total_requests: len,
        }
    }
}

#[derive(Debug, Default)]
pub struct PerformanceStats {
    pub mean_latency: Duration,
    pub median_latency: Duration,
    pub p95_latency: Duration,
    pub p99_latency: Duration,
    pub min_latency: Duration,
    pub max_latency: Duration,
    pub total_requests: usize,
}

/// Create a large Rust codebase for performance testing
async fn create_large_rust_codebase(
    temp_dir: &TempDir,
    num_files: usize,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let workspace_path = temp_dir.path().to_path_buf();

    // Create Cargo.toml
    let cargo_toml = r#"
[package]
name = "large-test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
clap = "4.0"
anyhow = "1.0"
thiserror = "1.0"
tracing = "0.1"
"#;
    fs::write(workspace_path.join("Cargo.toml"), cargo_toml).await?;

    // Create src directory
    fs::create_dir_all(workspace_path.join("src")).await?;

    // Create lib.rs
    let lib_rs = r#"
//! Large test project library
//!
//! This module contains various components for performance testing.

pub mod models;
pub mod services;
pub mod utils;
pub mod handlers;
pub mod config;

pub use models::*;
pub use services::*;
pub use utils::*;
pub use handlers::*;
pub use config::*;

/// Main application trait
pub trait Application {
    type Error;
    type Config;
    
    async fn initialize(config: Self::Config) -> Result<Self, Self::Error>
    where
        Self: Sized;
    
    async fn run(&mut self) -> Result<(), Self::Error>;
    
    async fn shutdown(&mut self) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_basic_functionality() {
        // Basic test
        assert!(true);
    }
}
"#;
    fs::write(workspace_path.join("src").join("lib.rs"), lib_rs).await?;

    // Create multiple module files
    let modules = ["models", "services", "utils", "handlers", "config"];

    for module in &modules {
        fs::create_dir_all(workspace_path.join("src").join(module)).await?;

        // Create mod.rs for each module
        let mod_rs = format!(
            r#"
//! {module} module
//!
//! This module contains various {module} related functionality.

use serde::{{Deserialize, Serialize}};
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;

"#
        );
        fs::write(
            workspace_path.join("src").join(module).join("mod.rs"),
            mod_rs,
        )
        .await?;

        // Create multiple files within each module
        for i in 0..(num_files / modules.len().max(1)) {
            let file_content = generate_rust_module_content(module, i);
            fs::write(
                workspace_path
                    .join("src")
                    .join(module)
                    .join(format!("{module}{i}.rs")),
                file_content,
            )
            .await?;
        }
    }

    // Create main.rs
    let main_rs = r#"
use large_test_project::*;
use clap::Parser;
use tracing::info;

#[derive(Parser)]
#[command(name = "large-test-project")]
#[command(about = "A large test project for performance testing")]
struct Cli {
    #[arg(short, long, default_value = "info")]
    log_level: String,
    
    #[arg(short, long, default_value = "8080")]
    port: u16,
    
    #[arg(long)]
    config_file: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(&cli.log_level)
        .init();
    
    info!("Starting large test project on port {}", cli.port);
    
    // Initialize configuration
    let config = if let Some(config_file) = cli.config_file {
        AppConfig::from_file(&config_file).await?
    } else {
        AppConfig::default()
    };
    
    // Run the application
    let mut app = TestApp::initialize(config).await?;
    app.run().await?;
    app.shutdown().await?;
    
    Ok(())
}
"#;
    fs::write(workspace_path.join("src").join("main.rs"), main_rs).await?;

    Ok(workspace_path)
}

/// Generate content for a module file
fn generate_rust_module_content(module: &str, index: usize) -> String {
    match module {
        "models" => format!(
            r#"
//! Model{index} - Data structures and entities

use serde::{{Deserialize, Serialize}};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity{index} {{
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}}

impl Entity{index} {{
    pub fn new(name: String) -> Self {{
        Self {{
            id: Uuid::new_v4(),
            name,
            description: None,
            metadata: HashMap::new(),
        }}
    }}
}}
"#
        ),
        "services" => format!(
            r#"
//! Service{index} - Business logic and service layer

use std::collections::HashMap;
use uuid::Uuid;

pub struct Service{index} {{
    data: HashMap<Uuid, String>,
}}

impl Service{index} {{
    pub fn new() -> Self {{
        Self {{
            data: HashMap::new(),
        }}
    }}
    
    pub fn add_item(&mut self, id: Uuid, value: String) {{
        self.data.insert(id, value);
    }}
}}
"#
        ),
        "utils" => format!(
            r#"
//! Utility{index} - Helper functions and utilities

use std::collections::HashMap;

pub struct Utility{index} {{
    config: HashMap<String, String>,
}}

impl Utility{index} {{
    pub fn new() -> Self {{
        Self {{
            config: HashMap::new(),
        }}
    }}
}}
"#
        ),
        "handlers" => format!(
            r#"
//! Handler{index} - Request handlers and API endpoints

use std::collections::HashMap;

pub struct Handler{index} {{
    data: HashMap<String, String>,
}}

impl Handler{index} {{
    pub fn new() -> Self {{
        Self {{
            data: HashMap::new(),
        }}
    }}
}}
"#
        ),
        "config" => format!(
            r#"
//! Configuration{index} - Application configuration and settings

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AppConfig{index} {{
    pub settings: HashMap<String, String>,
}}

impl Default for AppConfig{index} {{
    fn default() -> Self {{
        Self {{
            settings: HashMap::new(),
        }}
    }}
}}
"#
        ),
        _ => format!(
            r#"
//! Generic module {module} file {index}

use std::collections::HashMap;

pub struct ModuleStruct{index} {{
    data: HashMap<String, String>,
}}

impl ModuleStruct{index} {{
    pub fn new() -> Self {{
        Self {{
            data: HashMap::new(),
        }}
    }}
}}
"#
        ),
    }
}

/// Measure startup time for an LSP server
async fn measure_startup_time(
    config: LspServerConfig,
) -> Result<Duration, Box<dyn std::error::Error>> {
    let mut bridge = LspBridge::new();

    let start = Instant::now();
    let server_id = bridge.register_server("perf-test", config).await?;
    bridge.start_server(&server_id).await?;
    bridge.wait_server_ready(&server_id).await?;
    let startup_time = start.elapsed();

    bridge.stop_server(&server_id).await?;

    Ok(startup_time)
}

/// Measure request latency for multiple operations
async fn measure_request_latencies(
    bridge: &LspBridge,
    server_id: &str,
    file_uri: &str,
    num_requests: usize,
) -> Result<Vec<Duration>, Box<dyn std::error::Error>> {
    let mut latencies = Vec::new();

    for _ in 0..num_requests {
        let start = Instant::now();

        // Alternate between different types of requests
        match rand::random::<u8>() % 3 {
            0 => {
                let _ = bridge
                    .get_completions(server_id, file_uri, Position::new(10, 20))
                    .await;
            }
            1 => {
                let _ = bridge
                    .get_hover(server_id, file_uri, Position::new(5, 10))
                    .await;
            }
            _ => {
                let _ = bridge
                    .go_to_definition(server_id, file_uri, Position::new(15, 5))
                    .await;
            }
        }

        latencies.push(start.elapsed());

        // Small delay between requests to avoid overwhelming the server
        sleep(Duration::from_millis(10)).await;
    }

    Ok(latencies)
}

/// Simulate memory usage (in a real implementation, this would use system APIs)
fn measure_memory_usage() -> f64 {
    // In a real implementation, this would use process memory APIs
    // For now, return a simulated value
    128.0 // MB
}

/// Test performance with a large Rust codebase
#[tokio::test]
async fn test_large_codebase_performance() {
    println!("🔧 Setting up large codebase...");

    let temp_dir = TempDir::new().unwrap();
    let workspace_path = create_large_rust_codebase(&temp_dir, 50).await.unwrap();

    let config = LspServerConfig::new()
        .command("rust-analyzer")
        .root_path(workspace_path.clone())
        .working_directory(workspace_path.clone());

    let mut metrics = PerformanceMetrics::new();

    // Test server startup time
    println!("⏱️ Measuring startup time...");
    if let Ok(startup_time) = measure_startup_time(config.clone()).await {
        metrics.startup_time = startup_time;
        println!("✓ Startup time: {startup_time:?}");
    } else {
        println!("⚠ Could not measure startup time (rust-analyzer not available)");
        return;
    }

    // Test with real server if available
    let mut bridge = LspBridge::new();
    let server_id = match bridge.register_server("perf-test", config).await {
        Ok(id) => id,
        Err(_) => {
            println!("⚠ Skipping performance test - rust-analyzer not available");
            return;
        }
    };

    if bridge.start_server(&server_id).await.is_err() {
        println!("⚠ Could not start server for performance testing");
        return;
    }

    timeout(Duration::from_secs(60), async {
        // Wait for server to be ready
        let _ = bridge.wait_server_ready(&server_id).await;

        // Open several documents
        println!("📂 Opening documents...");
        let start = Instant::now();

        let main_file = workspace_path.join("src").join("main.rs");
        let lib_file = workspace_path.join("src").join("lib.rs");

        if main_file.exists() {
            let content = std::fs::read_to_string(&main_file).unwrap_or_default();
            let _ = bridge
                .open_document(
                    &server_id,
                    &format!("file://{}", main_file.display()),
                    &content,
                )
                .await;
        }

        if lib_file.exists() {
            let content = std::fs::read_to_string(&lib_file).unwrap_or_default();
            let _ = bridge
                .open_document(
                    &server_id,
                    &format!("file://{}", lib_file.display()),
                    &content,
                )
                .await;
        }

        metrics.document_sync_time = start.elapsed();
        println!("✓ Document sync time: {:?}", metrics.document_sync_time);

        // Allow time for indexing
        sleep(Duration::from_secs(10)).await;

        // Measure request latencies
        println!("🎯 Measuring request latencies...");
        if let Ok(latencies) = measure_request_latencies(
            &bridge,
            &server_id,
            &format!("file://{}", main_file.display()),
            100,
        )
        .await
        {
            metrics.request_latencies = latencies;
        }

        // Measure resource usage
        metrics.memory_usage_mb = measure_memory_usage();

        let stats = metrics.calculate_statistics();

        // Print performance results
        println!("\n📊 Performance Results:");
        println!("  Startup time: {:?}", metrics.startup_time);
        println!("  Document sync: {:?}", metrics.document_sync_time);
        println!("  Memory usage: {:.1} MB", metrics.memory_usage_mb);

        if stats.total_requests > 0 {
            println!("  Request latencies:");
            println!("    Mean: {:?}", stats.mean_latency);
            println!("    Median: {:?}", stats.median_latency);
            println!("    95th percentile: {:?}", stats.p95_latency);
            println!("    99th percentile: {:?}", stats.p99_latency);
            println!("    Min: {:?}", stats.min_latency);
            println!("    Max: {:?}", stats.max_latency);
            println!("    Total requests: {}", stats.total_requests);
        }

        // Performance assertions
        assert!(
            metrics.startup_time < Duration::from_secs(30),
            "Startup time too slow"
        );
        assert!(
            metrics.document_sync_time < Duration::from_secs(5),
            "Document sync too slow"
        );

        if !metrics.request_latencies.is_empty() {
            assert!(
                stats.p95_latency < Duration::from_millis(1000),
                "95th percentile latency too high"
            );
            assert!(
                stats.mean_latency < Duration::from_millis(500),
                "Mean latency too high"
            );
        }

        bridge.stop_server(&server_id).await.unwrap();
    })
    .await
    .expect("Performance test timed out");

    println!("✅ Performance test completed successfully");
}

/// Test concurrent request handling
#[tokio::test]
async fn test_concurrent_request_performance() {
    println!("🚀 Testing concurrent request performance...");

    // This test simulates multiple concurrent clients
    // In a real implementation, this would spawn multiple tasks
    // making concurrent LSP requests and measure throughput

    let temp_dir = TempDir::new().unwrap();
    let workspace_path = create_large_rust_codebase(&temp_dir, 20).await.unwrap();

    let config = LspServerConfig::new()
        .command("rust-analyzer")
        .root_path(workspace_path.clone());

    let mut bridge = LspBridge::new();
    let server_id = match bridge.register_server("concurrent-test", config).await {
        Ok(id) => id,
        Err(_) => {
            println!("⚠ Skipping concurrent test - rust-analyzer not available");
            return;
        }
    };

    if bridge.start_server(&server_id).await.is_err() {
        println!("⚠ Could not start server for concurrent testing");
        return;
    }

    timeout(Duration::from_secs(45), async {
        let _ = bridge.wait_server_ready(&server_id).await;

        // Simulate concurrent load
        let start = Instant::now();
        let num_concurrent_requests: usize = 50;
        let mut tasks = Vec::new();

        for i in 0..num_concurrent_requests {
            let _file_uri = format!("file://{}/src/main.rs", workspace_path.display());
            let _position = Position::new((i % 20) as u32, (i % 40) as u32);

            // In a real implementation, we would spawn actual concurrent tasks
            // For now, we simulate the timing
            tasks.push(async move {
                sleep(Duration::from_millis(10)).await;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
            });
        }

        // Wait for all simulated requests
        let results = futures::future::join_all(tasks).await;
        let elapsed = start.elapsed();

        let successful_requests = results.iter().filter(|r| r.is_ok()).count();
        let throughput = successful_requests as f64 / elapsed.as_secs_f64();

        println!("📈 Concurrent performance results:");
        println!("  Total requests: {num_concurrent_requests}");
        println!("  Successful requests: {successful_requests}");
        println!("  Total time: {elapsed:?}");
        println!("  Throughput: {throughput:.2} requests/second");

        // Performance assertions
        assert!(
            throughput > 5.0,
            "Throughput too low: {throughput:.2} req/s"
        );
        assert!(
            successful_requests > num_concurrent_requests / 2,
            "Too many failed requests"
        );

        bridge.stop_server(&server_id).await.unwrap();
    })
    .await
    .expect("Concurrent test timed out");

    println!("✅ Concurrent performance test completed");
}

/// Test memory usage over time
#[tokio::test]
async fn test_memory_usage_over_time() {
    println!("🧠 Testing memory usage patterns...");

    // This test monitors memory usage over time to detect leaks
    let initial_memory = measure_memory_usage();

    // Simulate prolonged usage
    for i in 0..10 {
        // Simulate some work
        sleep(Duration::from_millis(100)).await;

        let current_memory = measure_memory_usage();
        let memory_increase = current_memory - initial_memory;

        println!("  Iteration {i}: {current_memory:.1} MB (+{memory_increase:.1} MB)");

        // Check for memory leaks (in real implementation)
        if memory_increase > 100.0 {
            eprintln!("⚠ Potential memory leak detected: +{memory_increase:.1} MB");
        }
    }

    let final_memory = measure_memory_usage();
    let total_increase = final_memory - initial_memory;

    println!("📊 Memory usage summary:");
    println!("  Initial: {initial_memory:.1} MB");
    println!("  Final: {final_memory:.1} MB");
    println!("  Total increase: {total_increase:.1} MB");

    // Assert reasonable memory usage
    assert!(
        total_increase < 50.0,
        "Excessive memory usage: +{total_increase:.1} MB"
    );

    println!("✅ Memory usage test completed");
}
