//! Error types and utilities for LSP Bridge.

use std::fmt;
use thiserror::Error;

/// Primary error type for the LSP Bridge
#[derive(Error, Debug)]
pub enum LspBridgeError {
    /// LSP-related errors (existing LspError variants)
    #[error("LSP error: {0}")]
    Lsp(#[from] LspError),

    /// Resource limit exceeded
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    /// Circuit breaker is open
    #[error("Circuit breaker open: {0}")]
    CircuitBreakerOpen(String),

    /// Input validation failed
    #[error("Input validation failed: {0}")]
    ValidationFailed(String),

    /// Security violation
    #[error("Security violation: {0}")]
    SecurityViolation(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Timeout error
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Generic error
    #[error("Error: {0}")]
    Generic(String),
}

/// Result type alias for the LSP Bridge
pub type Result<T> = std::result::Result<T, LspBridgeError>;

/// Comprehensive error types for LSP Bridge operations.
#[derive(Error, Debug)]
pub enum LspError {
    /// Server startup or initialization failed
    #[error("Server startup failed: {message}")]
    ServerStartup { message: String },

    /// Server communication error
    #[error("Server communication error: {message}")]
    Communication { message: String },

    /// Server crashed or became unresponsive
    #[error("Server crashed: {server_id}")]
    ServerCrash { server_id: String },

    /// Protocol version mismatch
    #[error("Protocol version mismatch: expected {expected}, got {actual}")]
    ProtocolMismatch { expected: String, actual: String },

    /// Invalid server configuration
    #[error("Invalid server configuration: {message}")]
    InvalidConfiguration { message: String },

    /// Request timeout
    #[error("Request timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    /// Invalid document URI
    #[error("Invalid document URI: {uri}")]
    InvalidUri { uri: String },

    /// Server not found
    #[error("Server not found: {server_id}")]
    ServerNotFound { server_id: String },

    /// Feature not supported by server
    #[error("Feature '{feature}' not supported by server {server_id}")]
    FeatureNotSupported { feature: String, server_id: String },

    /// JSON-RPC error
    #[error("JSON-RPC error: {message}")]
    JsonRpc { message: String },

    /// The server answered `ContentModified` (-32801): the document changed while the request
    /// ran, so the result is stale and the request should be retried.
    #[error("Content modified: {message}")]
    ContentModified { message: String },

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Custom error for extensibility
    #[error("Custom error: {message}")]
    Custom { message: String },
}

impl LspError {
    /// Create a new server startup error
    pub fn server_startup<S: Into<String>>(message: S) -> Self {
        Self::ServerStartup {
            message: message.into(),
        }
    }

    /// Create a new communication error
    pub fn communication<S: Into<String>>(message: S) -> Self {
        Self::Communication {
            message: message.into(),
        }
    }

    /// Create a new server crash error
    pub fn server_crash<S: Into<String>>(server_id: S) -> Self {
        Self::ServerCrash {
            server_id: server_id.into(),
        }
    }

    /// Create a new invalid configuration error
    pub fn invalid_configuration<S: Into<String>>(message: S) -> Self {
        Self::InvalidConfiguration {
            message: message.into(),
        }
    }

    /// Create a new timeout error
    pub fn timeout(timeout_ms: u64) -> Self {
        Self::Timeout { timeout_ms }
    }

    /// Create a new invalid URI error
    pub fn invalid_uri<S: Into<String>>(uri: S) -> Self {
        Self::InvalidUri { uri: uri.into() }
    }

    /// Create a new server not found error
    pub fn server_not_found<S: Into<String>>(server_id: S) -> Self {
        Self::ServerNotFound {
            server_id: server_id.into(),
        }
    }

    /// Create a new feature not supported error
    pub fn feature_not_supported<S: Into<String>>(feature: S, server_id: S) -> Self {
        Self::FeatureNotSupported {
            feature: feature.into(),
            server_id: server_id.into(),
        }
    }

    /// Create a new JSON-RPC error
    pub fn json_rpc<S: Into<String>>(message: S) -> Self {
        Self::JsonRpc {
            message: message.into(),
        }
    }

    /// Create a new content-modified error
    pub fn content_modified<S: Into<String>>(message: S) -> Self {
        Self::ContentModified {
            message: message.into(),
        }
    }

    /// Create a new protocol error
    pub fn protocol<S: Into<String>>(message: S) -> Self {
        Self::Communication {
            message: format!("Protocol error: {}", message.into()),
        }
    }

    /// Create a new custom error
    pub fn custom<S: Into<String>>(message: S) -> Self {
        Self::Custom {
            message: message.into(),
        }
    }

    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::ServerCrash { .. } => true,
            Self::Communication { .. } => true,
            Self::Timeout { .. } => true,
            Self::ServerStartup { .. } => false,
            Self::ProtocolMismatch { .. } => false,
            Self::InvalidConfiguration { .. } => false,
            Self::InvalidUri { .. } => false,
            Self::ServerNotFound { .. } => false,
            Self::FeatureNotSupported { .. } => false,
            Self::JsonRpc { .. } => false,
            Self::ContentModified { .. } => true,
            Self::Io(_) => true,
            Self::Serialization(_) => false,
            Self::Custom { .. } => false,
        }
    }

    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::ServerCrash { .. } => ErrorSeverity::Critical,
            Self::ServerStartup { .. } => ErrorSeverity::Critical,
            Self::ProtocolMismatch { .. } => ErrorSeverity::Critical,
            Self::Communication { .. } => ErrorSeverity::High,
            Self::Timeout { .. } => ErrorSeverity::Medium,
            Self::InvalidConfiguration { .. } => ErrorSeverity::High,
            Self::InvalidUri { .. } => ErrorSeverity::Medium,
            Self::ServerNotFound { .. } => ErrorSeverity::Medium,
            Self::FeatureNotSupported { .. } => ErrorSeverity::Low,
            Self::JsonRpc { .. } => ErrorSeverity::Medium,
            Self::ContentModified { .. } => ErrorSeverity::Low,
            Self::Io(_) => ErrorSeverity::Medium,
            Self::Serialization(_) => ErrorSeverity::Medium,
            Self::Custom { .. } => ErrorSeverity::Medium,
        }
    }
}

/// Error severity levels for logging and handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "LOW"),
            Self::Medium => write!(f, "MEDIUM"),
            Self::High => write!(f, "HIGH"),
            Self::Critical => write!(f, "CRITICAL"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = LspError::server_startup("Test message");
        assert!(matches!(error, LspError::ServerStartup { .. }));
        assert!(!error.is_recoverable());
        assert_eq!(error.severity(), ErrorSeverity::Critical);
    }

    #[test]
    fn test_error_recoverable() {
        let recoverable = LspError::server_crash("test-server");
        let non_recoverable = LspError::invalid_configuration("bad config");

        assert!(recoverable.is_recoverable());
        assert!(!non_recoverable.is_recoverable());
    }

    #[test]
    fn test_content_modified_is_recoverable() {
        let error = LspError::content_modified("document changed");
        assert!(error.is_recoverable());
        assert_eq!(error.severity(), ErrorSeverity::Low);
    }

    #[test]
    fn test_error_severity() {
        let critical = LspError::server_startup("startup failed");
        let medium = LspError::timeout(5000);
        let low = LspError::feature_not_supported("hover", "test-server");

        assert_eq!(critical.severity(), ErrorSeverity::Critical);
        assert_eq!(medium.severity(), ErrorSeverity::Medium);
        assert_eq!(low.severity(), ErrorSeverity::Low);
    }
}
