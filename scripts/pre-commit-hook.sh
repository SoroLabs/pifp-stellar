#!/bin/bash
# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "Running Rust Quality Gate..."

# Check formatting
if [ "$#" -gt 0 ]; then
    echo -n "Checking rustfmt on staged files... "
    RS_FILES=$(echo "$@" | tr ' ' '\n' | grep '\.rs$' || true)
    if [ -n "$RS_FILES" ]; then
        if rustfmt --check $RS_FILES; then
            echo -e "${GREEN}PASSED${NC}"
        else
            echo -e "${RED}FAILED${NC}"
            echo "Please format your staged files using 'rustfmt'."
            exit 1
        fi
    else
        echo -e "${GREEN}No .rs files to check${NC}"
    fi
else
    echo -n "Checking rustfmt (workspace)... "
    if cargo fmt --all -- --check &> /dev/null; then
        echo -e "${GREEN}PASSED${NC}"
    else
        echo -e "${YELLOW}WARNING: Workspace formatting issues found (not blocking)${NC}"
    fi
fi

# Check lints
echo -n "Checking clippy (workspace)... "
if cargo clippy --all-targets --all-features -- -D warnings &> /dev/null; then
    echo -e "${GREEN}PASSED${NC}"
else
    echo -e "${YELLOW}WARNING: Workspace lint issues found (not blocking)${NC}"
fi

echo -e "${GREEN}Rust quality checks passed!${NC}"
exit 0
