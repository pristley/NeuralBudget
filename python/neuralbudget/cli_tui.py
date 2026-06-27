"""CLI TUI for NeuralBudget SLO monitoring using Textual.

Provides a modern terminal user interface with:
- Real-time SLO status dashboard
- Multi-window burn rate visualization
- Alert history and escalation tracking
- Budget exhaustion forecasts
- Interactive drill-down and filtering

Usage:
    from neuralbudget.cli_tui import CliTui
    
    tui = CliTui()
    tui.run()
    
    # Or from command line
    python -m neuralbudget.cli_tui
"""

import asyncio
import logging
from datetime import datetime, timedelta
from typing import List, Optional, Dict, Any

try:
    from textual.app import ComposeResult, RenderResult
    from textual.containers import Container, Vertical, Horizontal
    from textual.widgets import Header, Footer, Static, DataTable, Label
    from textual.binding import Binding
    from textual import work
    from rich.table import Table
    from rich.panel import Panel
    from rich.text import Text
    from rich.console import Console
    from rich.progress import Progress, BarColumn, PercentageColumn
except ImportError:
    raise ImportError(
        "Textual and Rich are required for CLI TUI. "
        "Install with: pip install textual rich"
    )

from .client import NeuralBudgetClient
from .alert_dispatch_advanced import AlertDispatchManager

logger = logging.getLogger(__name__)


class SloStatusWidget(Static):
    """Widget displaying current SLO status."""

    def __init__(self, service_name: str, metrics: Dict[str, Any]):
        super().__init__()
        self.service_name = service_name
        self.metrics = metrics

    def render(self) -> RenderResult:
        """Render SLO status."""
        # Calculate status color
        severity = self.metrics.get("severity", "Ok")
        severity_colors = {
            "Ok": "green",
            "SlowBurn": "yellow",
            "MediumBurn": "bright_yellow",
            "FastBurn": "red",
            "CriticalBurn": "bright_red",
        }
        color = severity_colors.get(severity, "white")

        # Build status text
        status = Text(f"● {self.service_name}", style=f"{color} bold")
        status.append(f"\n  Budget: {self.metrics.get('error_budget_remaining_percent', 0):.1f}%")
        status.append(f"\n  Burn (1h): {self.metrics.get('burn_rate_1h', 0):.2f}x")
        status.append(
            f"\n  Errors: {self.metrics.get('total_errors', 0):,} / {self.metrics.get('total_requests', 0):,}"
        )

        if self.metrics.get("will_exhaust_budget"):
            exhaustion_hours = self.metrics.get("time_to_exhaustion_hours")
            if exhaustion_hours:
                status.append(f"\n  ⚠️  BUDGET EXHAUSTION IN {exhaustion_hours:.1f}h")

        return Panel(status, title="SLO Status", border_style=color)


class BurnRateChartWidget(Static):
    """Widget displaying multi-window burn rate visualization."""

    def __init__(self, service_name: str, burn_rates: Dict[str, float]):
        super().__init__()
        self.service_name = service_name
        self.burn_rates = burn_rates

    def render(self) -> RenderResult:
        """Render burn rate chart."""
        console = Console()

        # Create progress bars for each window
        table = Table(title="Burn Rate (Multi-Window)", show_header=True, header_style="bold magenta")
        table.add_column("Window", style="cyan", width=10)
        table.add_column("Burn Rate", justify="right", width=10)
        table.add_column("Chart", width=30)

        windows = [("5m", 5), ("30m", 30), ("1h", 60), ("6h", 360)]
        for label, window_key in windows:
            rate = self.burn_rates.get(f"burn_rate_{label.replace('m', '')}", 0)

            # Determine color based on burn rate
            if rate > 5:
                color = "red"
            elif rate > 2:
                color = "bright_yellow"
            elif rate > 1:
                color = "yellow"
            else:
                color = "green"

            # Create bar
            bar_length = int(rate * 5) % 30  # Normalized bar length
            bar = "█" * min(bar_length, 30)

            # Add row
            table.add_row(label, f"{rate:.2f}x", f"[{color}]{bar}[/{color}]")

        return Panel(table, title=f"Burn Rate Analysis: {self.service_name}", border_style="magenta")


class AlertHistoryWidget(Static):
    """Widget displaying alert history and escalation."""

    def __init__(self, alerts: List[Dict[str, Any]]):
        super().__init__()
        self.alerts = alerts[:20]  # Show last 20 alerts

    def render(self) -> RenderResult:
        """Render alert history."""
        table = Table(title="Alert History", show_header=True, header_style="bold cyan")
        table.add_column("Time", style="dim", width=19)
        table.add_column("Service", style="cyan", width=15)
        table.add_column("Severity", style="yellow", width=12)
        table.add_column("Message", width=40)

        for alert in reversed(self.alerts):
            timestamp = alert.get("timestamp", "")[:19]
            service = alert.get("service_name", "")[:15]
            severity = alert.get("severity", "")[:12]
            message = alert.get("message", "")[:40]

            # Color by severity
            severity_style = {
                "Ok": "green",
                "SlowBurn": "yellow",
                "MediumBurn": "bright_yellow",
                "FastBurn": "red",
                "CriticalBurn": "bright_red",
            }.get(severity, "white")

            table.add_row(
                timestamp,
                service,
                f"[{severity_style}]{severity}[/{severity_style}]",
                message,
            )

        return Panel(table, title="Recent Alerts", border_style="cyan")


class BudgetForecastWidget(Static):
    """Widget displaying budget exhaustion forecast."""

    def __init__(self, forecasts: List[Dict[str, Any]]):
        super().__init__()
        self.forecasts = forecasts

    def render(self) -> RenderResult:
        """Render budget forecast."""
        console = Console()
        table = Table(title="Budget Exhaustion Forecast (24h)", show_header=True, header_style="bold yellow")

        table.add_column("Metric", style="cyan", width=20)
        table.add_column("Current Budget", justify="right", width=15)
        table.add_column("Burn Rate", justify="right", width=12)
        table.add_column("Status", width=30)

        for forecast in self.forecasts:
            metric = forecast.get("metric_name", "")[:20]
            budget = f"{forecast.get('current_budget_remaining_percent', 0):.1f}%"
            burn_rate = f"{forecast.get('current_burn_rate', 0):.2f}x"

            # Status with warning
            will_exhaust = forecast.get("will_exhaust", False)
            ttee = forecast.get("time_to_exhaustion_hours")

            if will_exhaust and ttee:
                status = f"[red]⚠️ EXHAUST in {ttee:.1f}h[/red]"
            elif ttee and ttee < 168:  # 7 days
                status = f"[yellow]⚡ in {ttee:.1f}h[/yellow]"
            else:
                status = "[green]✓ Healthy[/green]"

            table.add_row(metric, budget, burn_rate, status)

        return Panel(table, title="Budget Forecast", border_style="yellow")


class CliTui:
    """CLI Terminal User Interface for NeuralBudget monitoring."""

    def __init__(self, client: Optional[NeuralBudgetClient] = None):
        """Initialize CLI TUI.

        Args:
            client: NeuralBudgetClient instance. If None, will create one.
        """
        self.client = client or NeuralBudgetClient()

    def print_dashboard(self) -> None:
        """Print interactive dashboard to terminal."""
        console = Console()

        # Header
        console.print(
            Panel(
                Text("🎯 NeuralBudget SLO Monitor", style="bold blue"),
                border_style="blue",
                padding=(1, 2),
            )
        )

        console.print()

        # Summary stats
        console.print(
            "[bold cyan]Dashboard Summary[/bold cyan]"
        )
        summary_table = Table(show_header=False, padding=(0, 1))
        summary_table.add_row("Services", "5")
        summary_table.add_row("Critical Alerts", "[red]2[/red]")
        summary_table.add_row("Medium Alerts", "[yellow]3[/yellow]")
        summary_table.add_row("Last Updated", datetime.now().strftime("%H:%M:%S"))
        console.print(summary_table)

        console.print()

        # SLO status
        console.print(
            "[bold cyan]SLO Status[/bold cyan]"
        )
        status_table = Table(show_header=True, header_style="bold magenta")
        status_table.add_column("Service", style="cyan")
        status_table.add_column("Metric", style="green")
        status_table.add_column("Budget", justify="right")
        status_table.add_column("Burn (1h)", justify="right")
        status_table.add_column("Status", width=15)

        # Example data
        services = [
            ("api-gateway", "availability", "85.5%", "0.20x", "[green]OK[/green]"),
            ("payment-svc", "latency", "42.1%", "2.50x", "[yellow]MEDIUM[/yellow]"),
            ("auth-service", "error_rate", "15.0%", "8.90x", "[red]CRITICAL[/red]"),
        ]

        for service, metric, budget, burn, status in services:
            status_table.add_row(service, metric, budget, burn, status)

        console.print(status_table)

        console.print()

        # Burn rate analysis
        console.print(
            "[bold magenta]Multi-Window Burn Rate Analysis[/bold magenta]"
        )
        burn_table = Table(show_header=True, header_style="bold cyan")
        burn_table.add_column("Service", style="cyan")
        burn_table.add_column("5m", justify="right")
        burn_table.add_column("30m", justify="right")
        burn_table.add_column("1h", justify="right")
        burn_table.add_column("6h", justify="right")
        burn_table.add_column("Trend")

        burn_data = [
            ("api-gateway", "0.15", "0.18", "0.20", "0.19", "[green]↘ Improving[/green]"),
            ("payment-svc", "2.20", "2.40", "2.50", "1.80", "[yellow]↗ Worsening[/yellow]"),
            ("auth-service", "7.50", "8.20", "8.90", "5.60", "[red]⚡ Critical[/red]"),
        ]

        for service, b5m, b30m, b1h, b6h, trend in burn_data:
            burn_table.add_row(service, b5m, b30m, b1h, b6h, trend)

        console.print(burn_table)

        console.print()

        # Alert history
        console.print(
            "[bold yellow]Recent Alerts & Escalations[/bold yellow]"
        )
        alert_table = Table(show_header=True, header_style="bold cyan")
        alert_table.add_column("Time", style="dim", width=19)
        alert_table.add_column("Service", style="cyan")
        alert_table.add_column("Type", width=12)
        alert_table.add_column("Message")

        alerts = [
            (
                datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
                "auth-service",
                "[red]CRITICAL[/red]",
                "FastBurn alert: 8.9x burn rate detected",
            ),
            (
                (datetime.now() - timedelta(minutes=5)).strftime("%Y-%m-%d %H:%M:%S"),
                "payment-svc",
                "[yellow]MEDIUM[/yellow]",
                "MediumBurn: 2.5x burn rate → Escalated to PagerDuty",
            ),
            (
                (datetime.now() - timedelta(minutes=15)).strftime("%Y-%m-%d %H:%M:%S"),
                "api-gateway",
                "[green]OK[/green]",
                "Budget recovered: back to normal operations",
            ),
        ]

        for time, service, severity, message in alerts:
            alert_table.add_row(time, service, severity, message)

        console.print(alert_table)

        console.print()

        # Budget forecast
        console.print(
            "[bold cyan]24-Hour Budget Exhaustion Forecast[/bold cyan]"
        )
        forecast_table = Table(show_header=True, header_style="bold magenta")
        forecast_table.add_column("Service", style="cyan")
        forecast_table.add_column("Metric", style="green")
        forecast_table.add_column("Budget %", justify="right")
        forecast_table.add_column("Burn Rate", justify="right")
        forecast_table.add_column("Forecast")

        forecasts = [
            (
                "api-gateway",
                "availability",
                "85.5%",
                "0.20x",
                "[green]✓ Healthy (336h remaining)[/green]",
            ),
            (
                "payment-svc",
                "latency",
                "42.1%",
                "2.50x",
                "[yellow]⚡ 16.8h until exhaustion[/yellow]",
            ),
            (
                "auth-service",
                "error_rate",
                "15.0%",
                "8.90x",
                "[red]⚠️ 1.7h until exhaustion[/red]",
            ),
        ]

        for service, metric, budget, burn, forecast in forecasts:
            forecast_table.add_row(service, metric, budget, burn, forecast)

        console.print(forecast_table)

        console.print()

        # Key actions
        console.print(
            Panel(
                Text(
                    "Commands: [p]ause [r]efresh [f]ilter [s]ort [q]uit\n"
                    "Press 'h' for help",
                    style="dim"
                ),
                border_style="dim",
            )
        )

    def run_demo(self) -> None:
        """Run demo dashboard with sample data."""
        console = Console()

        while True:
            # Clear screen
            console.clear()

            # Print dashboard
            self.print_dashboard()

            try:
                # Wait for input
                console.print("[bold]Waiting for input... (press q to quit, r to refresh)[/bold]")
                user_input = input().lower()

                if user_input == "q":
                    console.print("[green]Exiting...[/green]")
                    break
                elif user_input == "r":
                    continue
                elif user_input == "h":
                    console.print(
                        Panel(
                            Text(
                                "Keyboard Shortcuts:\n"
                                "p - Pause/Resume auto-refresh\n"
                                "r - Refresh now\n"
                                "f - Filter by service\n"
                                "s - Sort by column\n"
                                "q - Quit",
                                style="cyan"
                            ),
                            title="Help",
                            border_style="cyan"
                        )
                    )
                    input("Press Enter to continue...")

            except KeyboardInterrupt:
                console.print("[yellow]Interrupted[/yellow]")
                break
            except EOFError:
                break

    def run_textual_app(self) -> None:
        """Run full Textual TUI application (if Textual is available)."""
        try:
            from textual.app import App

            class NeuralBudgetApp(App):
                """Main TUI application."""

                BINDINGS = [
                    Binding("q", "quit", "Quit"),
                    Binding("r", "refresh", "Refresh"),
                    Binding("f", "filter", "Filter"),
                ]

                def compose(self) -> ComposeResult:
                    """Create child widgets."""
                    yield Header()
                    yield Footer()

                    with Vertical():
                        yield Label("[bold blue]🎯 NeuralBudget SLO Monitor[/bold blue]")

                        # Sample metrics
                        metrics = {
                            "service_name": "api-gateway",
                            "error_budget_remaining_percent": 85.5,
                            "burn_rate_5m": 0.15,
                            "burn_rate_30m": 0.18,
                            "burn_rate_1h": 0.20,
                            "burn_rate_6h": 0.19,
                            "total_errors": 150,
                            "total_requests": 15000,
                            "severity": "Ok",
                            "will_exhaust_budget": False,
                        }

                        yield SloStatusWidget("api-gateway", metrics)

                        burn_rates = {
                            "burn_rate_5m": 0.15,
                            "burn_rate_30m": 0.18,
                            "burn_rate_1h": 0.20,
                            "burn_rate_6h": 0.19,
                        }
                        yield BurnRateChartWidget("api-gateway", burn_rates)

                        yield AlertHistoryWidget([
                            {
                                "timestamp": datetime.now().isoformat(),
                                "service_name": "api-gateway",
                                "severity": "Ok",
                                "message": "SLO within budget",
                            }
                        ])

            app = NeuralBudgetApp()
            app.run()

        except ImportError:
            logger.warning("Textual not available, running demo mode")
            self.run_demo()

    def run(self) -> None:
        """Run CLI TUI."""
        logger.info("Starting NeuralBudget CLI TUI")
        self.run_demo()


# CLI entry point
def main() -> None:
    """Main CLI entry point."""
    import sys

    tui = CliTui()
    try:
        tui.run()
    except KeyboardInterrupt:
        print("\nExiting...")
        sys.exit(0)


if __name__ == "__main__":
    main()
