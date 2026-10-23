//! Production monitoring and metrics collection
//!
//! This module provides comprehensive monitoring capabilities for the LSP Bridge,
//! including metrics collection, resource usage tracking, health checks, and alerting.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Comprehensive metrics collector for LSP Bridge operations
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    /// Request counters by method
    request_counts: Arc<RwLock<HashMap<String, AtomicU64>>>,
    /// Response time tracking
    response_times: Arc<RwLock<HashMap<String, Vec<Duration>>>>,
    /// Error counters by type
    error_counts: Arc<RwLock<HashMap<String, AtomicU64>>>,
    /// Resource usage tracking
    resource_usage: Arc<ResourceUsageTracker>,
    /// Connection metrics
    connection_metrics: Arc<ConnectionMetrics>,
    /// Health status
    health_status: Arc<RwLock<HealthStatus>>,
}

/// Resource usage tracking
#[derive(Debug)]
pub struct ResourceUsageTracker {
    /// Current memory usage estimate (MB)
    memory_usage_mb: AtomicUsize,
    /// Peak memory usage (MB)
    peak_memory_mb: AtomicUsize,
    /// CPU usage samples
    cpu_samples: RwLock<Vec<f64>>,
    /// Active request count
    active_requests: AtomicUsize,
    /// Peak concurrent requests
    peak_concurrent_requests: AtomicUsize,
}

/// Connection-related metrics
#[derive(Debug)]
pub struct ConnectionMetrics {
    /// Total connections established
    total_connections: AtomicU64,
    /// Currently active connections
    active_connections: AtomicUsize,
    /// Peak concurrent connections
    peak_connections: AtomicUsize,
    /// Connection errors
    connection_errors: AtomicU64,
    /// Average connection duration
    connection_durations: RwLock<Vec<Duration>>,
}

/// Overall health status of the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Overall health score (0-100)
    pub health_score: u8,
    /// Individual component statuses
    pub components: HashMap<String, ComponentHealth>,
    /// Last health check timestamp
    pub last_check: SystemTime,
    /// Any active alerts
    pub alerts: Vec<HealthAlert>,
}

/// Health status of individual components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component status
    pub status: ComponentStatus,
    /// Optional status message
    pub message: Option<String>,
    /// Last update timestamp
    pub last_update: SystemTime,
    /// Performance metrics
    pub metrics: HashMap<String, f64>,
}

/// Component status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentStatus {
    /// Component is healthy
    Healthy,
    /// Component has warnings but is functional
    Warning,
    /// Component is degraded but operational
    Degraded,
    /// Component is critical/failing
    Critical,
    /// Component is unavailable
    Unavailable,
}

/// Health alert information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAlert {
    /// Alert severity
    pub severity: AlertSeverity,
    /// Alert message
    pub message: String,
    /// Component that triggered the alert
    pub component: String,
    /// Alert timestamp
    pub timestamp: SystemTime,
    /// Alert ID for tracking
    pub id: String,
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Informational alert
    Info,
    /// Warning level alert
    Warning,
    /// Error level alert
    Error,
    /// Critical alert requiring immediate attention
    Critical,
}

/// Metrics snapshot for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Timestamp of the snapshot
    pub timestamp: SystemTime,
    /// Request metrics
    pub requests: RequestMetrics,
    /// Resource metrics
    pub resources: ResourceMetrics,
    /// Connection metrics
    pub connections: ConnectionMetricsSnapshot,
    /// Health status
    pub health: HealthStatus,
}

/// Request-related metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMetrics {
    /// Total requests processed
    pub total_requests: u64,
    /// Requests per second (average)
    pub requests_per_second: f64,
    /// Average response time
    pub avg_response_time_ms: f64,
    /// 95th percentile response time
    pub p95_response_time_ms: f64,
    /// Error rate percentage
    pub error_rate: f64,
    /// Most frequent request types
    pub top_request_types: Vec<(String, u64)>,
}

/// Resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    /// Current memory usage (MB)
    pub current_memory_mb: usize,
    /// Peak memory usage (MB)
    pub peak_memory_mb: usize,
    /// Average CPU usage percentage
    pub avg_cpu_usage: f64,
    /// Active requests
    pub active_requests: usize,
    /// Peak concurrent requests
    pub peak_concurrent_requests: usize,
}

/// Connection metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMetricsSnapshot {
    /// Total connections established
    pub total_connections: u64,
    /// Currently active connections
    pub active_connections: usize,
    /// Peak concurrent connections
    pub peak_connections: usize,
    /// Connection error rate
    pub connection_error_rate: f64,
    /// Average connection duration
    pub avg_connection_duration_ms: f64,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            request_counts: Arc::new(RwLock::new(HashMap::new())),
            response_times: Arc::new(RwLock::new(HashMap::new())),
            error_counts: Arc::new(RwLock::new(HashMap::new())),
            resource_usage: Arc::new(ResourceUsageTracker::new()),
            connection_metrics: Arc::new(ConnectionMetrics::new()),
            health_status: Arc::new(RwLock::new(HealthStatus::new())),
        }
    }

    /// Record a request
    pub async fn record_request(&self, method: &str, response_time: Duration) {
        // Update request counts
        {
            let mut counts = self.request_counts.write().await;
            let counter = counts
                .entry(method.to_string())
                .or_insert_with(|| AtomicU64::new(0));
            counter.fetch_add(1, Ordering::Relaxed);
        }

        // Record response time
        {
            let mut times = self.response_times.write().await;
            let method_times = times.entry(method.to_string()).or_insert_with(Vec::new);
            method_times.push(response_time);

            // Keep only recent samples (last 1000)
            if method_times.len() > 1000 {
                method_times.drain(..method_times.len() - 1000);
            }
        }
    }

    /// Record an error
    pub async fn record_error(&self, error_type: &str) {
        let mut counts = self.error_counts.write().await;
        let counter = counts
            .entry(error_type.to_string())
            .or_insert_with(|| AtomicU64::new(0));
        counter.fetch_add(1, Ordering::Relaxed);
    }

    /// Update resource usage
    pub fn update_memory_usage(&self, current_mb: usize) {
        self.resource_usage
            .memory_usage_mb
            .store(current_mb, Ordering::Relaxed);

        // Update peak if necessary
        let current_peak = self.resource_usage.peak_memory_mb.load(Ordering::Relaxed);
        if current_mb > current_peak {
            self.resource_usage
                .peak_memory_mb
                .store(current_mb, Ordering::Relaxed);
        }
    }

    /// Record CPU usage sample
    pub async fn record_cpu_usage(&self, cpu_percent: f64) {
        let mut samples = self.resource_usage.cpu_samples.write().await;
        samples.push(cpu_percent);

        // Keep only recent samples (last 100)
        let len = samples.len();
        if len > 100 {
            samples.drain(..len - 100);
        }
    }

    /// Track active request start
    pub fn start_request(&self) {
        let current = self
            .resource_usage
            .active_requests
            .fetch_add(1, Ordering::Relaxed)
            + 1;

        // Update peak if necessary
        let current_peak = self
            .resource_usage
            .peak_concurrent_requests
            .load(Ordering::Relaxed);
        if current > current_peak {
            self.resource_usage
                .peak_concurrent_requests
                .store(current, Ordering::Relaxed);
        }
    }

    /// Track active request completion
    pub fn complete_request(&self) {
        self.resource_usage
            .active_requests
            .fetch_sub(1, Ordering::Relaxed);
    }

    /// Record new connection
    pub fn record_connection(&self) {
        self.connection_metrics
            .total_connections
            .fetch_add(1, Ordering::Relaxed);
        let current = self
            .connection_metrics
            .active_connections
            .fetch_add(1, Ordering::Relaxed)
            + 1;

        // Update peak if necessary
        let current_peak = self
            .connection_metrics
            .peak_connections
            .load(Ordering::Relaxed);
        if current > current_peak {
            self.connection_metrics
                .peak_connections
                .store(current, Ordering::Relaxed);
        }
    }

    /// Record connection closure
    pub fn record_connection_close(&self, duration: Duration) {
        self.connection_metrics
            .active_connections
            .fetch_sub(1, Ordering::Relaxed);

        // Record duration - spawn task to avoid blocking
        let duration_holder = Arc::clone(&self.connection_metrics);
        tokio::spawn(async move {
            let mut durations_guard = duration_holder.connection_durations.write().await;
            durations_guard.push(duration);

            // Keep only recent samples (last 1000)
            let len = durations_guard.len();
            if len > 1000 {
                durations_guard.drain(..len - 1000);
            }
        });
    }

    /// Record connection error
    pub fn record_connection_error(&self) {
        self.connection_metrics
            .connection_errors
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Get current metrics snapshot
    pub async fn get_snapshot(&self) -> MetricsSnapshot {
        let request_metrics = self.calculate_request_metrics().await;
        let resource_metrics = self.calculate_resource_metrics().await;
        let connection_metrics = self.calculate_connection_metrics().await;
        let health_status = self.health_status.read().await.clone();

        MetricsSnapshot {
            timestamp: SystemTime::now(),
            requests: request_metrics,
            resources: resource_metrics,
            connections: connection_metrics,
            health: health_status,
        }
    }

    /// Perform health check and update status
    pub async fn perform_health_check(&self) -> HealthStatus {
        let mut health = HealthStatus::new();

        // Check various components
        health
            .components
            .insert("memory".to_string(), self.check_memory_health().await);
        health.components.insert(
            "connections".to_string(),
            self.check_connection_health().await,
        );
        health
            .components
            .insert("requests".to_string(), self.check_request_health().await);
        health
            .components
            .insert("errors".to_string(), self.check_error_health().await);

        // Calculate overall health score
        health.health_score = self.calculate_health_score(&health.components);

        // Generate alerts based on health status
        health.alerts = self.generate_health_alerts(&health.components).await;

        // Update stored health status
        *self.health_status.write().await = health.clone();

        health
    }

    async fn calculate_request_metrics(&self) -> RequestMetrics {
        let counts = self.request_counts.read().await;
        let total_requests: u64 = counts.values().map(|c| c.load(Ordering::Relaxed)).sum();

        let times = self.response_times.read().await;
        let all_times: Vec<Duration> = times.values().flatten().cloned().collect();

        let avg_response_time = if all_times.is_empty() {
            0.0
        } else {
            all_times.iter().map(|d| d.as_millis() as f64).sum::<f64>() / all_times.len() as f64
        };

        let p95_response_time = if all_times.is_empty() {
            0.0
        } else {
            let mut sorted_times = all_times.clone();
            sorted_times.sort();
            let p95_index = (sorted_times.len() as f64 * 0.95) as usize;
            sorted_times
                .get(p95_index.min(sorted_times.len() - 1))
                .unwrap_or(&Duration::from_millis(0))
                .as_millis() as f64
        };

        let error_counts = self.error_counts.read().await;
        let total_errors: u64 = error_counts
            .values()
            .map(|c| c.load(Ordering::Relaxed))
            .sum();
        let error_rate = if total_requests > 0 {
            (total_errors as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };

        let top_request_types: Vec<(String, u64)> = counts
            .iter()
            .map(|(method, count)| (method.clone(), count.load(Ordering::Relaxed)))
            .collect();

        RequestMetrics {
            total_requests,
            requests_per_second: 0.0, // Would need time window tracking
            avg_response_time_ms: avg_response_time,
            p95_response_time_ms: p95_response_time,
            error_rate,
            top_request_types,
        }
    }

    async fn calculate_resource_metrics(&self) -> ResourceMetrics {
        let cpu_samples = self.resource_usage.cpu_samples.read().await;
        let avg_cpu_usage = if cpu_samples.is_empty() {
            0.0
        } else {
            cpu_samples.iter().sum::<f64>() / cpu_samples.len() as f64
        };

        ResourceMetrics {
            current_memory_mb: self.resource_usage.memory_usage_mb.load(Ordering::Relaxed),
            peak_memory_mb: self.resource_usage.peak_memory_mb.load(Ordering::Relaxed),
            avg_cpu_usage,
            active_requests: self.resource_usage.active_requests.load(Ordering::Relaxed),
            peak_concurrent_requests: self
                .resource_usage
                .peak_concurrent_requests
                .load(Ordering::Relaxed),
        }
    }

    async fn calculate_connection_metrics(&self) -> ConnectionMetricsSnapshot {
        let durations = self.connection_metrics.connection_durations.read().await;
        let avg_duration = if durations.is_empty() {
            0.0
        } else {
            durations.iter().map(|d| d.as_millis() as f64).sum::<f64>() / durations.len() as f64
        };

        let total_connections = self
            .connection_metrics
            .total_connections
            .load(Ordering::Relaxed);
        let connection_errors = self
            .connection_metrics
            .connection_errors
            .load(Ordering::Relaxed);
        let error_rate = if total_connections > 0 {
            (connection_errors as f64 / total_connections as f64) * 100.0
        } else {
            0.0
        };

        ConnectionMetricsSnapshot {
            total_connections,
            active_connections: self
                .connection_metrics
                .active_connections
                .load(Ordering::Relaxed),
            peak_connections: self
                .connection_metrics
                .peak_connections
                .load(Ordering::Relaxed),
            connection_error_rate: error_rate,
            avg_connection_duration_ms: avg_duration,
        }
    }

    async fn check_memory_health(&self) -> ComponentHealth {
        let current_mb = self.resource_usage.memory_usage_mb.load(Ordering::Relaxed);
        let peak_mb = self.resource_usage.peak_memory_mb.load(Ordering::Relaxed);

        let status = if current_mb > 1000 {
            // > 1GB
            ComponentStatus::Critical
        } else if current_mb > 500 {
            // > 500MB
            ComponentStatus::Warning
        } else {
            ComponentStatus::Healthy
        };

        let message = if current_mb > 1000 {
            Some("High memory usage detected".to_string())
        } else {
            None
        };

        let mut metrics = HashMap::new();
        metrics.insert("current_mb".to_string(), current_mb as f64);
        metrics.insert("peak_mb".to_string(), peak_mb as f64);

        ComponentHealth {
            status,
            message,
            last_update: SystemTime::now(),
            metrics,
        }
    }

    async fn check_connection_health(&self) -> ComponentHealth {
        let active = self
            .connection_metrics
            .active_connections
            .load(Ordering::Relaxed);
        let peak = self
            .connection_metrics
            .peak_connections
            .load(Ordering::Relaxed);
        let errors = self
            .connection_metrics
            .connection_errors
            .load(Ordering::Relaxed);
        let total = self
            .connection_metrics
            .total_connections
            .load(Ordering::Relaxed);

        let error_rate = if total > 0 {
            (errors as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let status = if error_rate > 10.0 {
            ComponentStatus::Critical
        } else if error_rate > 5.0 || active > 100 {
            ComponentStatus::Warning
        } else {
            ComponentStatus::Healthy
        };

        let message = if error_rate > 10.0 {
            Some(format!("High connection error rate: {error_rate:.1}%"))
        } else if active > 100 {
            Some("High number of active connections".to_string())
        } else {
            None
        };

        let mut metrics = HashMap::new();
        metrics.insert("active_connections".to_string(), active as f64);
        metrics.insert("peak_connections".to_string(), peak as f64);
        metrics.insert("error_rate".to_string(), error_rate);

        ComponentHealth {
            status,
            message,
            last_update: SystemTime::now(),
            metrics,
        }
    }

    async fn check_request_health(&self) -> ComponentHealth {
        let active_requests = self.resource_usage.active_requests.load(Ordering::Relaxed);

        let status = if active_requests > 1000 {
            ComponentStatus::Critical
        } else if active_requests > 500 {
            ComponentStatus::Warning
        } else {
            ComponentStatus::Healthy
        };

        let message = if active_requests > 1000 {
            Some("Very high request load".to_string())
        } else if active_requests > 500 {
            Some("High request load".to_string())
        } else {
            None
        };

        let mut metrics = HashMap::new();
        metrics.insert("active_requests".to_string(), active_requests as f64);

        ComponentHealth {
            status,
            message,
            last_update: SystemTime::now(),
            metrics,
        }
    }

    async fn check_error_health(&self) -> ComponentHealth {
        let error_counts = self.error_counts.read().await;
        let total_errors: u64 = error_counts
            .values()
            .map(|c| c.load(Ordering::Relaxed))
            .sum();

        let request_counts = self.request_counts.read().await;
        let total_requests: u64 = request_counts
            .values()
            .map(|c| c.load(Ordering::Relaxed))
            .sum();

        let error_rate = if total_requests > 0 {
            (total_errors as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };

        let status = if error_rate > 10.0 {
            ComponentStatus::Critical
        } else if error_rate > 5.0 {
            ComponentStatus::Warning
        } else {
            ComponentStatus::Healthy
        };

        let message = if error_rate > 10.0 {
            Some(format!("High error rate: {error_rate:.1}%"))
        } else if error_rate > 5.0 {
            Some(format!("Elevated error rate: {error_rate:.1}%"))
        } else {
            None
        };

        let mut metrics = HashMap::new();
        metrics.insert("error_rate".to_string(), error_rate);
        metrics.insert("total_errors".to_string(), total_errors as f64);

        ComponentHealth {
            status,
            message,
            last_update: SystemTime::now(),
            metrics,
        }
    }

    fn calculate_health_score(&self, components: &HashMap<String, ComponentHealth>) -> u8 {
        let total_components = components.len();
        if total_components == 0 {
            return 100;
        }

        let score_sum: u32 = components
            .values()
            .map(|health| match health.status {
                ComponentStatus::Healthy => 100,
                ComponentStatus::Warning => 75,
                ComponentStatus::Degraded => 50,
                ComponentStatus::Critical => 25,
                ComponentStatus::Unavailable => 0,
            })
            .sum();

        (score_sum / total_components as u32) as u8
    }

    async fn generate_health_alerts(
        &self,
        components: &HashMap<String, ComponentHealth>,
    ) -> Vec<HealthAlert> {
        let mut alerts = Vec::new();

        for (component_name, health) in components {
            let (severity, should_alert) = match health.status {
                ComponentStatus::Critical => (AlertSeverity::Critical, true),
                ComponentStatus::Warning => (AlertSeverity::Warning, true),
                ComponentStatus::Degraded => (AlertSeverity::Warning, true),
                ComponentStatus::Unavailable => (AlertSeverity::Critical, true),
                ComponentStatus::Healthy => (AlertSeverity::Info, false),
            };

            if should_alert {
                let message = health
                    .message
                    .clone()
                    .unwrap_or_else(|| format!("{component_name} component is not healthy"));

                alerts.push(HealthAlert {
                    severity,
                    message,
                    component: component_name.clone(),
                    timestamp: SystemTime::now(),
                    id: format!(
                        "{}_{}",
                        component_name,
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or(Duration::from_secs(0))
                            .as_secs()
                    ),
                });
            }
        }

        alerts
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceUsageTracker {
    fn new() -> Self {
        Self {
            memory_usage_mb: AtomicUsize::new(0),
            peak_memory_mb: AtomicUsize::new(0),
            cpu_samples: RwLock::new(Vec::new()),
            active_requests: AtomicUsize::new(0),
            peak_concurrent_requests: AtomicUsize::new(0),
        }
    }
}

impl ConnectionMetrics {
    fn new() -> Self {
        Self {
            total_connections: AtomicU64::new(0),
            active_connections: AtomicUsize::new(0),
            peak_connections: AtomicUsize::new(0),
            connection_errors: AtomicU64::new(0),
            connection_durations: RwLock::new(Vec::new()),
        }
    }
}

impl HealthStatus {
    fn new() -> Self {
        Self {
            health_score: 100,
            components: HashMap::new(),
            last_check: SystemTime::now(),
            alerts: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_metrics_collection() {
        let metrics = MetricsCollector::new();

        // Record some test data
        metrics
            .record_request("textDocument/completion", Duration::from_millis(100))
            .await;
        metrics
            .record_request("textDocument/hover", Duration::from_millis(50))
            .await;
        metrics.record_error("timeout").await;

        metrics.update_memory_usage(100);
        metrics.record_cpu_usage(25.5).await;

        metrics.record_connection();

        let snapshot = metrics.get_snapshot().await;

        assert!(snapshot.requests.total_requests > 0);
        assert!(snapshot.resources.current_memory_mb == 100);
        assert!(snapshot.connections.total_connections > 0);
    }

    #[tokio::test]
    async fn test_health_check() {
        let metrics = MetricsCollector::new();

        // Simulate normal operation
        metrics.update_memory_usage(100); // 100MB - healthy
        metrics.record_cpu_usage(25.0).await;

        let health = metrics.perform_health_check().await;

        assert!(health.health_score > 50);
        assert!(health.components.contains_key("memory"));
        assert!(health.components.contains_key("connections"));
    }

    #[tokio::test]
    async fn test_resource_limits() {
        let metrics = MetricsCollector::new();

        // Simulate high memory usage (critical)
        metrics.update_memory_usage(1200); // 1.2GB - critical

        // Simulate high active requests (critical)
        metrics
            .resource_usage
            .active_requests
            .store(1500, Ordering::Relaxed);

        let health = metrics.perform_health_check().await;
        let memory_health = health.components.get("memory").unwrap();
        let requests_health = health.components.get("requests").unwrap();

        assert!(matches!(memory_health.status, ComponentStatus::Critical));
        assert!(matches!(requests_health.status, ComponentStatus::Critical));

        // With 2 critical (25 each) and 2 healthy (100 each): (25 + 25 + 100 + 100) / 4 = 62.5
        assert!(health.health_score < 75);
        assert!(!health.alerts.is_empty());
    }
}
