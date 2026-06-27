# Dashboard & CLI TUI Implementation Guide

## Overview

NeuralBudget now includes a lightweight native dashboard and CLI TUI for teams not running Grafana, providing real-time SLO monitoring without external dependencies.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│         NeuralBudgetClient                          │
│  (Evaluates SLOs, calculates metrics)               │
└────────────────────┬────────────────────────────────┘
                     │
          ┌──────────┴──────────┐
          │                     │
    ┌─────▼────────┐      ┌────▼──────────┐
    │ Dashboard    │      │ CLI TUI       │
    │ (FastAPI)    │      │ (Textual)     │
    └─────┬────────┘      └────┬──────────┘
          │                     │
    ┌─────▼────────┐      ┌────▼──────────┐
    │ HTTP Server  │      │ Terminal UI   │
    │ (localhost)  │      │ (Rich tables) │
    └──────────────┘      └───────────────┘
          │                     │
    [Browser]             [Terminal/SSH]
```

## Web Dashboard

### Features

- **Real-time SLO Status**: Current budget, burn rates, error rates
- **Multi-Window Burn Rate Visualization**: 5m, 30m, 1h, 6h windows
- **Alert History**: Recent alerts with timestamps and severity
- **Budget Forecasts**: 24-hour exhaustion predictions
- **Health Monitoring**: Overall system status and metrics
- **REST API**: Programmatic access to all data

### Quick Start

```python
from neuralbudget.dashboard import Dashboard

# Create and run dashboard
dashboard = Dashboard()
dashboard.run(host="127.0.0.1", port=8080)

# Access at: http://localhost:8080
```

### Architecture

**FastAPI Server**
- Lightweight HTTP server (no external dependencies beyond FastAPI)
- Local-only binding by default (127.0.0.1)
- RESTful API endpoints for all data
- Auto-generated HTML/CSS dashboard
- 30-second auto-refresh

### Deployment Modes

#### Mode 1: Local-Only (Development)
```python
dashboard = Dashboard(host="127.0.0.1", port=8080)
dashboard.run()
```
- Only accessible from local machine
- No firewall holes needed
- Perfect for development

#### Mode 2: Network-Accessible (Production)
```python
dashboard = Dashboard(host="0.0.0.0", port=8080)
dashboard.run()
```
- Accessible from other machines
- Requires firewall configuration
- Consider reverse proxy and authentication

#### Mode 3: Docker
```bash
docker-compose up -d
```
- Containerized deployment
- Easy scaling and management
- Clean separation of concerns

### API Endpoints

```
GET  /api/health           → Health check
GET  /api/status           → Dashboard status
GET  /api/slos             → All SLO snapshots
GET  /api/slos/{service}   → Service SLOs
GET  /api/alerts?limit=100 → Alert history
GET  /api/forecast/{service} → Budget forecast
POST /api/alerts/record    → Record alert event
POST /api/evaluate         → Trigger evaluation
GET  /                     → Dashboard HTML
```

### Data Models

#### SloSnapshot
```python
@dataclass
class SloSnapshot:
    service_name: str                           # e.g., "api-gateway"
    metric_name: str                            # e.g., "availability"
    timestamp: str                              # ISO timestamp
    error_budget_remaining_percent: float       # 0-100%
    burn_rate_5m: float                         # 5-minute burn rate
    burn_rate_30m: float                        # 30-minute burn rate
    burn_rate_1h: float                         # 1-hour burn rate
    burn_rate_6h: float                         # 6-hour burn rate
    total_errors: int                           # Cumulative errors
    total_requests: int                         # Cumulative requests
    error_rate_percent: float                   # Current error rate %
    severity: str                               # Ok|SlowBurn|MediumBurn|FastBurn|CriticalBurn
    will_exhaust_budget: bool                   # Forecast: will exhaust in 24h?
    time_to_exhaustion_hours: Optional[float]   # Hours until exhaustion
    last_alert_at: Optional[str]                # Last alert timestamp
    last_alert_severity: Optional[str]          # Last alert severity
```

#### AlertEvent
```python
@dataclass
class AlertEvent:
    timestamp: str                               # ISO timestamp
    service_name: str                            # Service triggering alert
    metric_name: str                             # Metric that violated SLO
    alert_type: str                              # violation|escalation|recovery
    severity: str                                # Ok|SlowBurn|MediumBurn|FastBurn|CriticalBurn
    message: str                                 # Human-readable message
    channels: List[str]                          # Notification channels
    status: str                                  # sent|failed|deduped
```

### Integration with NeuralBudgetClient

```python
from neuralbudget import NeuralBudgetClient
from neuralbudget.dashboard import Dashboard, SloSnapshot
import threading

# Load client config
client = NeuralBudgetClient()
client.load_config("config.yaml")

# Create dashboard
dashboard = Dashboard(client=client)

# Monitoring loop
def monitor():
    while True:
        # Evaluate SLOs
        result = client.evaluate(metric_data)
        
        # Update dashboard
        dashboard.update_slo_snapshot(
            SloSnapshot(
                service_name="api-gateway",
                metric_name="availability",
                # ... populate from result
            )
        )
        
        time.sleep(30)

# Run monitoring in background
thread = threading.Thread(target=monitor, daemon=True)
thread.start()

# Start dashboard
dashboard.run()
```

### Performance Characteristics

- **Memory**: ~500 bytes per tracked SLO snapshot
- **CPU**: Minimal (mostly I/O)
- **Network**: Lightweight JSON responses
- **Scalability**: Handles 100+ services easily

## CLI TUI

### Features

- **Terminal-Based Dashboard**: No browser needed
- **Real-time Monitoring**: All metrics in one view
- **Multi-Window Burn Rates**: Visual representation
- **Alert History**: Scrollable event log
- **Budget Forecasts**: Exhaustion predictions
- **Keyboard Navigation**: Shell-like controls
- **SSH Friendly**: Works over remote connections

### Quick Start

```python
from neuralbudget.cli_tui import CliTui

tui = CliTui()
tui.run()  # Interactive demo
```

Or via command line:
```bash
python -m neuralbudget.cli_tui
```

### Keyboard Controls

| Key | Action |
|-----|--------|
| `r` | Refresh dashboard |
| `f` | Filter by service |
| `s` | Sort by column |
| `p` | Pause/resume auto-refresh |
| `h` | Help |
| `q` | Quit |

### Display Modes

#### Mode 1: Demo Mode (Default)
```python
tui = CliTui()
tui.run_demo()  # Rich table-based display
```
- Interactive demo with sample data
- No Textual dependency required
- Rich formatting and colors

#### Mode 2: Full Textual App
```python
tui = CliTui()
tui.run_textual_app()  # Full interactive TUI
```
- Modern interactive dashboard
- Requires Textual package
- Better responsiveness

### Output Format

#### SLO Status Table
```
┌─────────────────────────────────────┐
│ Service: api-gateway                │
│ ● Status: OK                        │
│   Budget: 85.5%                     │
│   Burn (1h): 0.20x                  │
│   Errors: 150 / 15,000              │
└─────────────────────────────────────┘
```

#### Multi-Window Burn Rate
```
Window   Burn Rate   Chart
─────────────────────────────────
5m       0.15x       ████░░░░░░
30m      0.18x       █████░░░░░
1h       0.20x       ██████░░░░
6h       0.19x       █████░░░░░
```

#### Alert History
```
Time                Service       Severity   Message
────────────────────────────────────────────────────
2026-06-27 10:25:30 auth-service  CRITICAL   FastBurn alert: 8.9x burn rate
2026-06-27 10:20:15 payment-svc   MEDIUM     MediumBurn → Escalated to PagerDuty
2026-06-27 10:15:00 api-gateway   OK         Budget recovered
```

### Integration with NeuralBudgetClient

```python
from neuralbudget import NeuralBudgetClient
from neuralbudget.cli_tui import CliTui

# Load config
client = NeuralBudgetClient()
client.load_config("config.yaml")

# Create TUI with client
tui = CliTui(client=client)
tui.run_demo()
```

## Comparison Matrix

| Feature | Dashboard | CLI TUI |
|---------|-----------|---------|
| Access | Browser | Terminal/SSH |
| Browsers needed | Yes | No |
| Mobile support | Yes | No |
| Firewall holes | Needed | None (local) |
| Real-time updates | 30s refresh | On demand |
| Team usage | Excellent | Limited |
| SSH access | With proxy | Direct |
| Dependencies | FastAPI, Uvicorn | Rich + Optional Textual |
| Best for | Visual overview | DevOps engineers |
| Learning curve | Low | Minimal |

## Use Cases

### Use Case 1: DevOps Team Dashboard
- Deploy dashboard on shared server
- All team members access via browser
- Visual SLO overview for standups
- Alert history for incident analysis

### Use Case 2: Ops Engineer Remote Monitoring
- SSH into ops box
- Run CLI TUI
- Monitor in real-time
- No VPN needed for browser

### Use Case 3: Development Debugging
- Local dashboard on developer machine
- Monitor SLOs during feature development
- Lightweight alternative to Grafana
- No infrastructure setup needed

### Use Case 4: Production Single-Box Deployment
- Dashboard running on production box
- Accessible only from localhost
- No external access needed
- Minimal resource footprint

## Configuration

### Environment Variables

```bash
# Dashboard
NEURALBUDGET_DASHBOARD_HOST=127.0.0.1
NEURALBUDGET_DASHBOARD_PORT=8080
NEURALBUDGET_DASHBOARD_TITLE="My Dashboard"
NEURALBUDGET_ENABLE_CORS=true

# CLI TUI
NEURALBUDGET_TUI_AUTO_REFRESH=true
NEURALBUDGET_TUI_REFRESH_INTERVAL=5
```

### Config File Integration

```yaml
# config.yaml
dashboard:
  enabled: true
  host: 127.0.0.1
  port: 8080
  title: "Production SLO Dashboard"
  cors_enabled: true
  max_alert_history: 1000

cli_tui:
  enabled: true
  auto_refresh: true
  refresh_interval: 5
  theme: "dark"

monitoring:
  evaluation_interval: 30
  alert_recording: true
```

## Performance Tuning

### Dashboard Optimization
- Limit alert history: `max_alert_history = 1000`
- Adjust refresh: Change HTML `setInterval` value
- Batch updates: Use threading for monitoring loop
- Cache snapshots: Only update changed services

### CLI TUI Optimization
- Reduce refresh frequency: Adjust `auto_refresh` interval
- Limit displayed alerts: Filter by severity
- Use demo mode: Faster rendering than full TUI
- SSH optimization: Use local shell for rendering

## Troubleshooting

### Dashboard Won't Start
```
Error: Address already in use

Solution: Change port
dashboard.run(port=8081)
```

### Connection Refused
```
Error: Connection refused on localhost:8080

Solution 1: Check if dashboard is running
Solution 2: Check firewall rules
Solution 3: Use 127.0.0.1 instead of localhost

dashboard.run(host="127.0.0.1", port=8080)
```

### CLI TUI Not Rendering
```
Error: Terminal too small or Textual missing

Solution 1: Resize terminal to at least 80x24
Solution 2: Install Textual: pip install textual
Solution 3: Use demo mode: tui.run_demo()
```

### No Data Showing
```
Error: Dashboard shows "Loading..." indefinitely

Solution 1: Check NeuralBudgetClient config
Solution 2: Add SLO snapshots: dashboard.update_slo_snapshot()
Solution 3: Check API endpoint: curl http://localhost:8080/api/health
```

## Best Practices

1. **Local-Only by Default**: Use `127.0.0.1` for security
2. **Firewall**: Only expose if necessary, use reverse proxy
3. **Monitoring Loop**: Run in separate thread to avoid blocking
4. **Alert Recording**: Integrate with your alert system
5. **Scalability**: Use separate thread/process for dashboard
6. **SSL/TLS**: Use reverse proxy for production HTTPS
7. **Authentication**: Add auth layer for team deployments

## Security Considerations

- Dashboard runs on localhost by default
- No authentication built-in (add reverse proxy)
- HTTP only (add SSL via reverse proxy)
- CORS disabled by default
- Alert history stored in memory only
- No persistent storage on disk

## Roadmap

Future enhancements:
- [ ] Grafana integration layer
- [ ] Prometheus scrape endpoint
- [ ] Custom dashboard layouts
- [ ] Alert webhook receiver
- [ ] Database persistence option
- [ ] LDAP/OAuth integration
- [ ] Mobile app
- [ ] Slack bot integration

## Examples

See `examples/python/dashboard_cli_examples.py` for:
1. Basic dashboard usage
2. Custom configuration
3. Monitoring loop integration
4. CLI TUI basics
5. Alert integration
6. Docker deployment
7. Multi-service setup
8. Lightweight mode comparison

## API Reference

See `docs/reference/dashboard_cli.md` for complete API documentation.
