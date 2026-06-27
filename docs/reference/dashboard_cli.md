# Dashboard & CLI TUI API Reference

## Table of Contents

1. [Dashboard Class](#dashboard-class)
2. [CLI TUI Class](#cli-tui-class)
3. [Data Models](#data-models)
4. [API Endpoints](#api-endpoints)
5. [Usage Patterns](#usage-patterns)
6. [Examples](#examples)

## Dashboard Class

### Dashboard

Lightweight FastAPI-based web dashboard for SLO monitoring.

```python
from neuralbudget.dashboard import Dashboard
```

#### Constructor

```python
Dashboard(
    client: Optional[NeuralBudgetClient] = None,
    host: str = "127.0.0.1",
    port: int = 8080,
    title: str = "NeuralBudget Dashboard",
    enable_cors: bool = True,
) -> Dashboard
```

**Parameters:**
- `client` (NeuralBudgetClient): NeuralBudgetClient instance. Creates new one if not provided.
- `host` (str): Server host. Default: "127.0.0.1" (local only)
- `port` (int): Server port. Default: 8080
- `title` (str): Dashboard title shown in UI
- `enable_cors` (bool): Enable CORS middleware for development

**Example:**
```python
from neuralbudget import NeuralBudgetClient
from neuralbudget.dashboard import Dashboard

client = NeuralBudgetClient()
dashboard = Dashboard(
    client=client,
    host="127.0.0.1",
    port=8080,
    title="Production SLO Dashboard"
)
```

#### Methods

##### run()

```python
def run(
    host: Optional[str] = None,
    port: Optional[int] = None,
    reload: bool = False,
) -> None
```

Run dashboard server (blocking).

**Parameters:**
- `host` (str): Override server host
- `port` (int): Override server port
- `reload` (bool): Enable auto-reload on file changes (development)

**Returns:** None

**Raises:** OSError if port is in use

**Example:**
```python
dashboard.run(host="127.0.0.1", port=8080)
# Blocks until KeyboardInterrupt or server error
```

##### run_async()

```python
async def run_async(
    host: Optional[str] = None,
    port: Optional[int] = None,
) -> None
```

Run dashboard server (async).

**Parameters:**
- `host` (str): Override server host
- `port` (int): Override server port

**Returns:** Coroutine (awaitable)

**Example:**
```python
import asyncio

async def main():
    dashboard = Dashboard()
    await dashboard.run_async(port=8080)

asyncio.run(main())
```

##### update_slo_snapshot()

```python
def update_slo_snapshot(self, snapshot: SloSnapshot) -> None
```

Update SLO snapshot data.

**Parameters:**
- `snapshot` (SloSnapshot): SLO snapshot to store/update

**Returns:** None

**Example:**
```python
from neuralbudget.dashboard import SloSnapshot
from datetime import datetime

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
```

### SloSnapshot

Data class representing current SLO status.

```python
@dataclass
class SloSnapshot:
    service_name: str
    metric_name: str
    timestamp: str
    error_budget_remaining_percent: float
    burn_rate_5m: float
    burn_rate_30m: float
    burn_rate_1h: float
    burn_rate_6h: float
    total_errors: int
    total_requests: int
    error_rate_percent: float
    severity: str
    will_exhaust_budget: bool
    time_to_exhaustion_hours: Optional[float]
    last_alert_at: Optional[str]
    last_alert_severity: Optional[str]
```

**Attributes:**
- `service_name` (str): Service identifier
- `metric_name` (str): Metric being measured
- `timestamp` (str): ISO 8601 timestamp of snapshot
- `error_budget_remaining_percent` (float): 0-100, remaining budget
- `burn_rate_5m` (float): 5-minute burn rate (1.0 = consuming 100%/month)
- `burn_rate_30m` (float): 30-minute burn rate
- `burn_rate_1h` (float): 1-hour burn rate
- `burn_rate_6h` (float): 6-hour burn rate
- `total_errors` (int): Cumulative error count
- `total_requests` (int): Cumulative request count
- `error_rate_percent` (float): Current error rate percentage
- `severity` (str): One of: Ok, SlowBurn, MediumBurn, FastBurn, CriticalBurn
- `will_exhaust_budget` (bool): Forecast: budget exhaustion within 24h?
- `time_to_exhaustion_hours` (Optional[float]): Hours until budget exhaustion
- `last_alert_at` (Optional[str]): ISO timestamp of last alert
- `last_alert_severity` (Optional[str]): Severity of last alert

### AlertEvent

Data class representing historical alert event.

```python
@dataclass
class AlertEvent:
    timestamp: str
    service_name: str
    metric_name: str
    alert_type: str
    severity: str
    message: str
    channels: List[str]
    status: str
```

**Attributes:**
- `timestamp` (str): ISO 8601 timestamp
- `service_name` (str): Service triggering alert
- `metric_name` (str): Metric that violated SLO
- `alert_type` (str): One of: violation, escalation, recovery
- `severity` (str): One of: Ok, SlowBurn, MediumBurn, FastBurn, CriticalBurn
- `message` (str): Human-readable alert message
- `channels` (List[str]): Notification channels (e.g., ["slack", "pagerduty"])
- `status` (str): One of: sent, failed, deduped

## CLI TUI Class

### CliTui

Terminal User Interface for SLO monitoring.

```python
from neuralbudget.cli_tui import CliTui
```

#### Constructor

```python
CliTui(client: Optional[NeuralBudgetClient] = None) -> CliTui
```

**Parameters:**
- `client` (NeuralBudgetClient): NeuralBudgetClient instance. Creates new one if not provided.

**Example:**
```python
from neuralbudget import NeuralBudgetClient
from neuralbudget.cli_tui import CliTui

client = NeuralBudgetClient()
tui = CliTui(client=client)
```

#### Methods

##### run()

```python
def run(self) -> None
```

Run interactive TUI.

**Returns:** None

**Example:**
```python
tui = CliTui()
tui.run()
```

##### run_demo()

```python
def run_demo(self) -> None
```

Run demo dashboard with Rich table formatting.

**Returns:** None

**Features:**
- No Textual dependency required
- Rich table-based display with colors
- Interactive keyboard controls

**Example:**
```python
tui = CliTui()
tui.run_demo()
```

##### run_textual_app()

```python
def run_textual_app(self) -> None
```

Run full Textual interactive application.

**Returns:** None

**Requirements:** Textual package (`pip install textual`)

**Example:**
```python
tui = CliTui()
tui.run_textual_app()
```

##### print_dashboard()

```python
def print_dashboard(self) -> None
```

Print dashboard to terminal (non-interactive).

**Returns:** None

**Example:**
```python
tui = CliTui()
tui.print_dashboard()
```

## API Endpoints

### GET /api/health

Health check endpoint.

**Response:**
```json
{
  "status": "ok",
  "timestamp": "2026-06-27T10:30:00.000Z",
  "version": "1.0"
}
```

**Status Codes:**
- 200: Service is healthy

### GET /api/status

Get overall dashboard status.

**Response:**
```json
{
  "timestamp": "2026-06-27T10:30:00.000Z",
  "slos_evaluated": 5,
  "alerts_total": 12,
  "critical_alerts": 2,
  "medium_alerts": 3,
  "uptime_minutes": 1440
}
```

**Response Fields:**
- `slos_evaluated` (int): Number of services being monitored
- `critical_alerts` (int): Count of critical severity alerts
- `medium_alerts` (int): Count of medium/fast burn alerts
- `alerts_total` (int): Total alert events recorded

### GET /api/slos

Get all SLO snapshots.

**Response:**
```json
{
  "timestamp": "2026-06-27T10:30:00.000Z",
  "slos": [
    {
      "service_name": "api-gateway",
      "metric_name": "availability",
      "timestamp": "2026-06-27T10:30:00.000Z",
      "error_budget_remaining_percent": 85.5,
      "burn_rate_5m": 0.15,
      "burn_rate_30m": 0.18,
      "burn_rate_1h": 0.20,
      "burn_rate_6h": 0.19,
      "total_errors": 150,
      "total_requests": 15000,
      "error_rate_percent": 1.0,
      "severity": "Ok",
      "will_exhaust_budget": false,
      "time_to_exhaustion_hours": null,
      "last_alert_at": null,
      "last_alert_severity": null
    }
  ]
}
```

### GET /api/slos/{service_name}

Get SLO data for specific service.

**Path Parameters:**
- `service_name` (str): Service identifier

**Response:**
```json
{
  "service_name": "api-gateway",
  "slos": [...]
}
```

**Status Codes:**
- 200: Service found
- 404: Service not found

### GET /api/alerts

Get alert history.

**Query Parameters:**
- `limit` (int, optional): Maximum alerts to return. Default: 100
- `severity` (str, optional): Filter by severity (Ok, SlowBurn, MediumBurn, FastBurn, CriticalBurn)

**Response:**
```json
{
  "timestamp": "2026-06-27T10:30:00.000Z",
  "alerts": [
    {
      "timestamp": "2026-06-27T10:25:30.000Z",
      "service_name": "api-gateway",
      "metric_name": "availability",
      "alert_type": "violation",
      "severity": "FastBurn",
      "message": "FastBurn alert: 8.5x burn rate",
      "channels": ["slack", "pagerduty"],
      "status": "sent"
    }
  ],
  "total": 42
}
```

**Examples:**
```bash
# Get last 50 alerts
curl http://localhost:8080/api/alerts?limit=50

# Get critical alerts only
curl http://localhost:8080/api/alerts?severity=CriticalBurn

# Combine filters
curl http://localhost:8080/api/alerts?limit=100&severity=FastBurn
```

### POST /api/alerts/record

Record an alert event (for integration with alerting system).

**Request Body:**
```json
{
  "service_name": "api-gateway",
  "metric_name": "availability",
  "severity": "FastBurn",
  "message": "Burn rate 8.5x detected",
  "channels": ["slack", "pagerduty"]
}
```

**Response:**
```json
{
  "status": "recorded",
  "event": {
    "timestamp": "2026-06-27T10:30:00.000Z",
    "service_name": "api-gateway",
    "metric_name": "availability",
    "alert_type": "violation",
    "severity": "FastBurn",
    "message": "Burn rate 8.5x detected",
    "channels": ["slack", "pagerduty"],
    "status": "sent"
  }
}
```

### GET /api/forecast/{service_name}

Get budget exhaustion forecast.

**Path Parameters:**
- `service_name` (str): Service identifier

**Query Parameters:**
- `hours_ahead` (int, optional): Forecast window in hours. Default: 24

**Response:**
```json
{
  "service_name": "api-gateway",
  "timestamp": "2026-06-27T10:30:00.000Z",
  "forecasts": [
    {
      "metric_name": "availability",
      "will_exhaust": false,
      "time_to_exhaustion_hours": null,
      "current_burn_rate": 0.20,
      "current_budget_remaining_percent": 85.5,
      "hours_ahead": 24
    }
  ]
}
```

## Usage Patterns

### Pattern 1: Basic Dashboard

```python
from neuralbudget.dashboard import Dashboard

dashboard = Dashboard()
dashboard.run(host="127.0.0.1", port=8080)
```

### Pattern 2: Dashboard with Monitoring Loop

```python
import threading
from neuralbudget import NeuralBudgetClient
from neuralbudget.dashboard import Dashboard, SloSnapshot
from datetime import datetime

client = NeuralBudgetClient()
client.load_config("config.yaml")
dashboard = Dashboard(client=client)

def monitoring_loop():
    while True:
        result = client.evaluate(metric_data)
        dashboard.update_slo_snapshot(SloSnapshot(...))
        time.sleep(30)

threading.Thread(target=monitoring_loop, daemon=True).start()
dashboard.run()
```

### Pattern 3: CLI TUI for Terminal

```python
from neuralbudget.cli_tui import CliTui

tui = CliTui()
tui.run_demo()
```

### Pattern 4: Alert Integration

```python
import httpx
from neuralbudget import AlertDispatchManager

manager = AlertDispatchManager()

async def record_alert_on_dashboard(service_name, alert):
    async with httpx.AsyncClient() as client:
        await client.post(
            "http://localhost:8080/api/alerts/record",
            json={
                "service_name": service_name,
                "metric_name": alert["metric"],
                "severity": alert["severity"],
                "message": alert["message"],
                "channels": alert["channels"],
            }
        )
```

### Pattern 5: REST API Client

```python
import httpx

async def get_slo_status(service_name: str):
    async with httpx.AsyncClient() as client:
        response = await client.get(
            f"http://localhost:8080/api/slos/{service_name}"
        )
        return response.json()

async def get_alerts(limit: int = 100, severity: str = None):
    async with httpx.AsyncClient() as client:
        params = {"limit": limit}
        if severity:
            params["severity"] = severity
        response = await client.get(
            "http://localhost:8080/api/alerts",
            params=params
        )
        return response.json()
```

## Examples

### Example 1: Quick Start

```python
from neuralbudget.dashboard import Dashboard

# Start dashboard immediately
dashboard = Dashboard()
dashboard.run()  # http://localhost:8080
```

### Example 2: Multi-Service Monitoring

```python
from neuralbudget import NeuralBudgetClient
from neuralbudget.dashboard import Dashboard, SloSnapshot
from datetime import datetime

client = NeuralBudgetClient()
dashboard = Dashboard(client=client)

# Add multiple services
services = [
    ("api-gateway", "availability", 85.5, 0.2),
    ("payment", "latency", 42.1, 2.5),
    ("auth", "error_rate", 15.0, 8.9),
]

for service, metric, budget, burn_1h in services:
    dashboard.update_slo_snapshot(
        SloSnapshot(
            service_name=service,
            metric_name=metric,
            timestamp=datetime.utcnow().isoformat(),
            error_budget_remaining_percent=budget,
            burn_rate_5m=burn_1h * 0.75,
            burn_rate_30m=burn_1h * 0.9,
            burn_rate_1h=burn_1h,
            burn_rate_6h=burn_1h * 0.85,
            total_errors=100,
            total_requests=10000,
            error_rate_percent=1.0,
            severity="Ok",
            will_exhaust_budget=budget < 50,
            time_to_exhaustion_hours=budget / burn_1h if burn_1h > 0 else None,
            last_alert_at=None,
            last_alert_severity=None,
        )
    )

dashboard.run()
```

### Example 3: Docker Deployment

```dockerfile
FROM python:3.9-slim
WORKDIR /app
RUN pip install neuralbudget fastapi uvicorn
COPY config.yaml /app/
RUN echo "from neuralbudget.dashboard import Dashboard; Dashboard().run(host='0.0.0.0')" > start.py
EXPOSE 8080
CMD ["python", "start.py"]
```

### Example 4: CLI TUI with Custom Formatting

```python
from neuralbudget.cli_tui import CliTui
from rich.console import Console

console = Console()
console.print("[bold cyan]NeuralBudget CLI TUI[/bold cyan]")
console.print("Loading...\n")

tui = CliTui()
tui.run_demo()
```

## Performance Notes

- Dashboard: ~2-5MB memory overhead
- CLI TUI: ~1-2MB memory overhead
- SloSnapshot: ~500 bytes per service
- AlertEvent: ~200 bytes per alert
- Alert history limit: 1000 events (configurable)

## Error Handling

All endpoints return appropriate HTTP status codes:

- 200 OK: Request successful
- 400 Bad Request: Invalid parameters
- 404 Not Found: Resource not found
- 500 Internal Server Error: Server error

Example error response:
```json
{
  "detail": "Service 'unknown-service' not found"
}
```
