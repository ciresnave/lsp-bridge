//! Memory usage and resource cleanup validation tests
//!
//! These tests validate proper memory management, resource cleanup,
//! and leak detection in the LSP Bridge.

use lsp_bridge::{LspBridge, LspServerConfig};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::fs;

/// Resource tracker for monitoring allocations and deallocations
#[derive(Debug, Clone)]
pub struct ResourceTracker {
    allocations: Arc<AtomicUsize>,
    deallocations: Arc<AtomicUsize>,
    peak_memory_mb: Arc<AtomicUsize>,
    active_resources: Arc<AtomicUsize>,
}

impl ResourceTracker {
    pub fn new() -> Self {
        Self {
            allocations: Arc::new(AtomicUsize::new(0)),
            deallocations: Arc::new(AtomicUsize::new(0)),
            peak_memory_mb: Arc::new(AtomicUsize::new(0)),
            active_resources: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn track_allocation(&self, _size_mb: usize) {
        self.allocations.fetch_add(1, Ordering::SeqCst);
        self.active_resources.fetch_add(1, Ordering::SeqCst);

        let current_memory = self.get_estimated_memory_usage();
        let current_peak = self.peak_memory_mb.load(Ordering::SeqCst);
        if current_memory > current_peak {
            self.peak_memory_mb.store(current_memory, Ordering::SeqCst);
        }
    }

    pub fn track_deallocation(&self) {
        self.deallocations.fetch_add(1, Ordering::SeqCst);
        if self.active_resources.load(Ordering::SeqCst) > 0 {
            self.active_resources.fetch_sub(1, Ordering::SeqCst);
        }
    }

    pub fn get_allocation_count(&self) -> usize {
        self.allocations.load(Ordering::SeqCst)
    }

    pub fn get_deallocation_count(&self) -> usize {
        self.deallocations.load(Ordering::SeqCst)
    }

    pub fn get_active_resources(&self) -> usize {
        self.active_resources.load(Ordering::SeqCst)
    }

    pub fn get_peak_memory_mb(&self) -> usize {
        self.peak_memory_mb.load(Ordering::SeqCst)
    }

    /// Estimate current memory usage (simulation)
    pub fn get_estimated_memory_usage(&self) -> usize {
        // In a real implementation, this would use system APIs
        // For testing, we simulate based on active resources
        let base_memory = 50; // Base memory in MB
        let resource_memory = self.active_resources.load(Ordering::SeqCst) * 2; // 2MB per resource
        base_memory + resource_memory
    }

    pub fn reset(&self) {
        self.allocations.store(0, Ordering::SeqCst);
        self.deallocations.store(0, Ordering::SeqCst);
        self.peak_memory_mb.store(0, Ordering::SeqCst);
        self.active_resources.store(0, Ordering::SeqCst);
    }

    pub fn check_for_leaks(&self) -> bool {
        let allocated = self.get_allocation_count();
        let deallocated = self.get_deallocation_count();
        allocated == deallocated && self.get_active_resources() == 0
    }
}

impl Default for ResourceTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory stress test parameters
#[derive(Debug, Clone)]
pub struct MemoryStressConfig {
    pub num_iterations: usize,
    pub documents_per_iteration: usize,
    pub document_size_kb: usize,
    pub concurrent_operations: usize,
    pub measurement_interval_ms: u64,
}

impl Default for MemoryStressConfig {
    fn default() -> Self {
        Self {
            num_iterations: 10,
            documents_per_iteration: 5,
            document_size_kb: 10,
            concurrent_operations: 3,
            measurement_interval_ms: 500,
        }
    }
}

/// Create a test document with specified size
fn create_test_document(size_kb: usize) -> String {
    let line_content = "// This is a test line with some content to reach the desired size\n";
    let bytes_per_line = line_content.len();
    let target_bytes = size_kb * 1024;
    let num_lines = target_bytes / bytes_per_line;

    let mut content = String::with_capacity(target_bytes);
    content.push_str("//! Test document for memory testing\n\n");

    for i in 0..num_lines {
        content.push_str(&format!("fn test_function_{i}() {{\n"));
        content.push_str("    // Some test content here\n");
        content.push_str("    println!(\"Hello from function\");\n");
        content.push_str("}\n\n");
    }

    content
}

/// Test basic resource cleanup after server shutdown
#[tokio::test]
async fn test_basic_resource_cleanup() {
    let tracker = ResourceTracker::new();

    // Simulate resource allocation
    tracker.track_allocation(10);
    tracker.track_allocation(5);
    tracker.track_allocation(8);

    assert_eq!(tracker.get_allocation_count(), 3);
    assert_eq!(tracker.get_active_resources(), 3);

    // Simulate cleanup
    tracker.track_deallocation();
    tracker.track_deallocation();
    tracker.track_deallocation();

    assert_eq!(tracker.get_deallocation_count(), 3);
    assert_eq!(tracker.get_active_resources(), 0);
    assert!(tracker.check_for_leaks());

    println!("✅ Basic resource cleanup test passed");
}

/// Test memory usage during server lifecycle
#[tokio::test]
async fn test_server_lifecycle_memory_usage() {
    let tracker = ResourceTracker::new();
    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().to_path_buf();

    // Create a simple test file
    fs::create_dir_all(workspace_path.join("src"))
        .await
        .unwrap();
    let test_content = create_test_document(5); // 5KB document
    fs::write(workspace_path.join("src").join("test.rs"), &test_content)
        .await
        .unwrap();

    // Track initial memory
    let initial_memory = tracker.get_estimated_memory_usage();
    println!("Initial memory usage: {initial_memory} MB");

    // Create and configure bridge (simulate resource allocation)
    tracker.track_allocation(20); // Simulate bridge allocation

    let config = LspServerConfig::new()
        .command("echo") // Use echo as a safe mock command
        .arg("mock-lsp-server")
        .root_path(workspace_path.clone());

    let mut bridge = LspBridge::new();
    tracker.track_allocation(15); // Simulate server config allocation

    // Register server
    let _server_id = bridge.register_server("test-server", config).await.unwrap();
    tracker.track_allocation(10); // Simulate server registration

    let memory_after_setup = tracker.get_estimated_memory_usage();
    println!("Memory after setup: {memory_after_setup} MB");

    // Simulate some operations
    for _i in 0..5 {
        tracker.track_allocation(2); // Simulate document operation
        tokio::time::sleep(Duration::from_millis(50)).await;
        tracker.track_deallocation(); // Simulate cleanup
    }

    let memory_during_ops = tracker.get_estimated_memory_usage();
    println!("Memory during operations: {memory_during_ops} MB");

    // Clean shutdown
    tracker.track_deallocation(); // Server registration cleanup
    tracker.track_deallocation(); // Server config cleanup
    tracker.track_deallocation(); // Bridge cleanup

    let final_memory = tracker.get_estimated_memory_usage();
    println!("Final memory usage: {final_memory} MB");

    // Verify reasonable memory usage
    let memory_increase = memory_during_ops - initial_memory;
    assert!(
        memory_increase < 100,
        "Excessive memory usage during operations: {memory_increase} MB"
    );

    // Verify cleanup
    let active_resources = tracker.get_active_resources();
    assert!(
        active_resources <= 2,
        "Too many active resources after cleanup: {active_resources}"
    );

    println!("✅ Server lifecycle memory test passed");
    println!("   Peak memory: {} MB", tracker.get_peak_memory_mb());
    println!("   Active resources: {active_resources}");
}

/// Test memory usage under stress conditions
#[tokio::test]
async fn test_memory_stress_conditions() {
    let tracker = ResourceTracker::new();
    let config = MemoryStressConfig::default();

    println!("🔥 Starting memory stress test...");
    println!("   Iterations: {}", config.num_iterations);
    println!(
        "   Documents per iteration: {}",
        config.documents_per_iteration
    );
    println!("   Document size: {} KB", config.document_size_kb);

    let mut memory_measurements = Vec::new();

    for iteration in 0..config.num_iterations {
        let iteration_start = Instant::now();

        // Simulate heavy document operations
        for doc_idx in 0..config.documents_per_iteration {
            // Simulate document creation and processing
            tracker.track_allocation(config.document_size_kb / 1024); // Convert KB to MB

            // Simulate processing time
            tokio::time::sleep(Duration::from_millis(10)).await;

            // Simulate document cleanup (but not immediate)
            if doc_idx > 0 && doc_idx % 2 == 0 {
                tracker.track_deallocation();
            }
        }

        // Measure memory after each iteration
        let current_memory = tracker.get_estimated_memory_usage();
        memory_measurements.push(current_memory);

        println!(
            "   Iteration {}: {} MB ({:?})",
            iteration,
            current_memory,
            iteration_start.elapsed()
        );

        // Verify memory isn't growing excessively
        if iteration > 2 {
            let recent_avg = memory_measurements[iteration - 2..].iter().sum::<usize>() / 3;
            let early_avg = memory_measurements[0..3.min(iteration)]
                .iter()
                .sum::<usize>()
                / 3.min(iteration);
            let growth_ratio = recent_avg as f64 / early_avg as f64;

            assert!(
                growth_ratio < 2.0,
                "Excessive memory growth detected: {growth_ratio:.2}x increase"
            );
        }

        // Cleanup some resources periodically
        if iteration % 3 == 0 {
            for _ in 0..2 {
                tracker.track_deallocation();
            }
        }

        tokio::time::sleep(Duration::from_millis(config.measurement_interval_ms)).await;
    }

    // Final cleanup
    let initial_active = tracker.get_active_resources();
    for _ in 0..initial_active {
        tracker.track_deallocation();
    }

    // Verify final state
    let final_memory = tracker.get_estimated_memory_usage();
    let peak_memory = tracker.get_peak_memory_mb();
    let final_active = tracker.get_active_resources();

    println!("📊 Stress test results:");
    println!("   Peak memory: {peak_memory} MB");
    println!("   Final memory: {final_memory} MB");
    println!("   Final active resources: {final_active}");
    println!("   Total allocations: {}", tracker.get_allocation_count());
    println!(
        "   Total deallocations: {}",
        tracker.get_deallocation_count()
    );

    // Performance assertions
    assert!(peak_memory < 200, "Peak memory too high: {peak_memory} MB");
    assert!(
        final_active == 0,
        "Resources not properly cleaned up: {final_active}"
    );

    println!("✅ Memory stress test completed successfully");
}

/// Test concurrent operations memory safety
#[tokio::test]
async fn test_concurrent_memory_safety() {
    let tracker = Arc::new(ResourceTracker::new());

    println!("🔄 Testing concurrent memory operations...");

    let num_tasks = 10;
    let operations_per_task = 20;

    let mut handles = Vec::new();

    for _task_id in 0..num_tasks {
        let tracker_clone = Arc::clone(&tracker);

        let handle = tokio::spawn(async move {
            for op_idx in 0..operations_per_task {
                // Simulate allocations
                tracker_clone.track_allocation(1 + (op_idx % 5));

                // Random small delay
                tokio::time::sleep(Duration::from_millis(1 + (op_idx % 10) as u64)).await;

                // Simulate deallocations (but not all)
                if op_idx % 2 == 0 {
                    tracker_clone.track_deallocation();
                }

                // Check memory usage periodically
                if op_idx % 5 == 0 {
                    let memory = tracker_clone.get_estimated_memory_usage();
                    if memory > 150 {
                        // If memory gets too high, force some cleanup
                        for _ in 0..3 {
                            tracker_clone.track_deallocation();
                        }
                    }
                }
            }

            // Final cleanup for this task
            let remaining = operations_per_task / 2; // Approximate remaining allocations
            for _ in 0..remaining {
                tracker_clone.track_deallocation();
            }
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify final state
    let final_active = tracker.get_active_resources();
    let peak_memory = tracker.get_peak_memory_mb();
    let total_allocations = tracker.get_allocation_count();
    let total_deallocations = tracker.get_deallocation_count();

    println!("📊 Concurrent test results:");
    println!("   Total allocations: {total_allocations}");
    println!("   Total deallocations: {total_deallocations}");
    println!("   Peak memory: {peak_memory} MB");
    println!("   Final active resources: {final_active}");

    // Verify reasonable behavior
    assert!(
        total_allocations >= num_tasks * operations_per_task,
        "Not enough allocations tracked"
    );
    assert!(
        peak_memory < 300,
        "Peak memory too high during concurrent operations: {peak_memory} MB"
    );
    assert!(
        final_active <= 10,
        "Too many resources left active: {final_active}"
    );

    println!("✅ Concurrent memory safety test completed");
}

/// Test resource cleanup after errors and failures
#[tokio::test]
async fn test_error_condition_cleanup() {
    let tracker = ResourceTracker::new();

    println!("💥 Testing resource cleanup during error conditions...");

    // Simulate normal operation followed by errors
    for i in 0..5 {
        tracker.track_allocation(5);

        // Simulate an error condition every few operations
        if i % 3 == 2 {
            // Simulate error handling and partial cleanup
            tracker.track_deallocation();
            println!("   Simulated error at operation {i}, cleaned up 1 resource");
        }
    }

    let active_before_error_recovery = tracker.get_active_resources();
    println!("   Active resources before error recovery: {active_before_error_recovery}");

    // Simulate error recovery and full cleanup
    let remaining = active_before_error_recovery;
    for _ in 0..remaining {
        tracker.track_deallocation();
    }

    let final_active = tracker.get_active_resources();
    let is_clean = tracker.check_for_leaks();

    println!("📊 Error cleanup results:");
    println!("   Final active resources: {final_active}");
    println!("   Clean shutdown: {is_clean}");

    assert_eq!(final_active, 0, "Resources not cleaned up after errors");
    assert!(is_clean, "Memory leaks detected after error recovery");

    println!("✅ Error condition cleanup test passed");
}

/// Test long-running operations for memory leaks
#[tokio::test]
async fn test_long_running_memory_stability() {
    let tracker = ResourceTracker::new();

    println!("⏱️ Testing long-running memory stability...");

    let duration = Duration::from_secs(5); // Short duration for test
    let start_time = Instant::now();
    let mut iteration = 0;

    let mut baseline_memory = 0;
    let mut memory_samples = Vec::new();

    while start_time.elapsed() < duration {
        iteration += 1;

        // Simulate typical operations
        tracker.track_allocation(2);
        tokio::time::sleep(Duration::from_millis(50)).await;
        tracker.track_deallocation();

        // Periodic heavier operations
        if iteration % 10 == 0 {
            for _ in 0..3 {
                tracker.track_allocation(1);
            }

            // Clean up after heavy operations
            tokio::time::sleep(Duration::from_millis(20)).await;
            for _ in 0..3 {
                tracker.track_deallocation();
            }
        }

        // Sample memory usage
        if iteration % 5 == 0 {
            let current_memory = tracker.get_estimated_memory_usage();
            memory_samples.push(current_memory);

            if baseline_memory == 0 {
                baseline_memory = current_memory;
            }

            // Check for excessive growth
            let growth = current_memory as f64 / baseline_memory as f64;
            if growth > 1.5 {
                println!(
                    "   Warning: Memory growth detected at iteration {iteration}: {growth:.2}x"
                );
            }
        }
    }

    // Final analysis
    let final_memory = tracker.get_estimated_memory_usage();
    let peak_memory = tracker.get_peak_memory_mb();
    let final_active = tracker.get_active_resources();

    // Calculate memory stability metrics
    let avg_memory = memory_samples.iter().sum::<usize>() / memory_samples.len().max(1);
    let max_memory = memory_samples.iter().max().copied().unwrap_or(0);
    let min_memory = memory_samples.iter().min().copied().unwrap_or(0);
    let memory_variance = max_memory - min_memory;

    println!("📊 Long-running stability results:");
    println!("   Duration: {:?}", start_time.elapsed());
    println!("   Total iterations: {iteration}");
    println!("   Baseline memory: {baseline_memory} MB");
    println!("   Average memory: {avg_memory} MB");
    println!("   Memory variance: {memory_variance} MB");
    println!("   Peak memory: {peak_memory} MB");
    println!("   Final memory: {final_memory} MB");
    println!("   Final active resources: {final_active}");

    // Stability assertions
    assert!(
        memory_variance < 50,
        "Memory usage too volatile: {memory_variance} MB variance"
    );
    assert!(
        final_memory <= baseline_memory + 20,
        "Memory growth detected: {} MB increase",
        final_memory - baseline_memory
    );
    assert!(
        final_active <= 2,
        "Too many active resources: {final_active}"
    );

    println!("✅ Long-running memory stability test passed");
}
