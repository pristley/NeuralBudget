# Documentation Index

This index groups documentation by audience and workflow.

## Start Here

- README: Project overview, installation, examples, and high-level API surface.
- docs/guides/user-guide.md: End-to-end installation, usage, examples, applications, and troubleshooting.
- docs/guides/production-deployment.md: Production rollout patterns, Kubernetes manifests, and Prometheus scraping setup.
- CHANGELOG.md: Versioned release history and categorized change entries.

## Feature Plans

- docs/plans/mlops-model-drift-serving-plan.md: Detailed implementation plan for MlSlo hybrid scoring.

## API References

- docs/reference/convenience-layer.md: Detailed reference for the Python convenience layer, typed dataclass returns, and profile presets.
- docs/reference/composite-slo-dag.md: Reference for Composite SLO DAG schemas, dependency propagation, global score semantics, and errors.

## Testing and Quality Gates

- CI workflow: .github/workflows/ci.yml
- CD workflow: .github/workflows/cd.yml (validation + release + distribution)
- PyPI release process: .github/workflows/cd.yml (trusted publishing)
- Python convenience tests: tests/python_convenience_tests.py
- Python client facade tests: tests/python_client_tests.py

## Deployment Examples

- examples/kubernetes/configmap.yaml
- examples/kubernetes/deployment.yaml
- examples/kubernetes/service.yaml
- examples/kubernetes/servicemonitor.yaml
- examples/kubernetes/prometheus-additional-scrape-config.yaml

## Suggested Reading Order

1. README
2. docs/guides/user-guide.md
3. docs/guides/production-deployment.md
4. CHANGELOG.md
5. docs/reference/convenience-layer.md
6. docs/reference/composite-slo-dag.md
7. docs/plans/mlops-model-drift-serving-plan.md
