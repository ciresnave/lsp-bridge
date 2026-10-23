use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lsp_bridge::prelude::*;
use lsp_bridge::protocol::{LspRequest, RequestId};
use lsp_bridge::{LspError, Result};
use std::hint::black_box;
use std::time::Duration;

fn benchmark_message_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_serialization");

    for size in [100, 1000, 10000, 100000].iter() {
        let large_data = "x".repeat(*size);

        let request = LspMessage::Request(LspRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(1),
            method: "textDocument/completion".to_string(),
            params: Some(serde_json::json!({
                "textDocument": {
                    "uri": "file:///test.rs"
                },
                "position": {
                    "line": 0,
                    "character": 0
                },
                "context": {
                    "triggerKind": 1,
                    "triggerCharacter": ".",
                    "large_data": large_data
                }
            })),
        });

        group.bench_with_input(BenchmarkId::new("serialize", size), size, |b, _| {
            b.iter(|| {
                let _json = serde_json::to_string(black_box(&request)).unwrap();
            });
        });

        let json = serde_json::to_string(&request).unwrap();
        group.bench_with_input(BenchmarkId::new("deserialize", size), size, |b, _| {
            b.iter(|| {
                let _message: LspMessage = serde_json::from_str(black_box(&json)).unwrap();
            });
        });
    }

    group.finish();
}

fn benchmark_config_creation(c: &mut Criterion) {
    c.bench_function("config_creation", |b| {
        b.iter(|| {
            let config = LspServerConfig::new()
                .command(black_box("rust-analyzer"))
                .startup_timeout(black_box(Duration::from_secs(30)))
                .working_directory(black_box("/tmp"))
                .arg(black_box("--stdio"));
            black_box(config);
        });
    });
}

fn benchmark_server_creation(c: &mut Criterion) {
    c.bench_function("server_creation", |b| {
        b.iter(|| {
            let config = LspServerConfig::new().command(black_box("test_command"));

            let server = LspServer::new("test_server", black_box(config));
            black_box(server);
        });
    });
}

fn benchmark_bridge_operations(c: &mut Criterion) {
    c.bench_function("bridge_creation", |b| {
        b.iter(|| {
            let bridge = LspBridge::new();
            black_box(bridge);
        });
    });

    let mut group = c.benchmark_group("bridge_server_registration");

    for num_servers in [1, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("create_multiple_configs", num_servers),
            num_servers,
            |b, &num_servers| {
                b.iter(|| {
                    let mut configs = Vec::new();

                    for i in 0..num_servers {
                        let config = LspServerConfig::new().command(format!("test_command_{i}"));
                        configs.push(config);
                    }

                    black_box(configs);
                });
            },
        );
    }

    group.finish();
}

fn benchmark_protocol_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("protocol_parsing");

    // Test different message types
    let test_cases = vec![
        (
            "request",
            r#"{"jsonrpc":"2.0","id":1,"method":"textDocument/completion","params":{"textDocument":{"uri":"file:///test.rs"},"position":{"line":10,"character":5}}}"#,
        ),
        (
            "response",
            r#"{"jsonrpc":"2.0","id":1,"result":{"items":[{"label":"test","kind":1}]}}"#,
        ),
        (
            "notification",
            r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///test.rs","languageId":"rust","version":1,"text":"fn main() {}"}}}"#,
        ),
        (
            "error",
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid Request"}}"#,
        ),
    ];

    for (name, json) in test_cases {
        group.bench_function(name, |b| {
            b.iter(|| {
                let _message: LspMessage = serde_json::from_str(black_box(json)).unwrap();
            });
        });
    }

    group.finish();
}

fn benchmark_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");

    for num_objects in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("create_configs", num_objects),
            num_objects,
            |b, &num_objects| {
                b.iter(|| {
                    let mut configs = Vec::new();
                    for i in 0..num_objects {
                        let config = LspServerConfig::new().command(format!("command_{i}"));
                        configs.push(config);
                    }
                    black_box(configs);
                });
            },
        );
    }

    group.finish();
}

fn benchmark_error_handling(c: &mut Criterion) {
    c.bench_function("error_creation", |b| {
        b.iter(|| {
            let error = LspError::communication(black_box("Test error message"));
            black_box(error);
        });
    });

    c.bench_function("error_propagation", |b| {
        b.iter(|| {
            fn inner_function() -> Result<()> {
                Err(LspError::communication("Inner error").into())
            }

            fn outer_function() -> Result<()> {
                inner_function().map_err(|_| LspError::invalid_configuration("Outer error").into())
            }

            let result = outer_function();
            let _ = black_box(result);
        });
    });
}

fn benchmark_utils(c: &mut Criterion) {
    use lsp_bridge::utils::*;

    c.bench_function("uri_from_path", |b| {
        let path = std::path::Path::new("test.txt");
        b.iter(|| {
            let uri = uri::from_path(black_box(path));
            black_box(uri);
        });
    });

    c.bench_function("position_helpers", |b| {
        b.iter(|| {
            let pos = position::new(black_box(10), black_box(5));
            black_box(pos);
        });
    });
}

criterion_group!(
    benches,
    benchmark_message_serialization,
    benchmark_config_creation,
    benchmark_server_creation,
    benchmark_bridge_operations,
    benchmark_protocol_parsing,
    benchmark_memory_usage,
    benchmark_error_handling,
    benchmark_utils,
);

criterion_main!(benches);
