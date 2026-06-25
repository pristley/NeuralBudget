#!/usr/bin/env bash
set -euo pipefail

# Local simulator for the release packaging/publish flow.
# - Builds crate, wheel, and sdist
# - Collects artifacts into dist/release-local
# - Validates artifacts with twine check
# - Optionally uploads to TestPyPI/PyPI when --upload is passed

UPLOAD=false
REPOSITORY_URL="https://test.pypi.org/legacy/"
DIST_DIR="dist/release-local"

PROJECT_VERSION="$(python3 - <<'PY'
import re
from pathlib import Path

text = Path('Cargo.toml').read_text(encoding='utf-8')
in_package = False
for line in text.splitlines():
  stripped = line.strip()
  if stripped.startswith('['):
    in_package = stripped == '[package]'
    continue
  if in_package:
    m = re.match(r'version\s*=\s*"([^"]+)"', stripped)
    if m:
      print(m.group(1))
      break
PY
)"

usage() {
  cat <<'EOF'
Usage: scripts/release_local_simulator.sh [options]

Options:
  --upload                    Upload artifacts with twine after validation
  --repository-url <url>      Twine repository URL (default: TestPyPI)
  --dist-dir <path>           Output directory for assembled release artifacts
  -h, --help                  Show this help

Environment for upload:
  TWINE_USERNAME              Default: __token__
  TWINE_PASSWORD              API token for chosen repository

Examples:
  scripts/release_local_simulator.sh
  scripts/release_local_simulator.sh --upload
  scripts/release_local_simulator.sh --upload --repository-url https://upload.pypi.org/legacy/
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --upload)
      UPLOAD=true
      shift
      ;;
    --repository-url)
      REPOSITORY_URL="${2:-}"
      shift 2
      ;;
    --dist-dir)
      DIST_DIR="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage
      exit 1
      ;;
  esac
done

command -v cargo >/dev/null 2>&1 || { echo "cargo is required" >&2; exit 1; }
command -v python3 >/dev/null 2>&1 || { echo "python3 is required" >&2; exit 1; }

if ! command -v maturin >/dev/null 2>&1; then
  echo "Installing maturin..."
  python3 -m pip install --user maturin
fi

if ! python3 -m pip show twine >/dev/null 2>&1; then
  echo "Installing twine..."
  python3 -m pip install --user twine
fi

echo "[1/6] Running formatting check"
cargo fmt --all --check

echo "[2/6] Running clippy"
cargo clippy --all-targets --all-features -- -D warnings

echo "[3/6] Running tests"
cargo test --all-targets --all-features

echo "[4/6] Building wheel and sdist"
rm -f "target/wheels/neuralbudget-${PROJECT_VERSION}"*.whl
rm -f "target/wheels/neuralbudget-${PROJECT_VERSION}.tar.gz"
maturin build --release --manifest-path Cargo.toml
maturin sdist --manifest-path Cargo.toml --out target/wheels

echo "[5/6] Packaging crate"
rm -f "target/package/neuralbudget-${PROJECT_VERSION}.crate"
cargo package --allow-dirty --no-verify

echo "[6/6] Collecting artifacts in $DIST_DIR"
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"
cp -v "target/wheels/neuralbudget-${PROJECT_VERSION}"*.whl "$DIST_DIR"/
cp -v "target/wheels/neuralbudget-${PROJECT_VERSION}.tar.gz" "$DIST_DIR"/
cp -v "target/package/neuralbudget-${PROJECT_VERSION}.crate" "$DIST_DIR"/

echo "Validating Python distributions with twine check"
python3 -m twine check "$DIST_DIR"/*.whl "$DIST_DIR"/*.tar.gz

echo "Release local simulation succeeded"
ls -1 "$DIST_DIR"

if [[ "$UPLOAD" == "true" ]]; then
  : "${TWINE_PASSWORD:?TWINE_PASSWORD must be set when --upload is used}"
  TWINE_USERNAME="${TWINE_USERNAME:-__token__}"

  echo "Uploading to $REPOSITORY_URL"
  python3 -m twine upload \
    --repository-url "$REPOSITORY_URL" \
    --username "$TWINE_USERNAME" \
    --password "$TWINE_PASSWORD" \
    "$DIST_DIR"/*.whl \
    "$DIST_DIR"/*.tar.gz

  echo "Upload completed"
fi
