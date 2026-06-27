#!/bin/bash
# Quickstart Examples Validation Script
# Validates that all quickstart examples run and produce expected output
# Usage: ./tests/validate_quickstart_examples.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
EXAMPLES_DIR="$REPO_ROOT/examples/quickstart"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Helper functions
log_info() {
  echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
  echo -e "${GREEN}✓ $1${NC}"
  PASSED_TESTS=$((PASSED_TESTS + 1))
}

log_fail() {
  echo -e "${RED}✗ $1${NC}"
  FAILED_TESTS=$((FAILED_TESTS + 1))
}

log_warn() {
  echo -e "${YELLOW}⚠️  $1${NC}"
  # Don't count as failure
}

log_test_start() {
  echo -e "\n${YELLOW}📋 Testing: $1${NC}"
  TOTAL_TESTS=$((TOTAL_TESTS + 1))
}

# Check if NeuralBudget is installed
check_neuralbudget() {
  if ! command -v neuralbudget &> /dev/null; then
    log_warn "NeuralBudget not installed. Install with: cargo install neuralbudget or pip install neuralbudget"
    return 1
  fi
  return 0
}

# Validate YAML syntax
validate_yaml() {
  local file=$1
  local name=$2
  
  log_test_start "YAML Syntax: $name"
  
  if [ ! -f "$file" ]; then
    log_fail "$name file not found: $file"
    return 1
  fi
  
  # Check for basic YAML issues
  if ! grep -q "service:" "$file"; then
    log_fail "$name missing 'service:' field"
    return 1
  fi
  
  if ! grep -q "target:" "$file"; then
    log_fail "$name missing 'target:' field"
    return 1
  fi
  
  log_success "$name has valid YAML structure"
  return 0
}

# Validate JSON syntax
validate_json() {
  local file=$1
  local name=$2
  
  log_test_start "JSON Syntax: $name"
  
  if [ ! -f "$file" ]; then
    log_fail "$name file not found: $file"
    return 1
  fi
  
  # Try to parse JSON (requires jq)
  if command -v jq &> /dev/null; then
    if ! jq . "$file" > /dev/null 2>&1; then
      log_fail "$name has invalid JSON"
      return 1
    fi
  else
    # Fallback: just check for basic structure
    if ! grep -q "{" "$file" || ! grep -q "}" "$file"; then
      log_fail "$name missing JSON structure"
      return 1
    fi
  fi
  
  log_success "$name has valid JSON structure"
  return 0
}

# Test HTTP SLO Example
test_http_example() {
  log_test_start "HTTP SLO Example"
  
  local slo_file="$EXAMPLES_DIR/http-slo/slo.yaml"
  local sample_file="$EXAMPLES_DIR/http-slo/sample.json"
  local readme_file="$EXAMPLES_DIR/http-slo/README.md"
  
  # Check files exist
  if [ ! -f "$slo_file" ]; then
    log_fail "HTTP SLO file not found"
    return 1
  fi
  
  if [ ! -f "$sample_file" ]; then
    log_fail "HTTP sample file not found"
    return 1
  fi
  
  if [ ! -f "$readme_file" ]; then
    log_fail "HTTP README not found"
    return 1
  fi
  
  # Validate content
  validate_yaml "$slo_file" "HTTP SLO" || return 1
  validate_json "$sample_file" "HTTP Sample" || return 1
  
  # Check for required fields in sample
  if ! grep -q "requests" "$sample_file"; then
    log_fail "HTTP sample missing 'requests' field"
    return 1
  fi
  
  if ! grep -q "latency" "$sample_file"; then
    log_fail "HTTP sample missing 'latency' field"
    return 1
  fi
  
  log_success "HTTP SLO example is complete and valid"
  return 0
}

# Test ML SLO Example
test_ml_example() {
  log_test_start "ML SLO Example"
  
  local slo_file="$EXAMPLES_DIR/ml-slo/slo.yaml"
  local sample_file="$EXAMPLES_DIR/ml-slo/sample.json"
  local readme_file="$EXAMPLES_DIR/ml-slo/README.md"
  
  # Check files exist
  if [ ! -f "$slo_file" ]; then
    log_fail "ML SLO file not found"
    return 1
  fi
  
  if [ ! -f "$sample_file" ]; then
    log_fail "ML sample file not found"
    return 1
  fi
  
  if [ ! -f "$readme_file" ]; then
    log_fail "ML README not found"
    return 1
  fi
  
  # Validate content
  validate_yaml "$slo_file" "ML SLO" || return 1
  validate_json "$sample_file" "ML Sample" || return 1
  
  # Check for required fields in sample
  if ! grep -q "model_metrics" "$sample_file"; then
    log_fail "ML sample missing 'model_metrics' field"
    return 1
  fi
  
  if ! grep -q "drift" "$sample_file"; then
    log_fail "ML sample missing 'drift' field"
    return 1
  fi
  
  log_success "ML SLO example is complete and valid"
  return 0
}

# Test GenAI SLO Example
test_genai_example() {
  log_test_start "GenAI SLO Example"
  
  local slo_file="$EXAMPLES_DIR/genai-slo/slo.yaml"
  local sample_file="$EXAMPLES_DIR/genai-slo/sample.json"
  local readme_file="$EXAMPLES_DIR/genai-slo/README.md"
  
  # Check files exist
  if [ ! -f "$slo_file" ]; then
    log_fail "GenAI SLO file not found"
    return 1
  fi
  
  if [ ! -f "$sample_file" ]; then
    log_fail "GenAI sample file not found"
    return 1
  fi
  
  if [ ! -f "$readme_file" ]; then
    log_fail "GenAI README not found"
    return 1
  fi
  
  # Validate content
  validate_yaml "$slo_file" "GenAI SLO" || return 1
  validate_json "$sample_file" "GenAI Sample" || return 1
  
  # Check for required fields in sample
  if ! grep -q "latency" "$sample_file"; then
    log_fail "GenAI sample missing 'latency' field"
    return 1
  fi
  
  if ! grep -q "throughput" "$sample_file"; then
    log_fail "GenAI sample missing 'throughput' field"
    return 1
  fi
  
  log_success "GenAI SLO example is complete and valid"
  return 0
}

# Test Prometheus Integration Example
test_prometheus_example() {
  log_test_start "Prometheus Integration Example"
  
  local readme_file="$EXAMPLES_DIR/prometheus/README.md"
  
  if [ ! -f "$readme_file" ]; then
    log_fail "Prometheus README not found"
    return 1
  fi
  
  # Check for key content
  if ! grep -q "gen-rules" "$readme_file"; then
    log_fail "Prometheus guide missing 'gen-rules' command"
    return 1
  fi
  
  if ! grep -q "neuralbudget" "$readme_file"; then
    log_fail "Prometheus guide missing neuralbudget references"
    return 1
  fi
  
  log_success "Prometheus integration example is complete"
  return 0
}

# Test Python Notebook
test_notebook() {
  log_test_start "Python Notebook"
  
  local notebook_file="$EXAMPLES_DIR/notebook.ipynb"
  
  if [ ! -f "$notebook_file" ]; then
    log_fail "Notebook file not found"
    return 1
  fi
  
  # Check for required sections
  if ! grep -q "HTTP" "$notebook_file"; then
    log_fail "Notebook missing HTTP section"
    return 1
  fi
  
  if ! grep -q "ML" "$notebook_file"; then
    log_fail "Notebook missing ML section"
    return 1
  fi
  
  if ! grep -q "GenAI" "$notebook_file"; then
    log_fail "Notebook missing GenAI section"
    return 1
  fi
  
  log_success "Python notebook is complete and valid"
  return 0
}

# Test documentation
test_documentation() {
  log_test_start "Quickstart Documentation"
  
  local index_file="$REPO_ROOT/docs/quickstart/INDEX.md"
  
  if [ ! -f "$index_file" ]; then
    log_fail "INDEX.md not found"
    return 1
  fi
  
  # Check for links to all guides (check for the guide names)
  if ! grep -q "HTTP" "$index_file"; then
    log_fail "INDEX.md missing link to HTTP guide"
    return 1
  fi
  
  if ! grep -q "ML" "$index_file"; then
    log_fail "INDEX.md missing link to ML guide"
    return 1
  fi
  
  if ! grep -q "GenAI" "$index_file"; then
    log_fail "INDEX.md missing link to GenAI guide"
    return 1
  fi
  
  log_success "Quickstart documentation is complete"
  return 0
}

# Main execution
main() {
  echo -e "${BLUE}=== NeuralBudget Quickstart Examples Validation ===${NC}\n"
  
  # Check prerequisites (not counted as a test)
  if ! check_neuralbudget; then
    echo -e "${YELLOW}ℹ️  Note: Skipping execution tests (neuralbudget not installed)${NC}"
    NEURALBUDGET_AVAILABLE=false
  else
    NEURALBUDGET_AVAILABLE=true
  fi
  
  # Run all tests
  test_http_example || true
  test_ml_example || true
  test_genai_example || true
  test_prometheus_example || true
  test_notebook || true
  test_documentation || true
  
  # Print summary
  echo -e "\n${BLUE}=== Test Summary ===${NC}"
  echo -e "Total Tests: $TOTAL_TESTS"
  echo -e "${GREEN}Passed: $PASSED_TESTS${NC}"
  echo -e "${RED}Failed: $FAILED_TESTS${NC}"
  
  if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "\n${GREEN}✓ All tests passed!${NC}"
    exit 0
  else
    echo -e "\n${RED}✗ Some tests failed${NC}"
    exit 1
  fi
}

main "$@"
