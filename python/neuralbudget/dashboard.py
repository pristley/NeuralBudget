"""Lightweight FastAPI dashboard for NeuralBudget SLO monitoring.

Provides embedded HTTP server with real-time SLO metrics, burn rates,
alert history, and budget forecasts. Requires no external dependencies
beyond FastAPI and can run in local-only mode.

Usage:
    from neuralbudget.dashboard import Dashboard
    
    dashboard = Dashboard(host="127.0.0.1", port=8080)
    dashboard.run()  # Blocking
    
    # Or run async
    asyncio.run(dashboard.run_async())
"""

import json
import logging
from datetime import datetime, timedelta
from typing import Any, Dict, List, Optional
from dataclasses import dataclass, asdict

try:
    from fastapi import FastAPI, HTTPException
    from fastapi.responses import HTMLResponse, JSONResponse
    from fastapi.staticfiles import StaticFiles
    import uvicorn
except ImportError:
    raise ImportError(
        "FastAPI and uvicorn are required for dashboard. "
        "Install with: pip install fastapi uvicorn"
    )

from .client import NeuralBudgetClient, EvaluationMode
from .convenience import (
    HttpSloProfile,
    MlSloProfile,
    StatefulSloProfile,
    GenAiSloProfile,
)

logger = logging.getLogger(__name__)


@dataclass
class SloSnapshot:
    """Current snapshot of an SLO's status and metrics."""

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
    severity: str  # Ok, SlowBurn, MediumBurn, FastBurn, CriticalBurn
    will_exhaust_budget: bool
    time_to_exhaustion_hours: Optional[float]
    last_alert_at: Optional[str]
    last_alert_severity: Optional[str]


@dataclass
class AlertEvent:
    """Historical alert event for audit trail."""

    timestamp: str
    service_name: str
    metric_name: str
    alert_type: str  # violation, escalation, recovery
    severity: str
    message: str
    channels: List[str]
    status: str  # sent, failed, deduped


class Dashboard:
    """Lightweight embedded dashboard for NeuralBudget monitoring."""

    def __init__(
        self,
        client: Optional[NeuralBudgetClient] = None,
        host: str = "127.0.0.1",
        port: int = 8080,
        title: str = "NeuralBudget Dashboard",
        enable_cors: bool = True,
    ):
        """Initialize dashboard.

        Args:
            client: NeuralBudgetClient instance. If None, will create one.
            host: Server host (default: 127.0.0.1 for local-only)
            port: Server port (default: 8080)
            title: Dashboard title
            enable_cors: Enable CORS for development
        """
        self.client = client or NeuralBudgetClient()
        self.host = host
        self.port = port
        self.title = title

        # Create FastAPI app
        self.app = FastAPI(
            title=title,
            description="Lightweight SLO monitoring dashboard",
            version="1.0",
        )

        # Store recent data
        self._slo_snapshots: Dict[str, SloSnapshot] = {}
        self._alert_events: List[AlertEvent] = []
        self._max_alert_history = 1000

        # Setup routes
        self._setup_routes()

        if enable_cors:
            self._setup_cors()

    def _setup_cors(self) -> None:
        """Setup CORS for development."""
        try:
            from fastapi.middleware.cors import CORSMiddleware

            self.app.add_middleware(
                CORSMiddleware,
                allow_origins=["*"],
                allow_credentials=True,
                allow_methods=["*"],
                allow_headers=["*"],
            )
        except ImportError:
            logger.warning("CORS middleware not available")

    def _setup_routes(self) -> None:
        """Setup FastAPI routes."""

        @self.app.get("/", response_class=HTMLResponse)
        async def root():
            """Serve dashboard HTML."""
            return self._get_dashboard_html()

        @self.app.get("/api/health")
        async def health():
            """Health check endpoint."""
            return {
                "status": "ok",
                "timestamp": datetime.utcnow().isoformat(),
                "version": "1.0",
            }

        @self.app.get("/api/status")
        async def status():
            """Get overall dashboard status."""
            slos_evaluated = len(self._slo_snapshots)
            alerts_total = len(self._alert_events)
            critical_count = sum(
                1 for s in self._slo_snapshots.values()
                if s.severity == "CriticalBurn"
            )
            medium_count = sum(
                1 for s in self._slo_snapshots.values()
                if s.severity in ["FastBurn", "MediumBurn"]
            )

            return {
                "timestamp": datetime.utcnow().isoformat(),
                "slos_evaluated": slos_evaluated,
                "alerts_total": alerts_total,
                "critical_alerts": critical_count,
                "medium_alerts": medium_count,
                "uptime_minutes": 0,  # Would track actual uptime
            }

        @self.app.get("/api/slos")
        async def get_slos():
            """Get all SLO snapshots."""
            return {
                "timestamp": datetime.utcnow().isoformat(),
                "slos": [asdict(s) for s in self._slo_snapshots.values()],
            }

        @self.app.get("/api/slos/{service_name}")
        async def get_slo_by_service(service_name: str):
            """Get SLO data for specific service."""
            matching_slos = [
                asdict(s)
                for s in self._slo_snapshots.values()
                if s.service_name == service_name
            ]
            if not matching_slos:
                raise HTTPException(status_code=404, detail="Service not found")
            return {
                "service_name": service_name,
                "slos": matching_slos,
            }

        @self.app.get("/api/alerts")
        async def get_alerts(
            limit: int = 100, severity: Optional[str] = None
        ):
            """Get alert history."""
            events = self._alert_events[-limit:]
            if severity:
                events = [e for e in events if e.severity == severity]
            return {
                "timestamp": datetime.utcnow().isoformat(),
                "alerts": [asdict(e) for e in reversed(events)],
                "total": len(self._alert_events),
            }

        @self.app.post("/api/evaluate")
        async def trigger_evaluation(service_name: str):
            """Trigger evaluation for a service."""
            try:
                # Load latest config
                config = self.client.load_config()
                # Trigger evaluation
                result = await self._evaluate_service(service_name)
                return {"status": "evaluated", "result": result}
            except Exception as e:
                raise HTTPException(status_code=500, detail=str(e))

        @self.app.post("/api/alerts/record")
        async def record_alert(
            service_name: str,
            metric_name: str,
            severity: str,
            message: str,
            channels: List[str],
        ):
            """Record an alert event (for integration with alerting system)."""
            event = AlertEvent(
                timestamp=datetime.utcnow().isoformat(),
                service_name=service_name,
                metric_name=metric_name,
                alert_type="violation",
                severity=severity,
                message=message,
                channels=channels,
                status="sent",
            )
            self._alert_events.append(event)

            # Keep bounded history
            if len(self._alert_events) > self._max_alert_history:
                self._alert_events.pop(0)

            return {"status": "recorded", "event": asdict(event)}

        @self.app.get("/api/forecast/{service_name}")
        async def get_forecast(service_name: str, hours_ahead: int = 24):
            """Get budget exhaustion forecast."""
            slos = [
                s
                for s in self._slo_snapshots.values()
                if s.service_name == service_name
            ]
            if not slos:
                raise HTTPException(status_code=404, detail="Service not found")

            forecasts = []
            for slo in slos:
                forecast = {
                    "metric_name": slo.metric_name,
                    "will_exhaust": slo.will_exhaust_budget,
                    "time_to_exhaustion_hours": slo.time_to_exhaustion_hours,
                    "current_burn_rate": slo.burn_rate_1h,
                    "current_budget_remaining_percent": slo.error_budget_remaining_percent,
                    "hours_ahead": hours_ahead,
                }
                forecasts.append(forecast)

            return {
                "service_name": service_name,
                "timestamp": datetime.utcnow().isoformat(),
                "forecasts": forecasts,
            }

    def _get_dashboard_html(self) -> str:
        """Generate dashboard HTML."""
        return """
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>NeuralBudget Dashboard</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
            background: linear-gradient(135deg, #1e3a8a 0%, #1e40af 100%);
            color: #333;
            padding: 20px;
            min-height: 100vh;
        }
        .container {
            max-width: 1400px;
            margin: 0 auto;
        }
        .header {
            color: white;
            margin-bottom: 30px;
            padding-bottom: 20px;
            border-bottom: 2px solid rgba(255,255,255,0.1);
        }
        .header h1 {
            font-size: 28px;
            margin-bottom: 5px;
        }
        .header p {
            opacity: 0.9;
            font-size: 14px;
        }
        .grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }
        .card {
            background: white;
            border-radius: 8px;
            padding: 20px;
            box-shadow: 0 4px 6px rgba(0,0,0,0.1);
            transition: transform 0.2s, box-shadow 0.2s;
        }
        .card:hover {
            transform: translateY(-2px);
            box-shadow: 0 8px 12px rgba(0,0,0,0.15);
        }
        .stat-label {
            font-size: 12px;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            color: #666;
            margin-bottom: 8px;
        }
        .stat-value {
            font-size: 32px;
            font-weight: 600;
            color: #1e3a8a;
            margin-bottom: 4px;
        }
        .stat-unit {
            font-size: 14px;
            color: #999;
        }
        .badge {
            display: inline-block;
            padding: 4px 12px;
            border-radius: 4px;
            font-size: 12px;
            font-weight: 500;
            margin-top: 10px;
        }
        .badge.ok { background: #dcfce7; color: #166534; }
        .badge.slowburn { background: #fef3c7; color: #92400e; }
        .badge.mediumburn { background: #fed7aa; color: #9a3412; }
        .badge.fastburn { background: #fecaca; color: #991b1b; }
        .badge.criticalburn { background: #fca5a5; color: #7f1d1d; }
        .section-title {
            color: white;
            font-size: 18px;
            margin-bottom: 15px;
            font-weight: 600;
        }
        .slo-row {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 15px;
            border-bottom: 1px solid #eee;
            font-size: 14px;
        }
        .slo-row:last-child { border-bottom: none; }
        .slo-name { font-weight: 500; color: #1e3a8a; }
        .slo-metrics {
            display: flex;
            gap: 20px;
            justify-content: flex-end;
            color: #666;
        }
        .progress-bar {
            width: 100%;
            height: 8px;
            background: #e5e7eb;
            border-radius: 4px;
            overflow: hidden;
            margin-top: 8px;
        }
        .progress-fill {
            height: 100%;
            background: linear-gradient(90deg, #22c55e 0%, #1e40af 100%);
            transition: width 0.3s ease;
        }
        .alert-item {
            padding: 12px;
            border-left: 4px solid #1e40af;
            background: #f0f9ff;
            border-radius: 4px;
            margin-bottom: 8px;
            font-size: 13px;
        }
        .alert-item.critical { border-left-color: #dc2626; background: #fef2f2; }
        .alert-item.warning { border-left-color: #f59e0b; background: #fffbeb; }
        .alert-time {
            color: #999;
            font-size: 12px;
            margin-top: 4px;
        }
        .loading {
            text-align: center;
            color: white;
            padding: 40px;
        }
        .spinner {
            border: 3px solid rgba(255,255,255,0.3);
            border-top: 3px solid white;
            border-radius: 50%;
            width: 40px;
            height: 40px;
            animation: spin 1s linear infinite;
            margin: 0 auto 20px;
        }
        @keyframes spin {
            0% { transform: rotate(0deg); }
            100% { transform: rotate(360deg); }
        }
        .refresh-btn {
            background: #1e40af;
            color: white;
            border: none;
            padding: 10px 20px;
            border-radius: 6px;
            cursor: pointer;
            font-size: 14px;
            margin-top: 20px;
        }
        .refresh-btn:hover {
            background: #1e3a8a;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🎯 NeuralBudget Dashboard</h1>
            <p id="timestamp">Initializing...</p>
        </div>

        <div id="loading" class="loading">
            <div class="spinner"></div>
            <p>Loading SLO data...</p>
        </div>

        <div id="content" style="display: none;">
            <div class="grid" id="status-cards"></div>

            <div class="section-title">📊 SLO Metrics</div>
            <div class="card" id="slo-table"></div>

            <div class="section-title">🔔 Recent Alerts</div>
            <div class="card" id="alerts-list"></div>

            <button class="refresh-btn" onclick="refreshDashboard()">Refresh Now</button>
        </div>
    </div>

    <script>
        async function loadData() {
            try {
                const [status, slos, alerts] = await Promise.all([
                    fetch('/api/status').then(r => r.json()),
                    fetch('/api/slos').then(r => r.json()),
                    fetch('/api/alerts?limit=10').then(r => r.json()),
                ]);

                renderDashboard(status, slos, alerts);
                document.getElementById('loading').style.display = 'none';
                document.getElementById('content').style.display = 'block';
            } catch (err) {
                console.error('Failed to load data:', err);
                document.getElementById('loading').innerHTML = `
                    <p style="color: white;">Failed to load data: ${err.message}</p>
                    <p style="color: rgba(255,255,255,0.7); font-size: 14px; margin-top: 10px;">
                        Make sure the dashboard API is running
                    </p>
                `;
            }
        }

        function renderDashboard(status, slos, alerts) {
            const timestamp = new Date(status.timestamp).toLocaleString();
            document.getElementById('timestamp').innerText = `Last updated: ${timestamp}`;

            // Status cards
            const cards = `
                <div class="card">
                    <div class="stat-label">Services Monitored</div>
                    <div class="stat-value">${status.slos_evaluated}</div>
                </div>
                <div class="card">
                    <div class="stat-label">Critical Alerts</div>
                    <div class="stat-value" style="color: #dc2626;">${status.critical_alerts}</div>
                </div>
                <div class="card">
                    <div class="stat-label">Medium Alerts</div>
                    <div class="stat-value" style="color: #f59e0b;">${status.medium_alerts}</div>
                </div>
                <div class="card">
                    <div class="stat-label">Total Alert Events</div>
                    <div class="stat-value">${status.alerts_total}</div>
                </div>
            `;
            document.getElementById('status-cards').innerHTML = cards;

            // SLO table
            const sloHtml = `
                ${slos.slos.map(slo => `
                    <div class="slo-row">
                        <div>
                            <div class="slo-name">${slo.service_name} / ${slo.metric_name}</div>
                            <div class="progress-bar">
                                <div class="progress-fill" style="width: ${slo.error_budget_remaining_percent}%"></div>
                            </div>
                        </div>
                        <div class="slo-metrics">
                            <div>${slo.burn_rate_1h.toFixed(2)}x (1h)</div>
                            <span class="badge ${slo.severity.toLowerCase()}">${slo.severity}</span>
                        </div>
                    </div>
                `).join('')}
            `;
            document.getElementById('slo-table').innerHTML = sloHtml;

            // Alerts
            const alertsHtml = `
                ${alerts.alerts.length > 0 ? alerts.alerts.map(alert => `
                    <div class="alert-item ${alert.severity.toLowerCase()}">
                        <strong>${alert.service_name}</strong> - ${alert.message}
                        <div class="alert-time">${new Date(alert.timestamp).toLocaleString()}</div>
                    </div>
                `).join('') : '<p style="color: #999;">No recent alerts</p>'}
            `;
            document.getElementById('alerts-list').innerHTML = alertsHtml;
        }

        function refreshDashboard() {
            document.getElementById('content').style.display = 'none';
            document.getElementById('loading').style.display = 'block';
            loadData();
        }

        // Auto-refresh every 30 seconds
        loadData();
        setInterval(loadData, 30000);
    </script>
</body>
</html>
        """

    def update_slo_snapshot(self, snapshot: SloSnapshot) -> None:
        """Update SLO snapshot data."""
        key = f"{snapshot.service_name}/{snapshot.metric_name}"
        self._slo_snapshots[key] = snapshot

    async def _evaluate_service(self, service_name: str) -> Dict[str, Any]:
        """Evaluate a service (placeholder for integration)."""
        return {"service": service_name, "status": "evaluated"}

    def run(
        self,
        host: Optional[str] = None,
        port: Optional[int] = None,
        reload: bool = False,
    ) -> None:
        """Run dashboard server (blocking).

        Args:
            host: Override server host
            port: Override server port
            reload: Auto-reload on file changes (development)
        """
        host = host or self.host
        port = port or self.port

        logger.info(f"Starting NeuralBudget Dashboard at http://{host}:{port}")
        uvicorn.run(
            self.app,
            host=host,
            port=port,
            log_level="info",
            reload=reload,
        )

    async def run_async(
        self, host: Optional[str] = None, port: Optional[int] = None
    ) -> None:
        """Run dashboard server (async).

        Args:
            host: Override server host
            port: Override server port
        """
        host = host or self.host
        port = port or self.port

        import uvicorn

        config = uvicorn.Config(
            self.app,
            host=host,
            port=port,
            log_level="info",
        )
        server = uvicorn.Server(config)
        await server.serve()


# Example usage
if __name__ == "__main__":
    # Create dashboard with default client
    dashboard = Dashboard()

    # Add some example data
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

    # Run on localhost only
    dashboard.run(host="127.0.0.1", port=8080)
