"""Examples demonstrating GenAI telemetry connectors.

Shows how to:
1. Use individual connectors (OpenAI, Anthropic, vLLM, Triton)
2. Integrate with NeuralBudgetClient
3. Feed GenAI metrics into SLO evaluation
4. Monitor multiple GenAI services
5. Set up continuous monitoring
"""

import asyncio
import time
from datetime import datetime

from neuralbudget import NeuralBudgetClient
from neuralbudget.genai_connectors_base import MockGenAIConnector, ConnectorRegistry
from neuralbudget.genai_connectors_api import OpenAIConnector, AnthropicConnector
from neuralbudget.genai_connectors_inference import VLLMConnector, TritonConnector


# ============================================================================
# Example 1: Using OpenAI Connector
# ============================================================================
def example_1_openai_connector():
    """Get metrics from OpenAI API."""
    print("\n" + "=" * 70)
    print("Example 1: OpenAI Connector")
    print("=" * 70)
    code = """
from neuralbudget.genai_connectors_api import OpenAIConnector

# Create connector
openai = OpenAIConnector(api_key="sk-your-api-key")

# Get all metrics
metrics = openai.get_all_metrics(model="gpt-4")

print(f"Timestamp: {metrics.timestamp}")
print(f"Model: {metrics.model}")
print(f"Provider: {metrics.provider}")
print(f"Token usage: {metrics.tokens.total_tokens} total")
print(f"  - Prompt: {metrics.tokens.prompt_tokens}")
print(f"  - Completion: {metrics.tokens.completion_tokens}")
print(f"Cost: ${metrics.costs.total_cost_usd:.4f}")
print(f"Requests: {metrics.request_count}")
print(f"Success rate: {metrics.success_count}/{metrics.request_count}")
print(f"Throughput: {metrics.throughput_rps:.2f} req/s")

# Get individual metrics
usage = openai.get_usage_metrics(hours_back=24)
costs = openai.get_cost_metrics(hours_back=24)
latency = openai.get_latency_metrics()
errors = openai.get_error_metrics()

print(f"\\nLatency (1h): {latency.latency.mean_latency_ms:.0f}ms (p99: {latency.latency.p99_latency_ms:.0f}ms)")
print(f"Error rate: {errors.errors.error_rate_percent:.2f}%")

# Get metrics history
history = openai.get_metrics_history(limit=10)
print(f"\\nMetrics history: {len(history)} entries")

# Health check
healthy = openai.health_check()
print(f"Connector health: {'✓ OK' if healthy else '✗ FAILED'}")
"""
    print(code)


# ============================================================================
# Example 2: Using Anthropic Connector
# ============================================================================
def example_2_anthropic_connector():
    """Get metrics from Anthropic API."""
    print("\n" + "=" * 70)
    print("Example 2: Anthropic Connector")
    print("=" * 70)
    code = """
from neuralbudget.genai_connectors_api import AnthropicConnector

# Create connector
anthropic = AnthropicConnector(api_key="ant-your-api-key")

# Get all metrics
metrics = anthropic.get_all_metrics(model="claude-3-opus")

print(f"Provider: {metrics.provider}")
print(f"Model: {metrics.model}")
print(f"Total tokens: {metrics.tokens.total_tokens}")
print(f"Cost: ${metrics.costs.total_cost_usd:.4f}")
print(f"Throughput: {metrics.throughput_rps:.2f} req/s")
print(f"Availability: {metrics.availability_percent:.1f}%")

# Compare pricing across models
models = ["claude-3-haiku", "claude-3-sonnet", "claude-3-opus"]
for model in models:
    metrics = anthropic.get_cost_metrics(model=model, hours_back=1)
    print(f"{model}: ${metrics.costs.total_cost_usd:.4f}")
"""
    print(code)


# ============================================================================
# Example 3: Using vLLM Connector
# ============================================================================
def example_3_vllm_connector():
    """Get metrics from vLLM server."""
    print("\n" + "=" * 70)
    print("Example 3: vLLM Connector")
    print("=" * 70)
    code = """
from neuralbudget.genai_connectors_inference import VLLMConnector

# Create connector to local vLLM server
vllm = VLLMConnector(
    endpoint="http://localhost:8000",
    model="meta-llama/Llama-2-7b-hf"
)

# Get all metrics
metrics = vllm.get_all_metrics()

print(f"vLLM Server: {metrics.provider}")
print(f"Model: {metrics.model}")
print(f"Requests: {metrics.request_count}")
print(f"Success rate: {metrics.availability_percent:.1f}%")
print(f"Throughput: {metrics.throughput_rps:.2f} req/s")
print(f"Latency (p99): {metrics.latency.p99_latency_ms:.0f}ms")

# Check GPU utilization
print(f"\\nGPU cache: {metrics.metadata.get('gpu_cache_usage', 0):.1f}%")
print(f"CPU cache: {metrics.metadata.get('cpu_cache_usage', 0):.1f}%")
print(f"Preemptions: {metrics.metadata.get('preemptions', 0)}")

# Monitor for changes
prev_latency = metrics.latency.mean_latency_ms
for i in range(5):
    time.sleep(5)
    new_metrics = vllm.get_all_metrics()
    new_latency = new_metrics.latency.mean_latency_ms
    delta = new_latency - prev_latency
    print(f"Latency: {new_latency:.0f}ms ({delta:+.0f}ms)")
    prev_latency = new_latency
"""
    print(code)


# ============================================================================
# Example 4: Using Triton Connector
# ============================================================================
def example_4_triton_connector():
    """Get metrics from Triton server."""
    print("\n" + "=" * 70)
    print("Example 4: Triton Inference Server Connector")
    print("=" * 70)
    code = """
from neuralbudget.genai_connectors_inference import TritonConnector

# Create connector to Triton server
triton = TritonConnector(
    endpoint="http://localhost:8002",
    model="gpt-2"
)

# Get all metrics
metrics = triton.get_all_metrics()

print(f"Triton Server: {metrics.provider}")
print(f"Total inferences: {metrics.request_count}")
print(f"Successful: {metrics.success_count}")
print(f"Failed: {metrics.metadata.get('failed_inferences', 0)}")
print(f"Success rate: {metrics.availability_percent:.1f}%")
print(f"Throughput: {metrics.throughput_rps:.2f} inferences/s")

# Latency analysis
print(f"\\nLatency statistics:")
print(f"  Mean: {metrics.latency.mean_latency_ms:.0f}ms")
print(f"  P50:  {metrics.latency.p50_latency_ms:.0f}ms")
print(f"  P95:  {metrics.latency.p95_latency_ms:.0f}ms")
print(f"  P99:  {metrics.latency.p99_latency_ms:.0f}ms")
print(f"  Max:  {metrics.latency.max_latency_ms:.0f}ms")

# Error analysis
print(f"\\nError metrics:")
print(f"  Count: {metrics.errors.error_count}")
print(f"  Timeouts: {metrics.errors.timeout_errors}")
print(f"  Server errors: {metrics.errors.server_errors}")
print(f"  Rate: {metrics.errors.error_rate_percent:.2f}%")
"""
    print(code)


# ============================================================================
# Example 5: Connector Registry
# ============================================================================
def example_5_connector_registry():
    """Use connector registry to manage multiple connectors."""
    print("\n" + "=" * 70)
    print("Example 5: Connector Registry")
    print("=" * 70)
    code = """
from neuralbudget.genai_connectors_base import ConnectorRegistry, MockGenAIConnector
from neuralbudget.genai_connectors_api import OpenAIConnector
from neuralbudget.genai_connectors_inference import VLLMConnector

# Register connectors
ConnectorRegistry.register("openai-gpt4", 
    OpenAIConnector(api_key="sk-..."))
ConnectorRegistry.register("vllm-local",
    VLLMConnector(endpoint="http://localhost:8000"))

# List registered connectors
print(f"Registered connectors: {ConnectorRegistry.list()}")

# Get specific connector
openai = ConnectorRegistry.get("openai-gpt4")
metrics = openai.get_all_metrics()
print(f"OpenAI metrics: {metrics.request_count} requests")

# Get all connectors
connectors = ConnectorRegistry.get_all()
for name, connector in connectors.items():
    print(f"{name}: {connector.provider}")

# Collect metrics from all
for name in ConnectorRegistry.list():
    connector = ConnectorRegistry.get(name)
    try:
        metrics = connector.get_all_metrics()
        print(f"{name}: {metrics.availability_percent:.1f}% available")
    except Exception as e:
        print(f"{name}: ERROR - {e}")
"""
    print(code)


# ============================================================================
# Example 6: Integration with NeuralBudgetClient
# ============================================================================
def example_6_client_integration():
    """Integrate GenAI connectors with NeuralBudgetClient."""
    print("\n" + "=" * 70)
    print("Example 6: Integration with NeuralBudgetClient")
    print("=" * 70)
    code = """
from neuralbudget import NeuralBudgetClient
from neuralbudget.genai_connectors_api import OpenAIConnector
from neuralbudget.convenience import GenAiSloProfile, GenAiEvaluationResult
from datetime import datetime

# Create client
client = NeuralBudgetClient()
client.load_config("config.yaml")

# Create connectors
openai = OpenAIConnector(api_key="sk-...")

# Get GenAI metrics from connector
genai_metrics = openai.get_all_metrics()

# Create GenAI SLO profile
profile = GenAiSloProfile(
    availability_target=99.5,
    latency_p99_target_ms=500,
    cost_limit_usd_per_hour=100,
)

# Prepare metric data for evaluation
metric_data = {
    "genai_requests": genai_metrics.request_count,
    "genai_errors": genai_metrics.errors.error_count if genai_metrics.errors else 0,
    "genai_latency_p99_ms": genai_metrics.latency.p99_latency_ms if genai_metrics.latency else 0,
    "genai_cost_usd": genai_metrics.costs.total_cost_usd,
    "genai_tokens": genai_metrics.tokens.total_tokens,
}

# Evaluate against SLO
result = client.evaluate(metric_data, profile=profile)
print(f"SLO Evaluation Result:")
print(f"  Availability: {result['metrics']['availability']:.1f}% (target: {profile.availability_target}%)")
print(f"  Latency P99: {result['metrics']['latency_p99_ms']:.0f}ms (target: {profile.latency_p99_target_ms}ms)")
print(f"  Cost: ${result['metrics']['cost_usd']:.2f} (limit: ${profile.cost_limit_usd_per_hour}/h)")
print(f"  Status: {result['status']}")

# Track over time
metrics_history = []
for i in range(10):
    metrics = openai.get_all_metrics()
    metrics_history.append(metrics)
    result = client.evaluate(metric_data, profile=profile)
    print(f"[{i}] Availability: {result['metrics']['availability']:.1f}%")
    time.sleep(5)
"""
    print(code)


# ============================================================================
# Example 7: Multi-Service GenAI Monitoring
# ============================================================================
def example_7_multi_service():
    """Monitor multiple GenAI services simultaneously."""
    print("\n" + "=" * 70)
    print("Example 7: Multi-Service GenAI Monitoring")
    print("=" * 70)
    code = """
from neuralbudget.genai_connectors_api import OpenAIConnector, AnthropicConnector
from neuralbudget.genai_connectors_inference import VLLMConnector
import time

# Create connectors for multiple services
services = {
    "gpt4_prod": OpenAIConnector(api_key="sk-..."),
    "claude_prod": AnthropicConnector(api_key="ant-..."),
    "llama_internal": VLLMConnector(endpoint="http://prod-vllm:8000"),
}

# Monitor all services
print("Service Metrics Snapshot:")
print("-" * 80)

for service_name, connector in services.items():
    try:
        metrics = connector.get_all_metrics()
        
        print(f"\\n{service_name}:")
        print(f"  Provider: {metrics.provider}")
        print(f"  Requests: {metrics.request_count}")
        print(f"  Success: {metrics.availability_percent:.1f}%")
        print(f"  Latency (p99): {metrics.latency.p99_latency_ms:.0f}ms")
        print(f"  Cost: ${metrics.costs.total_cost_usd:.2f}")
        print(f"  Error rate: {metrics.errors.error_rate_percent:.2f}%" 
              if metrics.errors else "  Error rate: N/A")
    except Exception as e:
        print(f"\\n{service_name}: ERROR - {e}")

# Compare costs across services
print("\\n" + "=" * 80)
print("Cost Comparison (Last 24h):")
for service_name, connector in services.items():
    try:
        costs = connector.get_cost_metrics(hours_back=24)
        print(f"  {service_name}: ${costs.costs.total_cost_usd:.2f}")
    except:
        pass

# Find service with best SLO adherence
print("\\nSLO Adherence Ranking:")
slo_scores = []
for service_name, connector in services.items():
    try:
        metrics = connector.get_all_metrics()
        # Simple SLO score: availability * (1 - error_rate/100) * (500/latency)
        score = (
            (metrics.availability_percent / 100) *
            (1 - (metrics.errors.error_rate_percent if metrics.errors else 0) / 100) *
            (500 / max(metrics.latency.p99_latency_ms if metrics.latency else 500, 1))
        )
        slo_scores.append((service_name, score))
    except:
        pass

for service_name, score in sorted(slo_scores, key=lambda x: x[1], reverse=True):
    print(f"  {service_name}: {score:.3f}")
"""
    print(code)


# ============================================================================
# Example 8: Real-Time Monitoring Dashboard
# ============================================================================
def example_8_monitoring_dashboard():
    """Build real-time monitoring dashboard for GenAI services."""
    print("\n" + "=" * 70)
    print("Example 8: Real-Time Monitoring Dashboard")
    print("=" * 70)
    code = """
from neuralbudget.genai_connectors_base import MockGenAIConnector, ConnectorRegistry
import time
from rich.console import Console
from rich.table import Table
from rich.live import Live

# Setup connectors
ConnectorRegistry.register("openai", MockGenAIConnector("gpt-4"))
ConnectorRegistry.register("anthropic", MockGenAIConnector("claude-3"))
ConnectorRegistry.register("vllm", MockGenAIConnector("llama-2"))

def get_status_table():
    '''Build live status table'''
    table = Table(title="GenAI Services Status")
    table.add_column("Service", style="cyan")
    table.add_column("Requests", justify="right")
    table.add_column("Availability", justify="right", style="green")
    table.add_column("Latency P99", justify="right")
    table.add_column("Cost/h", justify="right", style="yellow")
    table.add_column("Status", justify="center")

    for name in ConnectorRegistry.list():
        connector = ConnectorRegistry.get(name)
        metrics = connector.get_all_metrics()
        
        status = "✓" if metrics.availability_percent > 99 else "⚠"
        
        table.add_row(
            name,
            str(metrics.request_count),
            f"{metrics.availability_percent:.1f}%",
            f"{metrics.latency.p99_latency_ms:.0f}ms" if metrics.latency else "N/A",
            f"${metrics.costs.total_cost_usd:.2f}",
            status
        )
    
    return table

# Live dashboard
console = Console()
try:
    with Live(console=console, refresh_per_second=1) as live:
        for i in range(60):
            live.update(get_status_table())
            time.sleep(1)
except KeyboardInterrupt:
    console.print("[yellow]Dashboard stopped[/yellow]")
"""
    print(code)


# ============================================================================
# Main
# ============================================================================
def main():
    """Run all examples."""
    import sys

    examples = [
        ("1", "OpenAI Connector", example_1_openai_connector),
        ("2", "Anthropic Connector", example_2_anthropic_connector),
        ("3", "vLLM Connector", example_3_vllm_connector),
        ("4", "Triton Connector", example_4_triton_connector),
        ("5", "Connector Registry", example_5_connector_registry),
        ("6", "NeuralBudgetClient Integration", example_6_client_integration),
        ("7", "Multi-Service Monitoring", example_7_multi_service),
        ("8", "Real-Time Dashboard", example_8_monitoring_dashboard),
    ]

    if len(sys.argv) > 1:
        example_num = sys.argv[1]
        for num, title, func in examples:
            if num == example_num:
                func()
                return
        print(f"Example {example_num} not found")
        sys.exit(1)

    # Show menu
    print("\n" + "=" * 70)
    print("GenAI Telemetry Connectors - Examples")
    print("=" * 70)
    print("\nAvailable examples:\n")
    for num, title, _ in examples:
        print(f"  {num}. {title}")

    print("\n" + "=" * 70)
    print("Usage: python examples/python/genai_connector_examples.py <number>")
    print("Example: python examples/python/genai_connector_examples.py 1")
    print("=" * 70 + "\n")


if __name__ == "__main__":
    main()
