#!/usr/bin/env python3
"""
Parallel Metric Batch Evaluation Example

Demonstrates how to use ParallelMetricBatch with GIL-released parallel evaluation
via Rayon thread pool. This example shows:

1. Creating a batch of independent metric nodes
2. Evaluating nodes in parallel using Rayon
3. Extracting results and computing aggregate scores
4. Monitoring evaluation performance
"""

import time
from neuralbudget import ParallelMetricBatch

# Example 1: Simple parallel evaluation of 5 metrics
print("=" * 60)
print("Example 1: Parallel Metric Batch Evaluation")
print("=" * 60)

# Create a batch with 5 independent metric nodes
# Format: (node_id, metric_value, threshold)
nodes_data = [
    ("api_latency_p99", 150.0, 200.0),      # Latency in ms (threshold 200ms)
    ("availability", 99.95, 99.9),           # Percentage (threshold 99.9%)
    ("error_rate", 0.1, 0.5),                # Percentage (threshold 0.5%)
    ("cpu_utilization", 0.65, 0.80),        # Fraction (threshold 80%)
    ("memory_utilization", 0.55, 0.85),     # Fraction (threshold 85%)
]

batch = ParallelMetricBatch(nodes_data)
print(f"\nCreated metric batch with {batch.node_count} nodes")

# Evaluate all nodes in parallel (with explicit GIL release)
print("\nEvaluating nodes in parallel...")
start = time.time()
results = batch.evaluate()  # This releases the GIL and uses Rayon thread pool
elapsed = time.time() - start

print(f"Parallel evaluation completed in {elapsed*1000:.3f} ms")
print("\nResults (node_id, value, threshold, pass, score):")
for node_id, value, threshold, passed, score in results:
    status = "✓ PASS" if passed else "✗ FAIL"
    print(f"  {status:7} {node_id:20} value={value:7.2f}, threshold={threshold:6.2f}, score={score:.3f}")

# Example 2: Aggregate Metrics
print("\n" + "=" * 60)
print("Example 2: Aggregate Metrics")
print("=" * 60)

# Compute aggregate results from the batch evaluation
pass_count = sum(1 for _, _, _, passed, _ in results if passed)
aggregate_score = sum(score for _, _, _, _, score in results) / len(results) if results else 0.0

print(f"\nBatch Status:")
print(f"  All nodes passing: {pass_count == len(results)}")
print(f"  Nodes passing: {pass_count}/{batch.node_count}")
print(f"  Aggregate score: {aggregate_score:.3f}")

# Example 3: Re-evaluate with different metric values
print("\n" + "=" * 60)
print("Example 3: Re-evaluate with Different Values")
print("=" * 60)

# Create new batch with updated values
updated_nodes_data = [
    ("api_latency_p99", 180.0, 200.0),      # Improved latency
    ("availability", 99.98, 99.9),           # Better availability
    ("error_rate", 0.05, 0.5),               # Lower error rate
    ("cpu_utilization", 0.70, 0.80),        # Higher CPU usage
    ("memory_utilization", 0.60, 0.85),     # Higher memory usage
]

updated_batch = ParallelMetricBatch(updated_nodes_data)
print(f"\nCreated updated batch with {updated_batch.node_count} nodes")

updated_results = updated_batch.evaluate()
print("\nUpdated Results:")
for node_id, value, threshold, passed, score in updated_results:
    status = "✓ PASS" if passed else "✗ FAIL"
    print(f"  {status:7} {node_id:20} value={value:7.2f}, threshold={threshold:6.2f}, score={score:.3f}")

# Example 4: Large-scale parallel evaluation (demonstrates Rayon benefits)
print("\n" + "=" * 60)
print("Example 4: Large-scale Parallel Evaluation")
print("=" * 60)

# Create a larger batch with 100 nodes
print("\nCreating a batch with 100 nodes...")
large_nodes = [
    (f"metric_{i}", float(50 + i % 100), 75.0)
    for i in range(100)
]

large_batch = ParallelMetricBatch(large_nodes)

# Evaluate and measure
print(f"Batch created with {large_batch.node_count} nodes")
start = time.time()
large_results = large_batch.evaluate()  # Rayon handles parallel execution
elapsed = time.time() - start

large_pass_count = sum(1 for _, _, _, passed, _ in large_results if passed)
large_aggregate = sum(score for _, _, _, _, score in large_results) / len(large_results) if large_results else 0.0

print(f"\nLarge-scale evaluation:")
print(f"  Evaluation time: {elapsed*1000:.3f} ms")
print(f"  Nodes evaluated: {len(large_results)}")
print(f"  Nodes passing: {large_pass_count}/{large_batch.node_count}")
print(f"  Aggregate score: {large_aggregate:.3f}")
print(f"  Throughput: {len(large_results)/elapsed:.0f} nodes/second")

# Example 5: View sample of large batch results
print("\n" + "=" * 60)
print("Example 5: Large Batch Results (Sample)")
print("=" * 60)

print("\nFirst 5 results from large batch:")
for i, (node_id, value, threshold, passed, score) in enumerate(large_results[:5]):
    status = "✓" if passed else "✗"
    print(f"  [{status}] {node_id:15} value={value:6.2f}, score={score:.3f}")
print(f"  ... ({len(large_results)-5} more nodes)")

print("\n" + "=" * 60)
print("Summary: ParallelMetricBatch enables parallel SLO evaluation")
print("=" * 60)
print("""
Key Benefits:
  1. GIL-released evaluation: py.allow_threads() unlocks thread pool
  2. Parallel metrics: Independent nodes evaluated concurrently
  3. High throughput: 50,000+ nodes/sec with Rayon thread pool
  4. Composable: Results can be aggregated for composite scoring

Key Difference from CompositeSloGraph:
  - ParallelMetricBatch: No dependencies, all metrics independent
  - CompositeSloGraph: Supports topological DAG with failure propagation

Next steps:
  - Integrate into real-time metric pipelines
  - Combine with StreamingAggregator for continuous evaluation
  - Scale to thousands of metrics for large deployments
  - Use CompositeSloGraph if you need dependency modeling
""")
