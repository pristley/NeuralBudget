use pyo3::prelude::*;
use rayon::prelude::*;
use std::collections::HashMap;

/// A metric node in the SLO evaluation graph.
/// Represents an independent metric that can be evaluated in parallel.
#[derive(Clone, Debug)]
pub struct SloNode {
    /// Unique identifier for this node
    pub id: String,
    /// Metric value for evaluation
    pub value: f64,
    /// Threshold for pass/fail determination
    pub threshold: f64,
}

impl SloNode {
    /// Evaluate this node against its threshold.
    /// Returns true if value meets the threshold.
    pub fn evaluate(&self) -> bool {
        self.value >= self.threshold
    }

    /// Calculate a score: value / threshold (clamped to [0.0, 1.0]).
    pub fn score(&self) -> f64 {
        if self.threshold <= 0.0 {
            1.0
        } else {
            (self.value / self.threshold).min(1.0)
        }
    }
}

/// Result of evaluating a single SLO node.
#[derive(Clone, Debug)]
pub struct SloNodeEvaluation {
    pub id: String,
    pub value: f64,
    pub threshold: f64,
    pub pass: bool,
    pub score: f64,
}

/// A batch of independent SLO metrics evaluated in parallel.
/// Unlike CompositeSloGraph, this does NOT model dependencies.
/// Nodes are evaluated independently and concurrently for maximum throughput.
///
/// ## Usage
///
/// Primary workflow:
/// ```python
/// batch = ParallelMetricBatch([("metric", 100.0, 200.0)])
/// results = batch.evaluate()  # [(id, value, threshold, pass, score), ...]
/// if batch.all_pass():
///     print("All metrics passing")
/// ```
///
/// ## Thread Safety
///
/// **NOT thread-safe for concurrent access.** Do NOT call `evaluate()` and `update_node()`
/// concurrently from different threads on the same instance. Synchronize externally with Mutex if needed.
///
/// ## Performance
///
/// `evaluate()` releases the Python GIL, enabling true concurrent execution on the Rayon thread pool.
#[pyclass]
pub struct ParallelMetricBatch {
    /// List of metric nodes to evaluate
    nodes: Vec<SloNode>,
    /// Optional adjacency info for dependency tracking (not used for ordering)
    #[pyo3(get)]
    pub node_count: usize,
}

#[pymethods]
impl ParallelMetricBatch {
    /// Create a new batch of independent metrics.
    ///
    /// Metrics are evaluated in parallel with no dependency modeling.
    /// No validation of node IDs (assume unique).
    #[new]
    pub fn new(node_data: Vec<(String, f64, f64)>) -> Self {
        let nodes = node_data
            .into_iter()
            .map(|(id, value, threshold)| SloNode {
                id,
                value,
                threshold,
            })
            .collect();

        let node_count = nodes.len();

        ParallelMetricBatch { nodes, node_count }
    }

    /// Evaluate all nodes in parallel using Rayon, with explicit GIL release.
    ///
    /// This method:
    /// 1. Releases the Python GIL via `py.allow_threads()`
    /// 2. Uses `rayon::par_iter()` to evaluate nodes concurrently
    /// 3. Returns results as a vector of evaluation outcomes
    ///
    /// **Important:** The underlying Rust computation runs in true parallel threads.
    /// Python code can continue running on other threads while this evaluates.
    pub fn evaluate(&self, py: Python) -> Vec<(String, f64, f64, bool, f64)> {
        // Explicitly release the GIL and run the thread pool
        let results = py.allow_threads(|| {
            // Use Rayon's parallel iterator on the nodes
            // Each node is evaluated independently on separate threads
            self.nodes
                .par_iter()
                .map(|node| {
                    let pass = node.evaluate();
                    let score = node.score();
                    (node.id.clone(), node.value, node.threshold, pass, score)
                })
                .collect::<Vec<_>>()
        });

        results
    }

    /// Get the overall graph pass/fail status (all nodes must pass).
    ///
    /// Returns true only if ALL nodes have value >= threshold.
    /// Returns true for an empty batch (vacuous truth).
    pub fn all_pass(&self) -> bool {
        self.nodes.iter().all(|node| node.evaluate())
    }

    /// Retrieve a specific node by ID.
    pub fn get_node(&self, node_id: &str) -> Option<(String, f64, f64)> {
        self.nodes
            .iter()
            .find(|n| n.id == node_id)
            .map(|n| (n.id.clone(), n.value, n.threshold))
    }

    /// Update a node's value by ID.
    /// Returns true if the node was found and updated.
    pub fn update_node(&mut self, node_id: &str, new_value: f64) -> bool {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == node_id) {
            node.value = new_value;
            true
        } else {
            false
        }
    }
}

impl Default for ParallelMetricBatch {
    fn default() -> Self {
        ParallelMetricBatch {
            nodes: Vec::new(),
            node_count: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slo_node_evaluation() {
        let pass_node = SloNode {
            id: "api_latency".to_string(),
            value: 250.0,
            threshold: 200.0,
        };
        assert!(pass_node.evaluate()); // 250 >= 200 is true

        let fail_node = SloNode {
            id: "api_latency".to_string(),
            value: 150.0,
            threshold: 200.0,
        };
        assert!(!fail_node.evaluate()); // 150 >= 200 is false
    }

    #[test]
    fn test_slo_node_score() {
        let node = SloNode {
            id: "availability".to_string(),
            value: 99.5,
            threshold: 99.9,
        };

        let score = node.score();
        assert!((score - (99.5 / 99.9)).abs() < 0.01); // ~0.996
    }

    #[test]
    fn test_slo_graph_creation() {
        let graph = ParallelMetricBatch::new(vec![
            ("latency".to_string(), 150.0, 200.0),
            ("availability".to_string(), 99.9, 99.0),
            ("error_rate".to_string(), 0.1, 0.5),
        ]);

        assert_eq!(graph.node_count, 3);
    }

    #[test]
    fn test_slo_graph_all_pass() {
        let graph = ParallelMetricBatch::new(vec![
            ("latency".to_string(), 250.0, 200.0),    // Pass: 250 >= 200
            ("availability".to_string(), 99.9, 99.0), // Pass: 99.9 >= 99.0
            ("error_rate".to_string(), 0.1, 0.5),     // Fail: 0.1 >= 0.5 is false
        ]);

        assert!(!graph.all_pass()); // Should fail because error_rate doesn't pass
    }

    #[test]
    fn test_slo_graph_aggregate_score() {
        let graph = ParallelMetricBatch::new(vec![
            ("latency".to_string(), 100.0, 100.0),     // Score: 1.0
            ("availability".to_string(), 50.0, 100.0), // Score: 0.5
        ]);

        let agg = graph.aggregate_score();
        assert!((agg - 0.75).abs() < 0.01); // (1.0 + 0.5) / 2 = 0.75
    }

    #[test]
    fn test_slo_graph_update_node() {
        let mut graph = ParallelMetricBatch::new(vec![("latency".to_string(), 150.0, 200.0)]);

        assert!(graph.update_node("latency", 250.0));
        assert!(!graph.update_node("nonexistent", 100.0));

        if let Some((_, value, _)) = graph.get_node("latency") {
            assert_eq!(value, 250.0);
        }
    }

    #[test]
    fn test_slo_graph_pass_count() {
        let graph = ParallelMetricBatch::new(vec![
            ("latency".to_string(), 250.0, 200.0),    // Pass
            ("availability".to_string(), 99.9, 99.0), // Pass
            ("error_rate".to_string(), 0.1, 0.5),     // Fail
        ]);

        assert_eq!(graph.pass_count(), 2);
    }

    #[test]
    fn test_slo_graph_parallel_evaluation() {
        let graph = ParallelMetricBatch::new(vec![
            ("node_1".to_string(), 100.0, 50.0),
            ("node_2".to_string(), 75.0, 100.0),
            ("node_3".to_string(), 200.0, 150.0),
            ("node_4".to_string(), 25.0, 50.0),
            ("node_5".to_string(), 90.0, 80.0),
        ]);

        // Note: We can't actually test py.allow_threads() without a Python context,
        // but we can verify the sequential behavior matches what parallel would produce
        let results = graph.nodes_as_tuples();
        assert_eq!(results.len(), 5);

        // node_1: 100 >= 50 = true, score = 100/50 = 1.0
        assert!(results[0].3); // pass
        assert_eq!(results[0].4, 1.0); // score

        // node_2: 75 >= 100 = false, score = 75/100 = 0.75
        assert!(!results[1].3); // fail
        assert!((results[1].4 - 0.75).abs() < 0.01); // score

        // node_3: 200 >= 150 = true, score = 200/150 = 1.0 (clamped)
        assert!(results[2].3); // pass
        assert_eq!(results[2].4, 1.0); // score

        // node_4: 25 >= 50 = false, score = 25/50 = 0.5
        assert!(!results[3].3); // fail
        assert!((results[3].4 - 0.5).abs() < 0.01); // score

        // node_5: 90 >= 80 = true, score = 90/80 = 1.0 (clamped)
        assert!(results[4].3); // pass
        assert_eq!(results[4].4, 1.0); // score
    }

    #[test]
    fn test_slo_graph_empty() {
        let graph = ParallelMetricBatch::new(vec![]);
        assert_eq!(graph.node_count, 0);
        assert!(graph.all_pass()); // Empty graph passes
        assert_eq!(graph.aggregate_score(), 1.0);
        assert_eq!(graph.pass_count(), 0);
    }

    #[test]
    fn test_slo_graph_zero_threshold() {
        let graph = ParallelMetricBatch::new(vec![("test".to_string(), 100.0, 0.0)]);
        let score = graph.nodes[0].score();
        assert_eq!(score, 1.0); // Division by zero avoided; returns 1.0
    }

    #[test]
    fn test_result_consistency_before_and_after_evaluate() {
        // This test verifies that query methods (all_pass, aggregate_score, pass_count, nodes_as_tuples)
        // recompute from current state and work correctly even before evaluate() is called,
        // and that they reflect updated values after mutations.

        let mut batch = ParallelMetricBatch::new(vec![
            ("latency".to_string(), 150.0, 200.0),     // Fails: 150 < 200
            ("availability".to_string(), 99.95, 99.9), // Passes: 99.95 >= 99.9
        ]);

        // BEFORE calling evaluate(), query methods should work correctly
        // (they recompute from current state, not from cached results)
        assert!(!batch.all_pass()); // One metric fails
        assert_eq!(batch.pass_count(), 1);
        assert!((batch.aggregate_score() - 0.75).abs() < 0.01); // Mean of 0.75 and 1.0

        let tuples = batch.nodes_as_tuples();
        assert_eq!(tuples.len(), 2);
        assert!(!tuples[0].3); // latency fails
        assert!(tuples[1].3); // availability passes

        // After update_node(), query methods immediately reflect the change
        batch.update_node("latency", 250.0); // Change to pass
        assert!(batch.all_pass()); // Both metrics now pass
        assert_eq!(batch.pass_count(), 2);
        assert_eq!(batch.aggregate_score(), 1.0); // Mean of 1.0 and 1.0

        // Calling evaluate() does not affect query results
        // (it's mainly for GIL release; results are not cached)
        let _eval_results = batch.nodes_as_tuples(); // This is what evaluate() would return
        assert!(batch.all_pass()); // Still passes
    }

    #[test]
    fn test_query_methods_always_consistent_with_state() {
        // Verify that calling query methods multiple times returns consistent results
        // (all based on current node state, not on cached results)

        let mut batch = ParallelMetricBatch::new(vec![
            ("metric_1".to_string(), 100.0, 200.0), // Fails
            ("metric_2".to_string(), 300.0, 200.0), // Passes
        ]);

        // First batch of calls
        let pass_count_1 = batch.pass_count();
        let score_1 = batch.aggregate_score();

        // Second batch of calls - should be identical
        let pass_count_2 = batch.pass_count();
        let score_2 = batch.aggregate_score();

        assert_eq!(pass_count_1, pass_count_2);
        assert_eq!(score_1, score_2);

        // Update a node
        batch.update_node("metric_1", 250.0); // Now passes

        // Query methods immediately reflect the change
        let pass_count_3 = batch.pass_count();
        let score_3 = batch.aggregate_score();

        assert_eq!(pass_count_3, 2); // Both pass now
        assert_eq!(score_3, 1.0); // Both score 1.0
        assert_ne!(pass_count_2, pass_count_3); // Different from before update
    }
}
