# Documentation Index

This index groups documentation by audience and workflow.

## Start Here

- README: Project overview, installation, examples, and high-level API surface.
- docs/guides/user-guide.md: End-to-end installation, usage, examples, applications, and troubleshooting.

## Feature Plans

- docs/plans/mlops-model-drift-serving-plan.md: Detailed implementation plan for MlSlo hybrid scoring.

## API References

- docs/reference/convenience-layer.md: Detailed reference for the Python convenience layer, typed dataclass returns, and profile presets.
- docs/reference/composite-slo-dag.md: Reference for Composite SLO DAG schemas, dependency propagation, global score semantics, and errors.

## Testing and Quality Gates

- CI workflow: .github/workflows/ci.yml
- CD workflow: .github/workflows/cd.yml
- PyPI release process: .github/workflows/cd.yml
- Python convenience tests: tests/python_convenience_tests.py
- Python client facade tests: tests/python_client_tests.py

## Suggested Reading Order

1. README
2. docs/guides/user-guide.md
3. docs/reference/convenience-layer.md
4. docs/reference/composite-slo-dag.md
5. docs/plans/mlops-model-drift-serving-plan.md
