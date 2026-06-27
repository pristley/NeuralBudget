# NeuralBudget Dashboard & CLI TUI - Quick Start Guide

## Overview

NeuralBudget now includes **two lightweight monitoring tools** for teams without Grafana:

1. **Web Dashboard**: Visual dashboard in your browser
2. **CLI TUI**: Terminal-based monitoring (no browser needed)

Both are lightweight, require no external infrastructure, and can run locally or on a shared server.

## Installation

### Basic Installation
```bash
pip install neuralbudget
```

### With Dashboard Support
```bash
pip install "neuralbudget[dashboard]"
# or
pip install neuralbudget fastapi uvicorn
```

### With CLI TUI Support
```bash
pip install "neuralbudget[tui]"
# or
pip install neuralbudget rich textual
```

### Full Installation
```bash
pip install "neuralbudget[dashboard,tui]"
```

## Quick Start

### Option 1: Web Dashboard (5 minutes)

Start the dashboard server:
```python
from neuralbudget.dashboard import Dashboard

dashboard = Dashboard()
dashboard.run()  # Starts on http://localhost:8080
```

Or use the CLI:
```bash
python examples/scripts/neuralbudget-cli.py dashboard
# Opens http://localhost:8080
```

Then open your browser and navigate to `http://localhost:8080`

### Option 2: CLI TUI (5 minutes)

Run the terminal dashboard:
```python
from neuralbudget.cli_tui import CliTui

tui = CliTui()
tui.run()  # Interactive terminal dashboard
```

Or use the CLI:
```bash
python examples/scripts/neuralbudget-cli.py tui --demo
```

## Features

### Dashboard Features
✅ Real-time SLO status and metrics  
✅ Multi-window burn rate visualization (5m, 30m, 1h, 6h)  
✅ Alert history with escalation tracking  
✅ Budget exhaustion forecasts (24-hour ahead)  
✅ Beautiful web UI with auto-refresh  
✅ REST API for programmatic access  
✅ Local-only by default (127.0.0.1)  

### CLI TUI Features
✅ No browser required  
✅ Works over SSH  
✅ Keyboard navigation  
✅ Multi-window burn rate charts  
✅ Alert history  
✅ Budget forecasts  
✅ Minimal dependencies  

## Common Use Cases

### Use Case 1: Development & Debugging
```python
from neuralbudget import NeuralBudgetClient
from neuralbudget.dashboard import Dashboard, SloSnapshot
from datetime import datetime

client = NeuralBudgetClient()
client.load_config("config.yaml")

dashboard = Dashboard(client=client)

# Add sample data
dashboard.update_slo_snapshot(
    SloSnapshot(
        service_name="my-service",
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

dashboard.run(host="127.0.0.1", port=8080)
```

### Use Case 2: Team Standups
```bash
# Share dashboard on team server
python examples/scripts/neuralbudget-cli.py dashboard --host 0.0.0.0 --port 8080
# Team accesses at http://your-server:8080
```

### Use Case 3: Ops Monitoring via SSH
```bash
# SSH to ops box
ssh ops-box

# Run CLI TUI
python examples/scripts/neuralbudget-cli.py tui --demo

# View SLO metrics, alert history, forecasts in terminal
```

### Use Case 4: Docker Deployment
```dockerfile
FROM python:3.9-slim
WORKDIR /app
RUN pip install neuralbudget fastapi uvicorn
COPY config.yaml /app/
CMD ["python", "examples/scripts/neuralbudget-cli.py", "dashboard", "--host", "0.0.0.0"]
```

## Configuration

### Custom Port
```python
dashboard.run(port=3000)  # Use port 3000 instead of 8080
```

### Public Endpoint (Use with Caution!)
```python
dashboard.run(host="0.0.0.0", port=8080)
# Accessible from any IP on your network
```

### With Monitoring Loop
```python
import threading
import time
from neuralbudget import NeuralBudgetClient
from neuralbudget.dashboard import Dashboard, SloSnapshot

client = NeuralBudgetClient()
client.load_config("config.yaml")
dashboard = Dashboard(client=client)

def monitor():
    while True:
        # Evaluate your SLOs
        result = client.evaluate(metric_data)
        
        # Update dashboard
        dashboard.update_slo_snapshot(SloSnapshot(...))
        
        time.sleep(30)  # Refresh every 30 seconds

# Run monitoring in background
monitor_thread = threading.Thread(target=monitor, daemon=True)
monitor_thread.start()

# Start dashboard
dashboard.run()
```

## Dashboard vs CLI TUI

| Feature | Dashboard | CLI TUI |
|---------|-----------|---------|
| Browser needed | Yes | No |
| Mobile support | Yes | No |
| Team access | Easy | Limited |
| SSH access | With proxy | Direct |
| Best for | Visual overview | DevOps/Ops |
| Learning curve | Low | Very low |

**Choose Dashboard if:**
- Team needs visual SLO overview
- Multiple users from different locations
- Non-technical users accessing
- Mobile access needed

**Choose CLI TUI if:**
- You prefer terminal
- Running over SSH
- No browser available
- Minimal dependencies

## API Endpoints

The dashboard exposes REST API endpoints:

```bash
# Get all SLOs
curl http://localhost:8080/api/slos

# Get specific service
curl http://localhost:8080/api/slos/api-gateway

# Get alert history
curl http://localhost:8080/api/alerts?limit=50

# Get forecast
curl http://localhost:8080/api/forecast/api-gateway

# Record an alert
curl -X POST http://localhost:8080/api/alerts/record \
  -H "Content-Type: application/json" \
  -d '{
    "service_name": "api-gateway",
    "metric_name": "availability",
    "severity": "FastBurn",
    "message": "Burn rate alert",
    "channels": ["slack"]
  }'
```

## Troubleshooting

### Dashboard won't start
```
ERROR: Address already in use: ('127.0.0.1', 8080)
```
**Solution:** Use a different port
```python
dashboard.run(port=8081)
```

### Connection refused when accessing dashboard
```
curl: (7) Failed to connect to localhost port 8080
```
**Solutions:**
1. Check dashboard is running
2. Try `http://127.0.0.1:8080` instead of `localhost`
3. Check firewall

### CLI TUI doesn't render
```
Terminal too small or Textual not installed
```
**Solutions:**
1. Resize terminal to at least 80x24 characters
2. Install Textual: `pip install textual`
3. Use demo mode: `python examples/scripts/neuralbudget-cli.py tui --demo`

### No data showing in dashboard
```
Dashboard loads but no SLO data displayed
```
**Solutions:**
1. Add SLO snapshots: `dashboard.update_slo_snapshot(...)`
2. Check NeuralBudgetClient configuration
3. Run `curl http://localhost:8080/api/health` to verify API works

## Security Considerations

- **Local-only by default**: Only `127.0.0.1` can connect
- **No authentication built-in**: Add reverse proxy with auth for production
- **HTTP only**: Use reverse proxy for HTTPS/SSL
- **Memory-only storage**: Alert history not persisted to disk
- **CORS disabled**: Configure explicitly if needed

For production:
1. Use reverse proxy (Nginx, Caddy, etc.)
2. Add authentication (OAuth, LDAP, etc.)
3. Enable HTTPS/SSL
4. Limit network access via firewall

## Performance

- **Memory**: ~2-5 MB overhead for dashboard
- **CPU**: Minimal, mostly I/O
- **Supports**: 100+ services easily
- **SLO snapshot**: ~500 bytes each
- **Alert event**: ~200 bytes each

## Examples

See `examples/python/dashboard_cli_examples.py` for:
1. Basic dashboard usage
2. Custom configuration
3. Monitoring loop integration
4. CLI TUI
5. Alert integration
6. Docker deployment
7. Multi-service setup

Run examples:
```bash
python examples/python/dashboard_cli_examples.py 1
python examples/python/dashboard_cli_examples.py 2
# ... etc
```

## Complete Documentation

- **Implementation Guide**: [docs/guides/dashboard_cli.md](docs/guides/dashboard_cli.md)
- **API Reference**: [docs/reference/dashboard_cli.md](docs/reference/dashboard_cli.md)
- **Examples**: [examples/python/dashboard_cli_examples.py](examples/python/dashboard_cli_examples.py)

## Next Steps

1. **Try the dashboard**: `python -m neuralbudget.dashboard`
2. **Try the CLI TUI**: `python -m neuralbudget.cli_tui`
3. **Read the guide**: See [docs/guides/dashboard_cli.md](docs/guides/dashboard_cli.md)
4. **Check examples**: See [examples/python/dashboard_cli_examples.py](examples/python/dashboard_cli_examples.py)

## Getting Help

- **Questions?** Check [docs/guides/dashboard_cli.md](docs/guides/dashboard_cli.md)
- **API help?** See [docs/reference/dashboard_cli.md](docs/reference/dashboard_cli.md)
- **Issues?** See troubleshooting section above

---

**Happy monitoring! 🎯**
