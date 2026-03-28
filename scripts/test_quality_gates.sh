#!/bin/bash
# Verification script for quality gates

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "Verifying Quality Gates..."

# Helper for cleanup
cleanup() {
    rm -f test_fail.rs test_fail.yaml
    git checkout -- contracts/pifp_protocol/src/lib.rs &> /dev/null || true
}
trap cleanup EXIT

# 1. Success Case: Current codebase
echo -n "Test 1: Clean codebase... "
if ./scripts/pre-commit-hook.sh &> /dev/null; then
    echo -e "${GREEN}PASSED${NC}"
else
    echo -e "${RED}FAILED (Clean codebase should pass)${NC}"
    exit 1
fi

# 2. Failure Case: Bad Formatting
echo -n "Test 2: Bad formatting... "
echo "fn   test_fmt()  {   }" > test_fail.rs
if ./scripts/pre-commit-hook.sh test_fail.rs &> /dev/null; then
    echo -e "${RED}FAILED (Bad formatting should be caught)${NC}"
    exit 1
else
    echo -e "${GREEN}PASSED${NC}"
fi
rm test_fail.rs

# 3. Failure Case: Lints (Clippy)
# Clippy checks the whole project, so this should still fail
echo -n "Test 3: Clippy warnings... "
sed -i 's/pub struct PifpProtocol;/pub struct PifpProtocol; fn unused() {}/' contracts/pifp_protocol/src/lib.rs

if ./scripts/pre-commit-hook.sh &> /dev/null; then
    echo -e "${RED}FAILED (Lints should be caught)${NC}"
    exit 1
else
    echo -e "${GREEN}PASSED${NC}"
fi
git checkout -- contracts/pifp_protocol/src/lib.rs

echo -e "${GREEN}All quality gate verifications successful!${NC}"
exit 0
