#!/usr/bin/env python3
"""
Composite SLO DAG Examples - Real-world scenarios demonstrating dependency-aware SLO evaluation.

This module shows how to use Composite SLO graphs to model inter-service dependencies
and automatically propagate failures across your system.

Key Features Demonstrated:
  - E-commerce checkout pipeline (dependency chain)
  - Multi-region deployments (weighted impact)
  - Failure cascade simulation
  - Integration with live metrics
  - Scenario-based testing
"""

from dataclasses import dataclass
import json
from typing import Optional

try:
    from neuralbudget import (
        CompositeServiceSlo,
        CompositeDependencyEdge,
        CompositeSloGraph,
        evaluate_composite_slo_graph,
    )
except ImportError:
    print("Error: neuralbudget package not installed. Install with: pip install neuralbudget")
    exit(1)


# ============================================================================
# EXAMPLE 1: E-Commerce Checkout Pipeline
# ============================================================================
def example_ecommerce_checkout():
    """
    E-commerce system with dependency chain:
      Feature Store (data) → ML Model (fraud detection) → Fraud Service → Payment Gateway

    Demonstrates: Linear dependency chain, cascading failures
    """
    print("\n" + "=" * 80)
    print("EXAMPLE 1: E-Commerce Checkout Pipeline")
    print("=" * 80)

    graph = CompositeSloGraph(
        services=[
            CompositeServiceSlo(
                service="feature_store",
                local_score=0.98,
                min_pass_score=0.95,
                impact_weight=3.0,  # Critical: all downstream depend on this
            ),
            CompositeServiceSlo(
                service="ml_model_server",
                local_score=0.96,
                min_pass_score=0.95,
                impact_weight=2.5,
            ),
            CompositeServiceSlo(
                service="fraud_detection",
                local_score=0.94,
                min_pass_score=0.90,
                impact_weight=2.0,
            ),
            CompositeServiceSlo(
                service="payment_gateway",
                local_score=0.99,
                min_pass_score=0.99,  # Strictest requirement
                impact_weight=4.0,  # Most critical
            ),
        ],
        dependencies=[
            CompositeDependencyEdge(
                dependency="feature_store",
                dependent="ml_model_server",
                failure_penalty=0.40,  # ML model severely degraded without features
            ),
            CompositeDependencyEdge(
                dependency="ml_model_server",
                dependent="fraud_detection",
                failure_penalty=0.35,
            ),
            CompositeDependencyEdge(
                dependency="fraud_detection",
                dependent="payment_gateway",
                failure_penalty=0.50,  # Can't process payments without fraud check
            ),
        ],
        global_min_pass_score=0.95,
    )

    result = evaluate_composite_slo_graph(graph)

    print(f"\nTopological Order (evaluation sequence):")
    print(f"  {' → '.join(result.topological_order)}")

    print(f"\nPer-Service Evaluation:")
    for svc in result.services:
        status = "✓ PASS" if svc.pass else "✗ FAIL"
        print(f"  {svc.service:20s} Local: {svc.local_score:.2f} "
              f"→ Effective: {svc.effective_score:.2f} {status}")
        if svc.failed_dependencies:
            print(f"    └─ Failed deps: {', '.join(svc.failed_dependencies)}")

    print(f"\nGlobal SLO: {result.global_slo:.2f} "
          f"({'✓ PASS' if result.global_pass else '✗ FAIL'})")

    # Now simulate feature store degradation
    print("\n" + "-" * 80)
    print("Scenario: Feature Store Degrades to 0.85 (below 0.95 threshold)")
    print("-" * 80)

    degraded_graph = CompositeSloGraph(
        services=[
            CompositeServiceSlo(
                service="feature_store",
                local_score=0.85,  # DEGRADED
                min_pass_score=0.95,
                impact_weight=3.0,
            ),
            CompositeServiceSlo(
                service="ml_model_server",
                local_score=0.96,
                min_pass_score=0.95,
                impact_weight=2.5,
            ),
            CompositeServiceSlo(
                service="fraud_detection",
                local_score=0.94,
                min_pass_score=0.90,
                impact_weight=2.0,
            ),
            CompositeServiceSlo(
                service="payment_gateway",
                local_score=0.99,
                min_pass_score=0.99,
                impact_weight=4.0,
            ),
        ],
        dependencies=graph.dependencies,
        global_min_pass_score=0.95,
    )

    degraded_result = evaluate_composite_slo_graph(degraded_graph)

    print(f"\nPer-Service After Degradation:")
    for svc in degraded_result.services:
        status = "✓ PASS" if svc.pass else "✗ FAIL"
        print(f"  {svc.service:20s} Local: {svc.local_score:.2f} "
              f"→ Effective: {svc.effective_score:.2f} {status}")
        if svc.failed_dependencies:
            print(f"    └─ Failed deps: {', '.join(svc.failed_dependencies)}")

    print(f"\nGlobal SLO: {degraded_result.global_slo:.2f} "
          f"({'✓ PASS' if degraded_result.global_pass else '✗ FAIL'})")
    print(f"\n⚠️  ONE service failure cascaded to ALL services!")
    print(f"    This is correct behavior: checkout cannot work without fraud detection.")


# ============================================================================
# EXAMPLE 2: Multi-Region Deployment
# ============================================================================
def example_multi_region():
    """
    Global platform with three regional deployments (US, EU, Asia).
    Each region has database + API with local dependency.

    Demonstrates: Multiple independent branches, weighted impact
    """
    print("\n" + "=" * 80)
    print("EXAMPLE 2: Multi-Region Deployment")
    print("=" * 80)

    graph = CompositeSloGraph(
        services=[
            # US-East Region (primary)
            CompositeServiceSlo(
                service="db_us_east",
                local_score=0.99,
                min_pass_score=0.95,
                impact_weight=2.5,  # Primary region = high impact
            ),
            CompositeServiceSlo(
                service="api_us_east",
                local_score=0.98,
                min_pass_score=0.95,
                impact_weight=3.0,
            ),
            # EU-West Region (secondary)
            CompositeServiceSlo(
                service="db_eu_west",
                local_score=0.97,
                min_pass_score=0.95,
                impact_weight=1.5,  # Secondary = lower impact
            ),
            CompositeServiceSlo(
                service="api_eu_west",
                local_score=0.96,
                min_pass_score=0.95,
                impact_weight=1.5,
            ),
            # Asia-Pacific Region (tertiary)
            CompositeServiceSlo(
                service="db_asia_pac",
                local_score=0.95,
                min_pass_score=0.95,
                impact_weight=1.0,  # Lowest impact
            ),
            CompositeServiceSlo(
                service="api_asia_pac",
                local_score=0.94,
                min_pass_score=0.90,
                impact_weight=1.0,
            ),
        ],
        dependencies=[
            # Each region's API depends on its database
            CompositeDependencyEdge(
                dependency="db_us_east",
                dependent="api_us_east",
                failure_penalty=0.60,
            ),
            CompositeDependencyEdge(
                dependency="db_eu_west",
                dependent="api_eu_west",
                failure_penalty=0.60,
            ),
            CompositeDependencyEdge(
                dependency="db_asia_pac",
                dependent="api_asia_pac",
                failure_penalty=0.60,
            ),
        ],
        global_min_pass_score=0.90,
    )

    result = evaluate_composite_slo_graph(graph)

    print(f"\nTopological Order:")
    print(f"  {' → '.join(result.topological_order)}")

    print(f"\nRegional Status:")
    print(f"\n  US-East (Weight 3.0 + 2.5 = 5.5):")
    for svc in result.services:
        if "us_east" in svc.service:
            print(f"    {svc.service:20s} {svc.local_score:.2f} "
                  f"→ {svc.effective_score:.2f} "
                  f"({'✓ PASS' if svc.pass else '✗ FAIL'})")

    print(f"\n  EU-West (Weight 1.5 + 1.5 = 3.0):")
    for svc in result.services:
        if "eu_west" in svc.service:
            print(f"    {svc.service:20s} {svc.local_score:.2f} "
                  f"→ {svc.effective_score:.2f} "
                  f"({'✓ PASS' if svc.pass else '✗ FAIL'})")

    print(f"\n  Asia-Pacific (Weight 1.0 + 1.0 = 2.0):")
    for svc in result.services:
        if "asia_pac" in svc.service:
            print(f"    {svc.service:20s} {svc.local_score:.2f} "
                  f"→ {svc.effective_score:.2f} "
                  f"({'✓ PASS' if svc.pass else '✗ FAIL'})")

    print(f"\nGlobal SLO: {result.global_slo:.2f} ✓ PASS")
    print(f"\n💡 Note: Weight distribution reflects traffic split:")
    print(f"   US-East 55% + EU-West 30% + Asia-Pac 15%")

    # Simulate US-East database failure
    print("\n" + "-" * 80)
    print("Scenario: US-East Database Fails (0.75 < 0.95 threshold)")
    print("-" * 80)

    failure_graph = CompositeSloGraph(
        services=[
            CompositeServiceSlo(
                service="db_us_east",
                local_score=0.75,  # FAILED
                min_pass_score=0.95,
                impact_weight=2.5,
            ),
            CompositeServiceSlo(
                service="api_us_east",
                local_score=0.98,
                min_pass_score=0.95,
                impact_weight=3.0,
            ),
            CompositeServiceSlo(
                service="db_eu_west",
                local_score=0.97,
                min_pass_score=0.95,
                impact_weight=1.5,
            ),
            CompositeServiceSlo(
                service="api_eu_west",
                local_score=0.96,
                min_pass_score=0.95,
                impact_weight=1.5,
            ),
            CompositeServiceSlo(
                service="db_asia_pac",
                local_score=0.95,
                min_pass_score=0.95,
                impact_weight=1.0,
            ),
            CompositeServiceSlo(
                service="api_asia_pac",
                local_score=0.94,
                min_pass_score=0.90,
                impact_weight=1.0,
            ),
        ],
        dependencies=graph.dependencies,
        global_min_pass_score=0.90,
    )

    failure_result = evaluate_composite_slo_graph(failure_graph)

    print(f"\nGlobal SLO After US-East DB Failure: {failure_result.global_slo:.2f} "
          f"(was {result.global_slo:.2f})")

    print(f"\nImpacted Services:")
    for svc in failure_result.services:
        if not svc.pass and svc.service in ["db_us_east", "api_us_east"]:
            print(f"  {svc.service:20s} {svc.effective_score:.2f} ✗ FAIL")

    print(f"\nStill Healthy:")
    for svc in failure_result.services:
        if svc.pass and "us_east" not in svc.service:
            print(f"  {svc.service:20s} {svc.effective_score:.2f} ✓ PASS")

    print(f"\n💡 Impact: US-East (55% traffic) failed, but EU+Asia (45%) still working")
    print(f"   System overall still healthy at {failure_result.global_slo:.0%} SLO")


# ============================================================================
# EXAMPLE 3: Complex DAG with Multiple Dependency Paths
# ============================================================================
def example_complex_dag():
    """
    Complex dependency graph with multiple paths:

    [Load Balancer]
        ↓
    [API Gateway] ← depends on {Auth, Rate Limit, Observability}
        ↓
    [Services] ← depends on {Cache, Database, Message Queue}

    Demonstrates: Multiple upstream dependencies, complex evaluation order
    """
    print("\n" + "=" * 80)
    print("EXAMPLE 3: Complex DAG with Multiple Dependency Paths")
    print("=" * 80)

    graph = CompositeSloGraph(
        services=[
            # Infrastructure
            CompositeServiceSlo(
                service="auth_service",
                local_score=0.97,
                min_pass_score=0.95,
                impact_weight=2.0,
            ),
            CompositeServiceSlo(
                service="rate_limiter",
                local_score=0.99,
                min_pass_score=0.95,
                impact_weight=1.5,
            ),
            CompositeServiceSlo(
                service="observability",
                local_score=0.98,
                min_pass_score=0.90,  # Less critical
                impact_weight=0.5,
            ),
            # Data Layer
            CompositeServiceSlo(
                service="cache",
                local_score=0.96,
                min_pass_score=0.90,
                impact_weight=1.5,
            ),
            CompositeServiceSlo(
                service="database",
                local_score=0.95,
                min_pass_score=0.95,
                impact_weight=3.0,
            ),
            CompositeServiceSlo(
                service="message_queue",
                local_score=0.97,
                min_pass_score=0.90,
                impact_weight=1.0,
            ),
            # Application
            CompositeServiceSlo(
                service="api_gateway",
                local_score=0.94,
                min_pass_score=0.95,
                impact_weight=3.0,
            ),
            CompositeServiceSlo(
                service="user_service",
                local_score=0.93,
                min_pass_score=0.90,
                impact_weight=2.5,
            ),
            CompositeServiceSlo(
                service="product_service",
                local_score=0.92,
                min_pass_score=0.90,
                impact_weight=2.5,
            ),
        ],
        dependencies=[
            # API Gateway depends on infrastructure
            CompositeDependencyEdge(
                dependency="auth_service",
                dependent="api_gateway",
                failure_penalty=0.40,
            ),
            CompositeDependencyEdge(
                dependency="rate_limiter",
                dependent="api_gateway",
                failure_penalty=0.20,
            ),
            CompositeDependencyEdge(
                dependency="observability",
                dependent="api_gateway",
                failure_penalty=0.05,  # Non-critical
            ),
            # Services depend on API Gateway + data layer
            CompositeDependencyEdge(
                dependency="api_gateway",
                dependent="user_service",
                failure_penalty=0.50,
            ),
            CompositeDependencyEdge(
                dependency="database",
                dependent="user_service",
                failure_penalty=0.60,
            ),
            CompositeDependencyEdge(
                dependency="cache",
                dependent="user_service",
                failure_penalty=0.20,
            ),
            CompositeDependencyEdge(
                dependency="api_gateway",
                dependent="product_service",
                failure_penalty=0.50,
            ),
            CompositeDependencyEdge(
                dependency="database",
                dependent="product_service",
                failure_penalty=0.60,
            ),
            CompositeDependencyEdge(
                dependency="message_queue",
                dependent="product_service",
                failure_penalty=0.15,
            ),
        ],
        global_min_pass_score=0.90,
    )

    result = evaluate_composite_slo_graph(graph)

    print(f"\nTopological Order (9 services evaluated in order):")
    for i, service in enumerate(result.topological_order, 1):
        print(f"  {i}. {service}")

    print(f"\nEvaluation Results:")
    print(f"\nInfrastructure Layer:")
    for svc in result.services:
        if svc.service in ["auth_service", "rate_limiter", "observability"]:
            print(f"  {svc.service:20s} {svc.local_score:.2f} "
                  f"→ {svc.effective_score:.2f} "
                  f"({'✓ PASS' if svc.pass else '✗ FAIL'})")

    print(f"\nData Layer:")
    for svc in result.services:
        if svc.service in ["cache", "database", "message_queue"]:
            print(f"  {svc.service:20s} {svc.local_score:.2f} "
                  f"→ {svc.effective_score:.2f} "
                  f"({'✓ PASS' if svc.pass else '✗ FAIL'})")

    print(f"\nApplication Layer:")
    for svc in result.services:
        if svc.service in ["api_gateway", "user_service", "product_service"]:
            status = "✓ PASS" if svc.pass else "✗ FAIL"
            adjusted = " (adjusted)" if svc.dependency_adjusted else ""
            print(f"  {svc.service:20s} {svc.local_score:.2f} "
                  f"→ {svc.effective_score:.2f} {status}{adjusted}")

    print(f"\nGlobal SLO: {result.global_slo:.2f} "
          f"({'✓ PASS' if result.global_pass else '✗ FAIL'})")

    print(f"\n💡 Note: API Gateway fails (0.94 < 0.95 threshold) despite no dep failures")
    print(f"   This is expected: service's own score can fail independent of deps")


# ============================================================================
# EXAMPLE 4: Failure Simulation Patterns
# ============================================================================
def example_failure_scenarios():
    """
    Systematically test various failure scenarios on a simple 3-service graph.

    Demonstrates: Test harness patterns, comparing outcomes
    """
    print("\n" + "=" * 80)
    print("EXAMPLE 4: Failure Simulation Patterns")
    print("=" * 80)

    # Base configuration
    base_services = [
        CompositeServiceSlo(
            service="web_server",
            local_score=0.96,
            min_pass_score=0.90,
            impact_weight=2.0,
        ),
        CompositeServiceSlo(
            service="app_server",
            local_score=0.95,
            min_pass_score=0.90,
            impact_weight=2.0,
        ),
        CompositeServiceSlo(
            service="database",
            local_score=0.97,
            min_pass_score=0.95,
            impact_weight=3.0,
        ),
    ]

    dependencies = [
        CompositeDependencyEdge(
            dependency="database",
            dependent="app_server",
            failure_penalty=0.50,
        ),
        CompositeDependencyEdge(
            dependency="app_server",
            dependent="web_server",
            failure_penalty=0.40,
        ),
    ]

    def evaluate_scenario(services, label):
        graph = CompositeSloGraph(
            services=services,
            dependencies=dependencies,
            global_min_pass_score=0.90,
        )
        result = evaluate_composite_slo_graph(graph)
        failures = sum(1 for s in result.services if not s.pass)
        return result, failures

    # Baseline
    print(f"\nScenario 1: Healthy System")
    result, failures = evaluate_scenario(base_services, "baseline")
    print(f"  Global SLO: {result.global_slo:.2f} | Failures: {failures}/3")

    # Database degradation
    print(f"\nScenario 2: Database Degrades (0.97 → 0.80)")
    degraded_services = [
        CompositeServiceSlo(
            service="web_server",
            local_score=0.96,
            min_pass_score=0.90,
            impact_weight=2.0,
        ),
        CompositeServiceSlo(
            service="app_server",
            local_score=0.95,
            min_pass_score=0.90,
            impact_weight=2.0,
        ),
        CompositeServiceSlo(
            service="database",
            local_score=0.80,  # Degraded
            min_pass_score=0.95,
            impact_weight=3.0,
        ),
    ]
    result, failures = evaluate_scenario(degraded_services, "db_degraded")
    print(f"  Global SLO: {result.global_slo:.2f} | Failures: {failures}/3")
    print(f"  └─ Cascaded: Database fail → App fail → Web fail")

    # Database total failure
    print(f"\nScenario 3: Database Total Failure (0.97 → 0.50)")
    failed_services = [
        CompositeServiceSlo(
            service="web_server",
            local_score=0.96,
            min_pass_score=0.90,
            impact_weight=2.0,
        ),
        CompositeServiceSlo(
            service="app_server",
            local_score=0.95,
            min_pass_score=0.90,
            impact_weight=2.0,
        ),
        CompositeServiceSlo(
            service="database",
            local_score=0.50,  # Failed
            min_pass_score=0.95,
            impact_weight=3.0,
        ),
    ]
    result, failures = evaluate_scenario(failed_services, "db_failed")
    print(f"  Global SLO: {result.global_slo:.2f} | Failures: {failures}/3")

    # Web server degrades (no downstream dependencies)
    print(f"\nScenario 4: Web Server Degrades (0.96 → 0.85)")
    web_degraded = [
        CompositeServiceSlo(
            service="web_server",
            local_score=0.85,  # Degraded but no one depends on it
            min_pass_score=0.90,
            impact_weight=2.0,
        ),
        CompositeServiceSlo(
            service="app_server",
            local_score=0.95,
            min_pass_score=0.90,
            impact_weight=2.0,
        ),
        CompositeServiceSlo(
            service="database",
            local_score=0.97,
            min_pass_score=0.95,
            impact_weight=3.0,
        ),
    ]
    result, failures = evaluate_scenario(web_degraded, "web_degraded")
    print(f"  Global SLO: {result.global_slo:.2f} | Failures: {failures}/3")
    print(f"  └─ Isolated: Only web fails, app & db unaffected")

    print(f"\n💡 Key Insight: Position in DAG matters!")
    print(f"   - Database failure (root) → cascades to all")
    print(f"   - Web failure (leaf) → isolated impact")


# ============================================================================
# EXAMPLE 5: Metrics Integration Pattern
# ============================================================================
def example_metrics_integration():
    """
    Demonstrate integration with live metrics system.

    Demonstrates: Building graphs from external data, JSON export
    """
    print("\n" + "=" * 80)
    print("EXAMPLE 5: Metrics Integration Pattern")
    print("=" * 80)

    # Simulated live metrics from monitoring system
    live_metrics = {
        "auth_service": 0.98,
        "cache_service": 0.97,
        "database": 0.96,
        "api_gateway": 0.95,
    }

    # Service configuration (typically from config file or service registry)
    service_config = {
        "auth_service": {"min_pass": 0.95, "weight": 1.5},
        "cache_service": {"min_pass": 0.90, "weight": 1.0},
        "database": {"min_pass": 0.95, "weight": 3.0},
        "api_gateway": {"min_pass": 0.95, "weight": 2.5},
    }

    # Dependency configuration
    dependency_config = [
        {"from": "auth_service", "to": "api_gateway", "penalty": 0.40},
        {"from": "cache_service", "to": "api_gateway", "penalty": 0.15},
        {"from": "database", "to": "api_gateway", "penalty": 0.50},
    ]

    # Build graph from configurations
    services = [
        CompositeServiceSlo(
            service=name,
            local_score=live_metrics[name],
            min_pass_score=service_config[name]["min_pass"],
            impact_weight=service_config[name]["weight"],
        )
        for name in live_metrics.keys()
    ]

    dependencies = [
        CompositeDependencyEdge(
            dependency=dep["from"],
            dependent=dep["to"],
            failure_penalty=dep["penalty"],
        )
        for dep in dependency_config
    ]

    graph = CompositeSloGraph(
        services=services,
        dependencies=dependencies,
        global_min_pass_score=0.92,
    )

    result = evaluate_composite_slo_graph(graph)

    print(f"\nBuilt graph from live metrics + config:")
    print(f"  Timestamp: <now>")
    print(f"  Services: {len(result.services)}")
    print(f"  Dependencies: {len(graph.dependencies)}")

    print(f"\nResults:")
    for svc in result.services:
        print(f"  {svc.service:20s} {svc.local_score:.2f} "
              f"→ {svc.effective_score:.2f} "
              f"({'✓ PASS' if svc.pass else '✗ FAIL'})")

    print(f"\nGlobal SLO: {result.global_slo:.2f} "
          f"({'✓ PASS' if result.global_pass else '✗ FAIL'})")

    # Export to JSON (e.g., for storage or visualization)
    result_json = {
        "timestamp": "<ISO timestamp>",
        "global_slo": float(result.global_slo),
        "global_pass": result.global_pass,
        "topological_order": result.topological_order,
        "services": [
            {
                "service": s.service,
                "local_score": float(s.local_score),
                "effective_score": float(s.effective_score),
                "min_pass_score": float(s.min_pass_score),
                "pass": s.pass,
                "dependency_adjusted": s.dependency_adjusted,
                "failed_dependencies": s.failed_dependencies,
            }
            for s in result.services
        ],
    }

    print(f"\nJSON Export (for storage/API response):")
    print(json.dumps(result_json, indent=2))


# ============================================================================
# Main Entry Point
# ============================================================================
def main():
    """Run all examples in sequence."""
    print("\n" + "=" * 80)
    print("COMPOSITE SLO DAG - COMPREHENSIVE EXAMPLES")
    print("=" * 80)
    print("\nThis script demonstrates real-world use cases for Composite SLO graphs.")
    print("Each example shows how failure in one service cascades to others.")

    example_ecommerce_checkout()
    example_multi_region()
    example_complex_dag()
    example_failure_scenarios()
    example_metrics_integration()

    print("\n" + "=" * 80)
    print("ALL EXAMPLES COMPLETED")
    print("=" * 80)
    print("\n✓ Composite DAG provides:")
    print("  • Automatic failure propagation across service dependencies")
    print("  • Deterministic topological evaluation order")
    print("  • Global SLO that reflects true system health")
    print("  • Weighted impact per service")
    print("  • Cycle detection and validation")
    print("\nFor more details, see docs/reference/composite-slo-dag.md")


if __name__ == "__main__":
    main()
