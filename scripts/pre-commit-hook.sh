#!/bin/bash
# PIFP Rust Quality Gate
# This script is meant to be run as a pre-commit hook to ensure code quality.

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "Running PIFP Quality Gates..."

# 1. Check Formatting
echo -n "Checking rustfmt... "
if cargo fmt --all -- --check &> /dev/null; then
    echo -e "${GREEN}PASSED${NC}"
else
    echo -e "${YELLOW}WARNING${NC}"
    echo -e "Formatting issues found. To fix automatically, run: ${GREEN}cargo fmt --all${NC}"
fi

# 2. Check Lints
echo -n "Checking clippy... "
if cargo clippy --all-targets --all-features -- -D warnings &> /dev/null; then
    echo -e "${GREEN}PASSED${NC}"
else
    echo -e "${YELLOW}WARNING${NC}"
    echo -e "Clippy warnings found. Consider fixing them before pushing."
    echo -e "Run ${YELLOW}cargo clippy --all-targets --all-features${NC} to see details."
fi

# 3. Check Unit Tests
echo -n "Checking unit tests... "
if cargo test --lib &> /dev/null; then
    echo -e "${GREEN}PASSED${NC}"
else
    echo -e "${RED}FAILED${NC}"
    echo -e "Some tests failed. Run ${RED}cargo test${NC} to debug."
    # We block on test failures because these usually indicate logic bugs.
    exit 1
fi

echo -e "${GREEN}Quality checks complete!${NC}"
exit 0
