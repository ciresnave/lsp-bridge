use lsp_bridge::LspClientCapabilities;
use lsp_bridge::prelude::*;
use std::time::Duration;

#[test]
fn test_empty_command_config() {
    let config = LspServerConfig::new();
    assert!(config.validate().is_err());
}

#[test]
fn test_zero_timeout_config() {
    let config = LspServerConfig::new()
        .command("test")
        .startup_timeout(Duration::from_secs(0));
    assert!(config.validate().is_err());
}

#[test]
fn test_malformed_json_rpc() {
    // Test various malformed JSON-RPC messages
    let test_cases = vec![
        r#"{"jsonrpc": "1.0"}"#,               // Wrong version
        r#"{"id": 1}"#,                        // Missing jsonrpc
        r#"{"jsonrpc": "2.0", "method": ""}"#, // Empty method
        r#"{"jsonrpc": "2.0", "id": "not_a_number", "method": "test"}"#, // Invalid id type for some cases
    ];

    for case in test_cases {
        let result = serde_json::from_str::<LspMessage>(case);
        // Most should fail to parse or be invalid
        if let Ok(msg) = result {
            // If it parses, it should at least have valid jsonrpc field
            match msg {
                LspMessage::Request(req) => assert_eq!(req.jsonrpc, "2.0"),
                LspMessage::Response(resp) => assert_eq!(resp.jsonrpc, "2.0"),
                LspMessage::Notification(notif) => assert_eq!(notif.jsonrpc, "2.0"),
            }
        }
    }
}

#[test]
fn test_large_message_content() {
    // Test handling of very large message content
    let large_content = "x".repeat(1_000_000); // 1MB string

    let request = LspRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        method: "test/large".to_string(),
        params: Some(serde_json::json!({
            "content": large_content
        })),
    };

    // Should be able to serialize large messages
    let serialized = serde_json::to_string(&request).expect("Should serialize large message");
    assert!(serialized.len() > 1_000_000);

    // Should be able to deserialize back
    let deserialized: LspRequest = serde_json::from_str(&serialized).expect("Should deserialize");
    assert_eq!(deserialized.method, "test/large");
}

#[test]
fn test_concurrent_access() {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let bridge = Arc::new(Mutex::new(LspBridge::new()));
    let mut handles = vec![];

    // Test concurrent access to bridge
    for i in 0..10 {
        let bridge_clone = bridge.clone();
        let handle = thread::spawn(move || {
            let _guard = bridge_clone.lock().unwrap();
            // Just test that we can acquire the lock
            thread::sleep(Duration::from_millis(1));
            i
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread should complete");
    }
}

#[test]
fn test_request_id_types() {
    // Test different request ID types
    let string_id = RequestId::String("test-id".to_string());
    let number_id = RequestId::Number(42);

    // Test serialization round-trip
    let string_json = serde_json::to_string(&string_id).expect("Should serialize string ID");
    let number_json = serde_json::to_string(&number_id).expect("Should serialize number ID");

    let parsed_string: RequestId =
        serde_json::from_str(&string_json).expect("Should parse string ID");
    let parsed_number: RequestId =
        serde_json::from_str(&number_json).expect("Should parse number ID");

    assert_eq!(parsed_string, string_id);
    assert_eq!(parsed_number, number_id);
}

#[test]
fn test_error_response_handling() {
    let error_response = LspResponse {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        result: None,
        error: Some(LspResponseError {
            code: -32601,
            message: "Method not found".to_string(),
            data: None,
        }),
    };

    // Should serialize and deserialize properly
    let json = serde_json::to_string(&error_response).expect("Should serialize error response");
    let parsed: LspResponse = serde_json::from_str(&json).expect("Should parse error response");

    assert!(parsed.error.is_some());
    assert!(parsed.result.is_none());
    assert_eq!(parsed.error.unwrap().code, -32601);
}

#[test]
fn test_unicode_content() {
    // Test handling of Unicode content in messages
    let unicode_content = "Hello 世界 🚀 émoji test";

    let notification = LspNotification {
        jsonrpc: "2.0".to_string(),
        method: "test/unicode".to_string(),
        params: Some(serde_json::json!({
            "text": unicode_content
        })),
    };

    let json = serde_json::to_string(&notification).expect("Should serialize Unicode");
    let parsed: LspNotification = serde_json::from_str(&json).expect("Should parse Unicode");

    if let Some(params) = parsed.params {
        let text = params["text"].as_str().expect("Should have text field");
        assert_eq!(text, unicode_content);
    } else {
        panic!("Should have params");
    }
}

#[test]
fn test_capabilities_serialization() {
    let capabilities = LspClientCapabilities::default();

    // Should be able to serialize default capabilities
    let json = serde_json::to_string(&capabilities).expect("Should serialize capabilities");
    let parsed: LspClientCapabilities =
        serde_json::from_str(&json).expect("Should parse capabilities");

    // Default capabilities should round-trip correctly
    assert_eq!(
        serde_json::to_string(&capabilities).unwrap(),
        serde_json::to_string(&parsed).unwrap()
    );
}

#[test]
fn test_config_field_access() {
    let config = LspServerConfig::new()
        .command("test-server")
        .arg("--stdio")
        .arg("--verbose")
        .startup_timeout(Duration::from_secs(15))
        .request_timeout(Duration::from_secs(5));

    // Test field access
    assert_eq!(config.command, "test-server");
    assert_eq!(config.args, vec!["--stdio", "--verbose"]);
    assert_eq!(config.startup_timeout, Duration::from_secs(15));
    assert_eq!(config.request_timeout, Duration::from_secs(5));
}
