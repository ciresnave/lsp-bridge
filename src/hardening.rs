//! Production hardening and resource limits
//!
//! This module provides production-grade hardening features including
//! resource limits, rate limiting, input validation, and enhanced error recovery.

use crate::error::{LspBridgeError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use tokio::time::{sleep, timeout};

/// Production hardening configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HardeningConfig {
    /// Resource limits configuration
    pub resource_limits: ResourceLimits,
    /// Rate limiting configuration
    pub rate_limits: RateLimits,
    /// Security configuration
    pub security: SecurityConfig,
    /// Error recovery configuration
    pub error_recovery: ErrorRecoveryConfig,
    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
}

/// Resource limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory usage in MB
    pub max_memory_mb: usize,
    /// Maximum number of concurrent connections
    pub max_connections: usize,
    /// Maximum number of concurrent requests per connection
    pub max_requests_per_connection: usize,
    /// Maximum request size in bytes
    pub max_request_size_bytes: usize,
    /// Maximum response size in bytes
    pub max_response_size_bytes: usize,
    /// Request timeout duration
    pub request_timeout: Duration,
    /// Connection timeout duration
    pub connection_timeout: Duration,
    /// Maximum number of retries for failed operations
    pub max_retries: usize,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    /// Requests per second per connection
    pub requests_per_second: f64,
    /// Burst capacity for rate limiter
    pub burst_capacity: usize,
    /// Rate limit window duration
    pub window_duration: Duration,
    /// Enable adaptive rate limiting
    pub adaptive: bool,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable input validation
    pub enable_input_validation: bool,
    /// Enable output sanitization
    pub enable_output_sanitization: bool,
    /// Maximum depth for nested JSON structures
    pub max_json_depth: usize,
    /// Maximum string length in requests
    pub max_string_length: usize,
    /// Enable request logging for security audit
    pub enable_security_logging: bool,
}

/// Error recovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecoveryConfig {
    /// Enable automatic retry for transient errors
    pub enable_auto_retry: bool,
    /// Base delay for exponential backoff
    pub retry_base_delay: Duration,
    /// Maximum delay for exponential backoff
    pub retry_max_delay: Duration,
    /// Enable circuit breaker pattern
    pub enable_circuit_breaker: bool,
    /// Circuit breaker failure threshold
    pub circuit_breaker_threshold: usize,
    /// Circuit breaker timeout duration
    pub circuit_breaker_timeout: Duration,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable detailed metrics collection
    pub enable_detailed_metrics: bool,
    /// Metrics collection interval
    pub metrics_interval: Duration,
    /// Enable health checks
    pub enable_health_checks: bool,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Enable alerting
    pub enable_alerting: bool,
}

/// Production hardening manager
#[derive(Debug)]
pub struct HardeningManager {
    config: HardeningConfig,
    /// Connection semaphore for limiting concurrent connections
    connection_semaphore: Arc<Semaphore>,
    /// Request semaphores per connection
    request_semaphores: Arc<RwLock<HashMap<String, Arc<Semaphore>>>>,
    /// Rate limiters per connection
    rate_limiters: Arc<RwLock<HashMap<String, Arc<RateLimiter>>>>,
    /// Circuit breakers per service
    circuit_breakers: Arc<RwLock<HashMap<String, Arc<CircuitBreaker>>>>,
    /// Resource usage tracker
    resource_tracker: Arc<ResourceTracker>,
    /// Input validator
    input_validator: Arc<InputValidator>,
}

/// Rate limiter implementation using token bucket algorithm
#[derive(Debug)]
pub struct RateLimiter {
    capacity: usize,
    tokens: Arc<AtomicUsize>,
    refill_rate: f64,
    last_refill: Arc<RwLock<Instant>>,
}

/// Circuit breaker implementation
#[derive(Debug)]
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitBreakerState>>,
    failure_count: Arc<AtomicUsize>,
    failure_threshold: usize,
    timeout: Duration,
    last_failure: Arc<RwLock<Option<Instant>>>,
}

/// Circuit breaker states
#[derive(Debug, Clone)]
pub enum CircuitBreakerState {
    /// Circuit is closed, requests flow normally
    Closed,
    /// Circuit is open, requests are rejected
    Open,
    /// Circuit is half-open, testing if service has recovered
    HalfOpen,
}

/// Resource usage tracker
#[derive(Debug)]
pub struct ResourceTracker {
    memory_usage: Arc<AtomicUsize>,
    connection_count: Arc<AtomicUsize>,
    request_count: Arc<AtomicUsize>,
    start_time: Instant,
}

/// Input validator for security hardening
#[derive(Debug)]
pub struct InputValidator {
    config: SecurityConfig,
}

/// Validation result
#[derive(Debug)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub sanitized_input: Option<String>,
}

impl HardeningManager {
    /// Create a new hardening manager with the given configuration
    pub fn new(config: HardeningConfig) -> Self {
        let connection_semaphore = Arc::new(Semaphore::new(config.resource_limits.max_connections));

        Self {
            connection_semaphore,
            request_semaphores: Arc::new(RwLock::new(HashMap::new())),
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            resource_tracker: Arc::new(ResourceTracker::new()),
            input_validator: Arc::new(InputValidator::new(config.security.clone())),
            config,
        }
    }

    /// Acquire a connection permit (blocks if limit exceeded)
    pub async fn acquire_connection_permit(&self) -> Result<ConnectionPermit> {
        let permit = timeout(
            self.config.resource_limits.connection_timeout,
            self.connection_semaphore.clone().acquire_owned(),
        )
        .await
        .map_err(|_| LspBridgeError::ResourceLimitExceeded("Connection timeout".to_string()))?
        .map_err(|_| {
            LspBridgeError::ResourceLimitExceeded("Connection semaphore closed".to_string())
        })?;

        self.resource_tracker.increment_connections();

        Ok(ConnectionPermit {
            _permit: permit,
            tracker: Arc::clone(&self.resource_tracker),
        })
    }

    /// Acquire a request permit for a specific connection
    pub async fn acquire_request_permit(&self, connection_id: &str) -> Result<RequestPermit> {
        // Get or create request semaphore for this connection
        let semaphore = {
            let mut semaphores = self.request_semaphores.write().await;
            let entry = semaphores.entry(connection_id.to_string());
            entry
                .or_insert_with(|| {
                    Arc::new(Semaphore::new(
                        self.config.resource_limits.max_requests_per_connection,
                    ))
                })
                .clone()
        };

        let permit = timeout(
            self.config.resource_limits.request_timeout,
            semaphore.acquire_owned(),
        )
        .await
        .map_err(|_| LspBridgeError::ResourceLimitExceeded("Request timeout".to_string()))?
        .map_err(|_| {
            LspBridgeError::ResourceLimitExceeded("Request semaphore closed".to_string())
        })?;

        self.resource_tracker.increment_requests();

        Ok(RequestPermit {
            _permit: permit,
            tracker: Arc::clone(&self.resource_tracker),
        })
    }

    /// Check rate limit for a connection
    pub async fn check_rate_limit(&self, connection_id: &str) -> Result<()> {
        let rate_limiter = {
            let mut limiters = self.rate_limiters.write().await;
            let entry = limiters.entry(connection_id.to_string());
            entry
                .or_insert_with(|| {
                    Arc::new(RateLimiter::new(
                        self.config.rate_limits.requests_per_second,
                        self.config.rate_limits.burst_capacity,
                    ))
                })
                .clone()
        };

        if !rate_limiter.allow_request().await {
            return Err(LspBridgeError::RateLimitExceeded(
                "Too many requests".to_string(),
            ));
        }

        Ok(())
    }

    /// Check circuit breaker for a service
    pub async fn check_circuit_breaker(&self, service_name: &str) -> Result<()> {
        if !self.config.error_recovery.enable_circuit_breaker {
            return Ok(());
        }

        let circuit_breaker = {
            let mut breakers = self.circuit_breakers.write().await;
            let entry = breakers.entry(service_name.to_string());
            entry
                .or_insert_with(|| {
                    Arc::new(CircuitBreaker::new(
                        self.config.error_recovery.circuit_breaker_threshold,
                        self.config.error_recovery.circuit_breaker_timeout,
                    ))
                })
                .clone()
        };

        circuit_breaker.check_request().await
    }

    /// Record a successful operation for circuit breaker
    pub async fn record_success(&self, service_name: &str) {
        if let Some(circuit_breaker) = self.circuit_breakers.read().await.get(service_name) {
            circuit_breaker.record_success().await;
        }
    }

    /// Record a failed operation for circuit breaker
    pub async fn record_failure(&self, service_name: &str) {
        if let Some(circuit_breaker) = self.circuit_breakers.read().await.get(service_name) {
            circuit_breaker.record_failure().await;
        }
    }

    /// Validate input data
    pub async fn validate_input(&self, input: &str) -> ValidationResult {
        self.input_validator.validate(input).await
    }

    /// Execute operation with automatic retry and circuit breaker
    pub async fn execute_with_retry<F, Fut, T>(&self, service_name: &str, operation: F) -> Result<T>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<T>> + Send,
        T: Send,
    {
        self.check_circuit_breaker(service_name).await?;

        let mut last_error = None;
        let mut delay = self.config.error_recovery.retry_base_delay;

        for attempt in 0..=self.config.resource_limits.max_retries {
            match operation().await {
                Ok(result) => {
                    self.record_success(service_name).await;
                    return Ok(result);
                }
                Err(error) => {
                    last_error = Some(error);

                    if attempt < self.config.resource_limits.max_retries {
                        // Exponential backoff with jitter
                        let jitter = Duration::from_millis((rand::random::<f64>() * 100.0) as u64);
                        sleep(delay + jitter).await;
                        delay =
                            std::cmp::min(delay * 2, self.config.error_recovery.retry_max_delay);
                    }
                }
            }
        }

        self.record_failure(service_name).await;
        Err(last_error.unwrap())
    }

    /// Get current resource usage
    pub fn get_resource_usage(&self) -> ResourceUsage {
        self.resource_tracker.get_usage()
    }

    /// Check if memory usage is within limits
    pub fn check_memory_limit(&self, additional_mb: usize) -> bool {
        let current_usage = self.resource_tracker.memory_usage.load(Ordering::Relaxed);
        current_usage + additional_mb <= self.config.resource_limits.max_memory_mb
    }

    /// Update memory usage
    pub fn update_memory_usage(&self, usage_mb: usize) {
        self.resource_tracker
            .memory_usage
            .store(usage_mb, Ordering::Relaxed);
    }
}

/// Connection permit that automatically releases when dropped
pub struct ConnectionPermit {
    _permit: tokio::sync::OwnedSemaphorePermit,
    tracker: Arc<ResourceTracker>,
}

impl Drop for ConnectionPermit {
    fn drop(&mut self) {
        self.tracker.decrement_connections();
    }
}

/// Request permit that automatically releases when dropped
pub struct RequestPermit {
    _permit: tokio::sync::OwnedSemaphorePermit,
    tracker: Arc<ResourceTracker>,
}

impl Drop for RequestPermit {
    fn drop(&mut self) {
        self.tracker.decrement_requests();
    }
}

/// Current resource usage snapshot
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub memory_mb: usize,
    pub connection_count: usize,
    pub request_count: usize,
    pub uptime: Duration,
}

impl RateLimiter {
    fn new(requests_per_second: f64, capacity: usize) -> Self {
        Self {
            capacity,
            tokens: Arc::new(AtomicUsize::new(capacity)),
            refill_rate: requests_per_second,
            last_refill: Arc::new(RwLock::new(Instant::now())),
        }
    }

    async fn allow_request(&self) -> bool {
        self.refill_tokens().await;

        let current_tokens = self.tokens.load(Ordering::Relaxed);
        if current_tokens > 0 {
            self.tokens.fetch_sub(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    async fn refill_tokens(&self) {
        let now = Instant::now();
        let mut last_refill = self.last_refill.write().await;

        let elapsed = now.duration_since(*last_refill);
        let tokens_to_add = (elapsed.as_secs_f64() * self.refill_rate) as usize;

        if tokens_to_add > 0 {
            let current_tokens = self.tokens.load(Ordering::Relaxed);
            let new_tokens = std::cmp::min(current_tokens + tokens_to_add, self.capacity);
            self.tokens.store(new_tokens, Ordering::Relaxed);
            *last_refill = now;
        }
    }
}

impl CircuitBreaker {
    fn new(failure_threshold: usize, timeout: Duration) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitBreakerState::Closed)),
            failure_count: Arc::new(AtomicUsize::new(0)),
            failure_threshold,
            timeout,
            last_failure: Arc::new(RwLock::new(None)),
        }
    }

    async fn check_request(&self) -> Result<()> {
        let state = self.state.read().await.clone();

        match state {
            CircuitBreakerState::Closed => Ok(()),
            CircuitBreakerState::Open => {
                // Check if timeout has passed
                let last_failure = self.last_failure.read().await;
                if let Some(last_fail_time) = *last_failure {
                    if last_fail_time.elapsed() >= self.timeout {
                        // Transition to half-open
                        drop(last_failure);
                        *self.state.write().await = CircuitBreakerState::HalfOpen;
                        Ok(())
                    } else {
                        Err(LspBridgeError::CircuitBreakerOpen(
                            "Service is temporarily unavailable".to_string(),
                        ))
                    }
                } else {
                    Ok(())
                }
            }
            CircuitBreakerState::HalfOpen => Ok(()),
        }
    }

    async fn record_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        *self.state.write().await = CircuitBreakerState::Closed;
    }

    async fn record_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        *self.last_failure.write().await = Some(Instant::now());

        if failures >= self.failure_threshold {
            *self.state.write().await = CircuitBreakerState::Open;
        }
    }
}

impl ResourceTracker {
    fn new() -> Self {
        Self {
            memory_usage: Arc::new(AtomicUsize::new(0)),
            connection_count: Arc::new(AtomicUsize::new(0)),
            request_count: Arc::new(AtomicUsize::new(0)),
            start_time: Instant::now(),
        }
    }

    fn increment_connections(&self) {
        self.connection_count.fetch_add(1, Ordering::Relaxed);
    }

    fn decrement_connections(&self) {
        self.connection_count.fetch_sub(1, Ordering::Relaxed);
    }

    fn increment_requests(&self) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
    }

    fn decrement_requests(&self) {
        self.request_count.fetch_sub(1, Ordering::Relaxed);
    }

    fn get_usage(&self) -> ResourceUsage {
        ResourceUsage {
            memory_mb: self.memory_usage.load(Ordering::Relaxed),
            connection_count: self.connection_count.load(Ordering::Relaxed),
            request_count: self.request_count.load(Ordering::Relaxed),
            uptime: self.start_time.elapsed(),
        }
    }
}

impl InputValidator {
    fn new(config: SecurityConfig) -> Self {
        Self { config }
    }

    async fn validate(&self, input: &str) -> ValidationResult {
        let mut errors = Vec::new();
        let mut is_valid = true;

        if !self.config.enable_input_validation {
            return ValidationResult {
                is_valid: true,
                errors: Vec::new(),
                sanitized_input: None,
            };
        }

        // Check string length
        if input.len() > self.config.max_string_length {
            errors.push(format!(
                "Input exceeds maximum length of {} characters",
                self.config.max_string_length
            ));
            is_valid = false;
        }

        // Validate JSON structure if applicable
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(input)
            && Self::get_json_depth(&json_value) > self.config.max_json_depth
        {
            errors.push(format!(
                "JSON structure exceeds maximum depth of {}",
                self.config.max_json_depth
            ));
            is_valid = false;
        }

        // Basic sanitization
        let sanitized = if self.config.enable_output_sanitization {
            Some(self.sanitize_string(input))
        } else {
            None
        };

        ValidationResult {
            is_valid,
            errors,
            sanitized_input: sanitized,
        }
    }

    fn get_json_depth(value: &serde_json::Value) -> usize {
        match value {
            serde_json::Value::Object(map) => {
                1 + map.values().map(Self::get_json_depth).max().unwrap_or(0)
            }
            serde_json::Value::Array(arr) => {
                1 + arr.iter().map(Self::get_json_depth).max().unwrap_or(0)
            }
            _ => 0,
        }
    }

    fn sanitize_string(&self, input: &str) -> String {
        // Basic sanitization - remove control characters
        input
            .chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
            .collect()
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: 1024, // 1GB
            max_connections: 100,
            max_requests_per_connection: 50,
            max_request_size_bytes: 10 * 1024 * 1024, // 10MB
            max_response_size_bytes: 10 * 1024 * 1024, // 10MB
            request_timeout: Duration::from_secs(30),
            connection_timeout: Duration::from_secs(10),
            max_retries: 3,
        }
    }
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            requests_per_second: 100.0,
            burst_capacity: 200,
            window_duration: Duration::from_secs(1),
            adaptive: true,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_input_validation: true,
            enable_output_sanitization: true,
            max_json_depth: 10,
            max_string_length: 1024 * 1024, // 1MB
            enable_security_logging: true,
        }
    }
}

impl Default for ErrorRecoveryConfig {
    fn default() -> Self {
        Self {
            enable_auto_retry: true,
            retry_base_delay: Duration::from_millis(100),
            retry_max_delay: Duration::from_secs(30),
            enable_circuit_breaker: true,
            circuit_breaker_threshold: 5,
            circuit_breaker_timeout: Duration::from_secs(60),
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_detailed_metrics: true,
            metrics_interval: Duration::from_secs(30),
            enable_health_checks: true,
            health_check_interval: Duration::from_secs(60),
            enable_alerting: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_limits() {
        let config = HardeningConfig {
            resource_limits: ResourceLimits {
                max_connections: 2,
                ..Default::default()
            },
            ..Default::default()
        };

        let manager = HardeningManager::new(config);

        // Acquire first connection
        let _permit1 = manager.acquire_connection_permit().await.unwrap();

        // Acquire second connection
        let _permit2 = manager.acquire_connection_permit().await.unwrap();

        // Third connection should timeout quickly
        let config_quick_timeout = HardeningConfig {
            resource_limits: ResourceLimits {
                max_connections: 2,
                connection_timeout: Duration::from_millis(100),
                ..Default::default()
            },
            ..Default::default()
        };

        let manager_quick = HardeningManager::new(config_quick_timeout);
        let _permit1_quick = manager_quick.acquire_connection_permit().await.unwrap();
        let _permit2_quick = manager_quick.acquire_connection_permit().await.unwrap();

        // This should fail due to timeout
        let result = manager_quick.acquire_connection_permit().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let rate_limiter = RateLimiter::new(2.0, 2); // 2 requests per second, burst of 2

        // First two requests should succeed (burst)
        assert!(rate_limiter.allow_request().await);
        assert!(rate_limiter.allow_request().await);

        // Third request should fail (rate limited)
        assert!(!rate_limiter.allow_request().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let circuit_breaker = CircuitBreaker::new(2, Duration::from_millis(100));

        // Normal operation
        assert!(circuit_breaker.check_request().await.is_ok());

        // Record failures
        circuit_breaker.record_failure().await;
        circuit_breaker.record_failure().await;

        // Circuit should be open now
        assert!(circuit_breaker.check_request().await.is_err());

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should be half-open now
        assert!(circuit_breaker.check_request().await.is_ok());

        // Record success to close circuit
        circuit_breaker.record_success().await;
        assert!(circuit_breaker.check_request().await.is_ok());
    }

    #[tokio::test]
    async fn test_input_validation() {
        let config = SecurityConfig {
            enable_input_validation: true,
            max_string_length: 10,
            max_json_depth: 2,
            ..Default::default()
        };

        let validator = InputValidator::new(config);

        // Valid input
        let result = validator.validate("hello").await;
        assert!(result.is_valid);

        // Too long input
        let result = validator.validate("this is too long").await;
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }
}
