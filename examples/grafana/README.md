# NeuralBudget Grafana Dashboards

Pre-built Grafana templates for each NeuralBudget SLO mode.

## Included Dashboards

- `dashboards/http-slo-dashboard.json`
- `dashboards/stateful-slo-dashboard.json`
- `dashboards/ml-slo-dashboard.json`
- `dashboards/genai-slo-dashboard.json`
- `dashboards/composite-slo-dashboard.json`

## Metrics Assumptions

These templates target the metric names exported by `PrometheusExporter` in `src/exporter.rs`:

- HTTP: `neuralbudget_http_*`
- Stateful: `neuralbudget_stateful_*`
- ML: `neuralbudget_ml_*`
- GenAI: `neuralbudget_genai_*`
- Composite: `neuralbudget_composite_*`

If your exporter namespace is not `neuralbudget` (for example `PrometheusExporter::with_namespace("myteam")`), update each dashboard query prefix accordingly (for example `myteam_http_pass`).

## Import Steps

1. Open Grafana.
2. Go to `Dashboards` -> `New` -> `Import`.
3. Upload one of the JSON files from `examples/grafana/dashboards/`.
4. Select your Prometheus datasource.
5. Save the dashboard.

## Template Variables

All dashboards include:

- `datasource`: Prometheus datasource selector.
- `service`: service label selector where applicable.
- `graph`: composite graph selector in the composite dashboard.

## Recommended Recording Rules

For large deployments, add recording rules from `docs/guides/prometheus-scraping-examples.md` to reduce dashboard query cost.
