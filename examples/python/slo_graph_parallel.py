#!/usr/bin/env python3
"""
Parallel SLO Graph Evaluation Example

Demonstrates how to use the SloGraph with GIL-released parallel evaluation
via Rayon thread pool. This example shows:

1. Creating a graph of independent metric nodes
2. Evaluating nodes in parallel using Rayon
3. Extracting results and computing aggregate scores
4. Monitoring evaluation performance
"""

import time
from neuralbudget import SloGraph

# Example 1: Simple parallel evaluation of 5 metrics
print("=" * 60)
print("Example 1: Parallel SLO Graph Evaluation")
print("=" * 60)

# Create a graph with 5 independent metric nodes
# Format: (node_id, metric_value, threshold)
nodes_data = [
    ("api_latency_p99", 150.0, 200.0),      # Latency in ms (threshold 200ms)
    ("availability", 99.95, 99.9),           # Percentage (threshold 99.9%)
    ("error_rate", 0.1, 0.5),                # Percentage (threshold 0.5%)
    ("cpu_utilization", 0.65, 0.80),        # Fraction (threshold 80%)
    ("memory_utilization", 0.55, 0.85),     # Fraction (threshold 85%)
]

graph = SloGraph(nodes_data)
print(f"\nCreated SLO graph with {graph.node_count} nodes")

# Evaluate all nodes in parallel (with explicit GIL release)
print("\nEvaluating nodes in parallel...")
start = time.time()
results = graph.evaluate()  # This releases the GIL and uses Rayon thread pool
elapsed = time.time() - start

print(f"Parallel evaluation completed in {elapsed*1000:.3f} ms")
print("\nResults (node_id, value, threshold, pass, score):")
for node_id, value, threshold, passed, score in results:
    status = "✓ PASS" if passed else "✗ FAIL"
    print(f"  {status:7} {node_id:20} value={value:7.2f}, threshold={threshold:6.2f}, score={score:.3f}")

# Example 2: Aggregate scoring
print("\n" + "=" * 60)
print("Example 2: Aggregate Metrics")
print("=" * 60)

all_pass = graph.all_pass()
aggregate_score = graph.aggregate_score()
pass_count = graph.pass_count()

print(f"\nGraph Status:")
print(f"  All nodes passing: {all_pass}")
print(f"  Nodes passing: {pass_count}/{graph.node_count}")
print(f"  Aggregate score: {aggregate_score:.3f}")

# Example 3: Node-specific operations
print("\n" + "=" * 60)
print("Example 3: Node-specific Operations")
print("=" * 60)

# Retrieve a specific node
api_latency = graph.get_node("api_latency_p99")
if api_latency:
    node_id, value, threshold = api_latency
    print(f"\nRetrieved node '{node_id}':")
    print(f"  Current value: {value} ms")
    print(f"  Threshold: {threshold} ms")
    print(f"  Status: {'✓ PASS' if value >= threshold else '✗ FAIL'}")

# Update a node's value
print("\nUpdating 'api_latency_p99' to 180.0 ms...")
success = graph.update_node("api_latency_p99", 180.0)
if success:
    api_latency = graph.get_node("api_latency_p99")
    if api_latency:
        node_id, value, threshold = api_latency
        print(f"  Updated value: {value} ms (now {'✓ PASS' if value >= threshold else '✗ FAIL'})")

# Example 4: Large-scale parallel evaluation (demonstrates Rayon benefits)
print("\n" + "=" * 60)
print("Example 4: Large-scale Parallel Evaluation")
print("=" * 60)

# Create a larger graph with 100 nodes
print("\nCreating a graph with 100 nodes...")
large_nodes = [
    (f"metric_{i}", float(50 + i % 100), 75.0)
    for i in range(100)
]

large_graph = SloGraph(large_nodes)

# Evaluate and measure
print(f"Graph created with {large_graph.node_count} nodes")
start = time.time()
large_results = large_graph.evaluate()  # Rayon handles parallel execution
elapsed = time.time() - start

large_aggregate = large_graph.aggregate_score()
large_pass_count = large_graph.pass_count()

print(f"\nLarge-scale evaluation:")
print(f"  Evaluation time: {elapsed*1000:.3f} ms")
print(f"  Nodes evaluated: {len(large_results)}")
print(f"  Nodes passing: {large_pass_count}/{large_graph.node_count}")
print(f"  Aggregate score: {large_aggregate:.3f}")
print(f"  Throughput: {len(large_results)/elapsed:.0f} nodes/second")

# Example 5: Export all nodes as tuples
print("\n" + "=" * 60)
print("Example 5: Export Node Data")
print("=" * 60)

print("\nAll nodes (first 5 only):")
all_nodes = large_graph.nodes_as_tuples()
for i, (node_id, value, threshold, passed, score) in enumerate(all_nodes[:5]):
    status = "✓" if passed else "✗"
    print(f"  [{status}] {node_id:15} value={value:6.2f}, score={score:.3f}")
print(f"  ... ({len(all_nodes)-5} more nodes)")

print("\n" + "=" * 60)
print("Summary: SloGraph enables parallel SLO evaluation via Rayon")
print("=" * 60)
print("""
Key Benefits:
  1. GIL-released evaluation: py.allow_threads() unlocks thread pool
  2. Parallel metrics: Independent nodes evaluated concurrently
  3. High throughput: 100+ nodes/ms with Rayon thread pool
  4. Composable: Results can be aggregated for composite scoring

Next steps:
  - Integrate into real-time metric pipelines
  - Combine with StreamingAggregator for continuous evaluation
  - Scale to thousands of metrics for large deployments
""")
