"""Examples demonstrating dashboard and CLI TUI usage."""

import asyncio
from datetime import datetime, timedelta

from neuralbudget import NeuralBudgetClient
from neuralbudget.dashboard import Dashboard, SloSnapshot
from neuralbudget.cli_tui import CliTui


# ============================================================================
# Example 1: Basic Dashboard Usage - Lightweight HTTP Server
# ============================================================================
def example_1_basic_dashboard():
    """Start a basic dashboard server on localhost."""
    print("\n" + "=" * 70)
    print("Example 1: Basic Dashboard Server")
    print("=" * 70)
    print(
        """
This example starts a lightweight HTTP server on port 8080 that you can
access from your browser at http://localhost:8080

The dashboard displays:
- Current SLO status and metrics
- Multi-window burn rates
- Alert history
- Budget exhaustion forecasts

Usage:
    dashboard = Dashboard()
    dashboard.run(host="127.0.0.1", port=8080)

Access at: http://localhost:8080
"""
    )
    # Uncomment to run:
    # dashboard = Dashboard()
    # dashboard.run(host="127.0.0.1", port=8080)


# ============================================================================
# Example 2: Dashboard with Custom Configuration
# ============================================================================
def example_2_dashboard_with_config():
    """Dashboard with custom NeuralBudgetClient configuration."""
    print("\n" + "=" * 70)
    print("Example 2: Dashboard with Custom Configuration")
    print("=" * 70)
    code = """
from neuralbudget import NeuralBudgetClient
from neuralbudget.dashboard import Dashboard

# Load custom config
client = NeuralBudgetClient()
client.load_config("config.yaml")

# Create dashboard with custom client
dashboard = Dashboard(
    client=client,
    host="0.0.0.0",  # Listen on all interfaces
    port=3000,
    title="Production SLO Dashboard"
)

# Add SLO snapshots from your evaluation results
dashboard.update_slo_snapshot(
    SloSnapshot(
        service_name="api-gateway",
        metric_name="availability",
        timestamp=datetime.utcnow().isoformat(),
        error_budget_remaining_percent=85.5,
        burn_rate_5m=0.1,
        burn_rate_30m=0.15,
        burn_rate_1h=0.2,
        burn_rate_6h=0.18,
        total_errors=150,
        total_requests=15000,
        error_rate_percent=1.0,
        severity="Ok",
        will_exhaust_budget=False,
        time_to_exhaustion_hours=None,
        last_alert_at=None,
        last_alert_severity=None,
    )
)

# Run on custom port
dashboard.run(host="0.0.0.0", port=3000)
"""
    print(code)


# ============================================================================
# Example 3: Dashboard Integration with Monitoring Loop
# ============================================================================
def example_3_dashboard_monitoring_loop():
    """Continuously evaluate SLOs and update dashboard."""
    print("\n" + "=" * 70)
    print("Example 3: Dashboard with Monitoring Loop")
    print("=" * 70)
    code = """
import asyncio
import threading
from neuralbudget import NeuralBudgetClient
from neuralbudget.dashboard import Dashboard, SloSnapshot
from datetime import datetime

async def monitoring_loop(dashboard, client):
    '''Periodically evaluate SLOs and update dashboard.'''
    while True:
        try:
            # Evaluate each configured SLO
            config = client.load_config()
            
            for service in config.get("services", []):
                # Evaluate the service
                result = client.evaluate(service["metrics"])
                
                # Update dashboard snapshot
                dashboard.update_slo_snapshot(
                    SloSnapshot(
                        service_name=service["name"],
                        metric_name=service["metrics"][0]["name"],
                        timestamp=datetime.utcnow().isoformat(),
                        error_budget_remaining_percent=result.get("budget_remaining", 100),
                        burn_rate_5m=result.get("burn_rate_5m", 0),
                        burn_rate_30m=result.get("burn_rate_30m", 0),
                        burn_rate_1h=result.get("burn_rate_1h", 0),
                        burn_rate_6h=result.get("burn_rate_6h", 0),
                        total_errors=result.get("errors", 0),
                        total_requests=result.get("requests", 0),
                        error_rate_percent=result.get("error_rate", 0),
                        severity=result.get("severity", "Ok"),
                        will_exhaust_budget=result.get("will_exhaust", False),
                        time_to_exhaustion_hours=result.get("ttee_hours"),
                        last_alert_at=result.get("last_alert_at"),
                        last_alert_severity=result.get("last_alert_severity"),
                    )
                )
            
            # Wait before next evaluation
            await asyncio.sleep(30)  # Evaluate every 30 seconds
            
        except Exception as e:
            print(f"Error in monitoring loop: {e}")
            await asyncio.sleep(60)

# Setup
client = NeuralBudgetClient()
client.load_config("config.yaml")
dashboard = Dashboard(client=client)

# Run monitoring loop in separate thread
monitor_thread = threading.Thread(
    target=lambda: asyncio.run(monitoring_loop(dashboard, client)),
    daemon=True
)
monitor_thread.start()

# Start dashboard server
dashboard.run(host="127.0.0.1", port=8080)
"""
    print(code)


# ============================================================================
# Example 4: CLI TUI - Terminal Dashboard
# ============================================================================
def example_4_cli_tui_basic():
    """Run CLI TUI for terminal-based monitoring."""
    print("\n" + "=" * 70)
    print("Example 4: CLI TUI - Terminal Dashboard")
    print("=" * 70)
    print(
        """
The CLI TUI provides a terminal-based dashboard accessible via:

Usage:
    python -m neuralbudget.cli_tui

Or programmatically:
    from neuralbudget.cli_tui import CliTui
    
    tui = CliTui()
    tui.run()  # Opens interactive demo

Features:
- Real-time SLO status
- Multi-window burn rate charts
- Alert history and escalation tracking
- Budget exhaustion forecasts
- Keyboard shortcuts for navigation
- No browser required - works over SSH

Keyboard Commands:
  r - Refresh dashboard
  f - Filter by service
  s - Sort by column
  p - Pause/resume auto-refresh
  q - Quit
  h - Help
"""
    )
    # Uncomment to run:
    # tui = CliTui()
    # tui.run()


# ============================================================================
# Example 5: CLI TUI with Custom Client
# ============================================================================
def example_5_cli_tui_custom():
    """CLI TUI with custom NeuralBudgetClient."""
    print("\n" + "=" * 70)
    print("Example 5: CLI TUI with Custom Configuration")
    print("=" * 70)
    code = """
from neuralbudget import NeuralBudgetClient
from neuralbudget.cli_tui import CliTui

# Load custom config
client = NeuralBudgetClient()
client.load_config("config.yaml")

# Create TUI with custom client
tui = CliTui(client=client)

# Run demo mode
tui.run_demo()  # Rich table display

# Or run full Textual TUI (requires textual package)
# tui.run_textual_app()
"""
    print(code)


# ============================================================================
# Example 6: Dashboard + Alert Integration
# ============================================================================
def example_6_dashboard_alerts():
    """Dashboard with alert recording."""
    print("\n" + "=" * 70)
    print("Example 6: Dashboard with Alert Recording")
    print("=" * 70)
    code = """
from neuralbudget import NeuralBudgetClient
from neuralbudget.dashboard import Dashboard
import asyncio

async def main():
    dashboard = Dashboard()
    
    # When an alert fires, record it on the dashboard
    async def on_alert_fired(service_name, metric_name, severity, message):
        response = await dashboard.app.post(
            "/api/alerts/record",
            json={
                "service_name": service_name,
                "metric_name": metric_name,
                "severity": severity,
                "message": message,
                "channels": ["slack", "pagerduty"],
            }
        )
        print(f"Alert recorded: {response}")
    
    # Trigger evaluation and recording
    await on_alert_fired(
        "api-gateway",
        "availability",
        "FastBurn",
        "Burn rate 8.5x detected on api-gateway"
    )

# Or run dashboard server
# dashboard = Dashboard()
# dashboard.run(port=8080)
"""
    print(code)


# ============================================================================
# Example 7: Docker Deployment
# ============================================================================
def example_7_docker_deployment():
    """Dashboard deployment in Docker."""
    print("\n" + "=" * 70)
    print("Example 7: Docker Deployment")
    print("=" * 70)
    dockerfile = """
FROM python:3.9-slim

WORKDIR /app

# Install dependencies
RUN pip install neuralbudget fastapi uvicorn

# Copy config
COPY config.yaml /app/config.yaml

# Create startup script
RUN cat > /app/start.py << 'EOF'
from neuralbudget.dashboard import Dashboard

dashboard = Dashboard(host="0.0.0.0", port=8080)
dashboard.run()
EOF

# Expose port
EXPOSE 8080

# Run dashboard
CMD ["python", "/app/start.py"]
"""
    print("Dockerfile:")
    print(dockerfile)

    docker_compose = """
version: '3.8'

services:
  neuralbudget-dashboard:
    build: .
    ports:
      - "8080:8080"
    volumes:
      - ./config.yaml:/app/config.yaml
    environment:
      - LOG_LEVEL=info
    restart: unless-stopped
"""
    print("\ndocker-compose.yml:")
    print(docker_compose)

    print("\nUsage:")
    print("  docker-compose up -d")
    print("  # Access at http://localhost:8080")


# ============================================================================
# Example 8: Multi-Service Dashboard
# ============================================================================
def example_8_multi_service_dashboard():
    """Dashboard monitoring multiple services."""
    print("\n" + "=" * 70)
    print("Example 8: Multi-Service Dashboard")
    print("=" * 70)
    code = """
from neuralbudget import NeuralBudgetClient
from neuralbudget.dashboard import Dashboard, SloSnapshot
from datetime import datetime

# Create dashboard
dashboard = Dashboard(title="Multi-Service SLO Dashboard")

# Add snapshots for multiple services
services = [
    {
        "name": "api-gateway",
        "metric": "availability",
        "budget": 85.5,
        "burn_1h": 0.2,
        "errors": 150,
        "requests": 15000,
        "severity": "Ok",
    },
    {
        "name": "payment-service",
        "metric": "latency",
        "budget": 42.1,
        "burn_1h": 2.5,
        "errors": 1200,
        "requests": 15000,
        "severity": "MediumBurn",
    },
    {
        "name": "auth-service",
        "metric": "error_rate",
        "budget": 15.0,
        "burn_1h": 8.9,
        "errors": 5400,
        "requests": 15000,
        "severity": "CriticalBurn",
    },
]

for svc in services:
    dashboard.update_slo_snapshot(
        SloSnapshot(
            service_name=svc["name"],
            metric_name=svc["metric"],
            timestamp=datetime.utcnow().isoformat(),
            error_budget_remaining_percent=svc["budget"],
            burn_rate_5m=svc["burn_1h"] * 0.75,
            burn_rate_30m=svc["burn_1h"] * 0.9,
            burn_rate_1h=svc["burn_1h"],
            burn_rate_6h=svc["burn_1h"] * 0.85,
            total_errors=svc["errors"],
            total_requests=svc["requests"],
            error_rate_percent=(svc["errors"] / svc["requests"]) * 100,
            severity=svc["severity"],
            will_exhaust_budget=svc["budget"] < 50,
            time_to_exhaustion_hours=svc["budget"] / svc["burn_1h"] if svc["burn_1h"] > 0 else None,
            last_alert_at=None,
            last_alert_severity=None,
        )
    )

# Run dashboard
dashboard.run(host="127.0.0.1", port=8080)
"""
    print(code)


# ============================================================================
# Example 9: Lightweight Mode (No External Server)
# ============================================================================
def example_9_lightweight_mode():
    """Dashboard in lightweight mode - local only."""
    print("\n" + "=" * 70)
    print("Example 9: Lightweight Mode (Local-Only)")
    print("=" * 70)
    code = """
from neuralbudget.dashboard import Dashboard

# Create dashboard with local-only binding
dashboard = Dashboard(
    host="127.0.0.1",  # Only localhost
    port=8080,
)

# Run - only accessible from this machine
dashboard.run()

# Benefits:
# - No firewall holes needed
# - Lightweight - runs on any machine
# - Secure by default (no external access)
# - Perfect for development and debugging
"""
    print(code)


# ============================================================================
# Example 10: Comparison - Dashboard vs CLI TUI
# ============================================================================
def example_10_comparison():
    """Comparison of dashboard vs CLI TUI."""
    print("\n" + "=" * 70)
    print("Example 10: Dashboard vs CLI TUI Comparison")
    print("=" * 70)
    comparison = """
┌─────────────────────┬──────────────────────┬──────────────────────┐
│ Feature             │ Web Dashboard        │ CLI TUI              │
├─────────────────────┼──────────────────────┼──────────────────────┤
│ Access              │ Browser (any device) │ Terminal (local/SSH)  │
│ Port exposure       │ Requires firewall    │ None (local only)     │
│ Mobile support      │ Yes                  │ No                    │
│ Requires browser    │ Yes                  │ No                    │
│ Real-time updates   │ 30s refresh          │ On demand             │
│ Interactive         │ Click/scroll         │ Keyboard shortcuts    │
│ Storage needed      │ Minimal              │ Minimal               │
│ Dependencies        │ FastAPI, Uvicorn     │ Rich, Textual (opt)   │
│ Use case            │ Team dashboards      │ Ops terminal tools    │
│ Over SSH            │ With reverse proxy   │ Direct                │
│ Best for            │ Non-technical users  │ Engineers             │
│ Learning curve      │ Familiar (web)       │ Shell-like            │
└─────────────────────┴──────────────────────┴──────────────────────┘

CHOOSE WEB DASHBOARD IF:
- Team needs visual overview
- Multiple users from different locations
- Non-technical users accessing
- Mobile access needed
- REST API integration important

CHOOSE CLI TUI IF:
- Engineers prefer terminal
- Running over SSH/remote
- No browser available
- Minimal dependencies
- Real-time monitoring from shell
- Keyboard-driven workflow
"""
    print(comparison)


# ============================================================================
# Main
# ============================================================================
def main():
    """Run all examples."""
    import sys

    examples = [
        ("1", "Basic Dashboard Usage", example_1_basic_dashboard),
        ("2", "Dashboard with Custom Configuration", example_2_dashboard_with_config),
        ("3", "Dashboard with Monitoring Loop", example_3_dashboard_monitoring_loop),
        ("4", "CLI TUI - Terminal Dashboard", example_4_cli_tui_basic),
        ("5", "CLI TUI with Custom Client", example_5_cli_tui_custom),
        ("6", "Dashboard with Alert Integration", example_6_dashboard_alerts),
        ("7", "Docker Deployment", example_7_docker_deployment),
        ("8", "Multi-Service Dashboard", example_8_multi_service_dashboard),
        ("9", "Lightweight Mode", example_9_lightweight_mode),
        ("10", "Comparison Dashboard vs TUI", example_10_comparison),
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
    print("NeuralBudget Dashboard & CLI TUI Examples")
    print("=" * 70)
    print("\nAvailable examples:\n")
    for num, title, _ in examples:
        print(f"  {num}. {title}")

    print("\n" + "=" * 70)
    print("Usage: python examples/dashboard_cli_examples.py <number>")
    print("Example: python examples/dashboard_cli_examples.py 1")
    print("=" * 70 + "\n")

    # Run first example by default
    print("Running Example 1...\n")
    example_1_basic_dashboard()


if __name__ == "__main__":
    main()
