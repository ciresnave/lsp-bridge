//! Basic integration tests that don't require external LSP servers.
//!
//! These tests validate the core functionality of the bridge without
//! requiring rust-analyzer or other external dependencies.

use lsp_bridge::{LspBridge, LspServerConfig};
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;

/// Test basic bridge creation and configuration
#[tokio::test]
async fn test_bridge_basic_operations() {
    let mut bridge = LspBridge::new();

    // Test initial state
    assert_eq!(bridge.server_count(), 0);
    assert!(bridge.list_servers().is_empty());

    // Test server registration (without starting)
    let config = LspServerConfig::new()
        .command("mock-server")
        .root_path(PathBuf::from("/tmp/test"));

    let server_id = bridge.register_server("test-server", config).await.unwrap();
    assert_eq!(server_id, "test-server");
    assert_eq!(bridge.server_count(), 1);
    assert!(bridge.has_server(&server_id));
    assert_eq!(bridge.list_servers(), vec!["test-server"]);
}

/// Test server configuration validation
#[tokio::test]
async fn test_server_configuration_validation() {
    let mut bridge = LspBridge::new();

    // Test valid configuration
    let valid_config = LspServerConfig::new()
        .command("valid-server")
        .root_path(PathBuf::from("/tmp/test"))
        .startup_timeout(Duration::from_secs(10));

    let result = bridge.register_server("valid", valid_config).await;
    assert!(result.is_ok());

    // Test configuration with validation
    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().to_path_buf();

    let config_with_workspace = LspServerConfig::new()
        .command("server-with-workspace")
        .root_path(workspace_path)
        .working_directory(temp_dir.path());

    let result = bridge
        .register_server("workspace-server", config_with_workspace)
        .await;
    assert!(result.is_ok());
}

/// Test error handling for non-existent servers
#[tokio::test]
async fn test_error_handling() {
    let mut bridge = LspBridge::new();

    // Test starting non-existent server
    let result = bridge.start_server("non-existent").await;
    assert!(result.is_err());

    // Test operations on non-existent server
    let result = bridge.stop_server("non-existent").await;
    assert!(result.is_err());
}

/// Test multiple server registration
#[tokio::test]
async fn test_multiple_server_registration() {
    let mut bridge = LspBridge::new();
    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().to_path_buf();

    // Register multiple servers
    let configs = vec![
        ("server1", "cmd1"),
        ("server2", "cmd2"),
        ("server3", "cmd3"),
    ];

    let mut server_ids = Vec::new();
    for (name, cmd) in configs {
        let config = LspServerConfig::new()
            .command(cmd)
            .root_path(workspace_path.clone());

        let server_id = bridge.register_server(name, config).await.unwrap();
        server_ids.push(server_id);
    }

    // Verify all servers are registered
    assert_eq!(bridge.server_count(), 3);
    for server_id in &server_ids {
        assert!(bridge.has_server(server_id));
    }

    // Verify server list
    let mut listed_servers = bridge.list_servers();
    listed_servers.sort();
    server_ids.sort();
    assert_eq!(listed_servers, server_ids);
}

/// Test resource cleanup
#[tokio::test]
async fn test_resource_cleanup() {
    let mut bridge = LspBridge::new();
    let temp_dir = TempDir::new().unwrap();

    // Register a server
    let config = LspServerConfig::new()
        .command("cleanup-test-server")
        .root_path(temp_dir.path());

    let server_id = bridge
        .register_server("cleanup-test", config)
        .await
        .unwrap();
    assert_eq!(bridge.server_count(), 1);

    // Unregister the server - this should complete quickly
    let unregister_result =
        timeout(Duration::from_secs(5), bridge.unregister_server(&server_id)).await;

    match unregister_result {
        Ok(Ok(())) => {
            // Clean unregistration succeeded
            assert_eq!(bridge.server_count(), 0);
            assert!(!bridge.has_server(&server_id));
        }
        Ok(Err(e)) => {
            // Unregistration failed - this could be acceptable for non-existent servers
            // but the server should still be removed from the bridge
            eprintln!("Unregister failed with error: {e}");
            assert_eq!(bridge.server_count(), 0);
            assert!(!bridge.has_server(&server_id));
        }
        Err(_) => {
            // Timeout occurred - this is a test failure!
            panic!(
                "Unregister operation timed out after 5 seconds - this indicates a bug in the cleanup logic"
            );
        }
    }
}

/// Test configuration builder pattern
#[tokio::test]
async fn test_configuration_builder() {
    let temp_dir = TempDir::new().unwrap();

    // Test comprehensive configuration
    let config = LspServerConfig::new()
        .command("test-server")
        .args(vec!["--stdio".to_string(), "--verbose".to_string()])
        .root_path(temp_dir.path())
        .working_directory(temp_dir.path())
        .startup_timeout(Duration::from_secs(30))
        .request_timeout(Duration::from_secs(10));

    // Validate configuration
    assert!(config.validate().is_ok());

    let mut bridge = LspBridge::new();
    let result = bridge.register_server("builder-test", config).await;
    assert!(result.is_ok());
}

/// Test sequential operations (LspBridge requires exclusive access)
#[tokio::test]
async fn test_sequential_operations() {
    let mut bridge = LspBridge::new();
    let temp_dir = TempDir::new().unwrap();

    // Register multiple servers sequentially
    for i in 0..5 {
        let config = LspServerConfig::new()
            .command(format!("sequential-server-{i}"))
            .root_path(temp_dir.path());

        let server_id = format!("sequential-{i}");
        let result = bridge.register_server(&server_id, config).await;
        assert!(result.is_ok());
    }

    // Verify all servers are registered
    assert_eq!(bridge.server_count(), 5);

    // Verify each server exists
    for i in 0..5 {
        let server_id = format!("sequential-{i}");
        assert!(bridge.has_server(&server_id));
    }
}

/// Test timeout scenarios
#[tokio::test]
async fn test_timeout_handling() {
    let mut bridge = LspBridge::new();

    // Test with very short timeout to simulate timeout conditions
    let config = LspServerConfig::new()
        .command("non-existent-server-that-will-timeout")
        .startup_timeout(Duration::from_millis(1)); // Very short timeout

    let server_id = bridge
        .register_server("timeout-test", config)
        .await
        .unwrap();

    // Attempt to start should timeout/fail quickly
    let result = timeout(Duration::from_secs(5), bridge.start_server(&server_id)).await;

    // Should either timeout or fail due to non-existent server
    if let Ok(start_result) = result {
        assert!(start_result.is_err()); // Server start failed
    } // Timeout occurred (also acceptable)
}
