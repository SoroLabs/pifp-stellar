#!/bin/bash
# Example: Verify a project and release funds
#
# Usage: ./verify_project.sh <project_id> <proof_cid> [--dry-run]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check arguments
if [ $# -lt 2 ]; then
    echo -e "${RED}Error: Missing required arguments${NC}"
    echo "Usage: $0 <project_id> <proof_cid> [--dry-run]"
    echo ""
    echo "Example:"
    echo "  $0 42 QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG --dry-run"
    exit 1
fi

PROJECT_ID=$1
PROOF_CID=$2
DRY_RUN_FLAG=""

if [ "$3" == "--dry-run" ]; then
    DRY_RUN_FLAG="--dry-run"
    echo -e "${YELLOW}Running in DRY RUN mode - no transaction will be submitted${NC}"
fi

# Check if .env exists
if [ ! -f .env ]; then
    echo -e "${RED}Error: .env file not found${NC}"
    echo "Please copy .env.example to .env and configure it:"
    echo "  cp .env.example .env"
    exit 1
fi

# Load environment variables
source .env

# Validate required variables
if [ -z "$CONTRACT_ID" ]; then
    echo -e "${RED}Error: CONTRACT_ID not set in .env${NC}"
    exit 1
fi

if [ -z "$ORACLE_SECRET_KEY" ]; then
    echo -e "${RED}Error: ORACLE_SECRET_KEY not set in .env${NC}"
    exit 1
fi

echo -e "${GREEN}=== PIFP Oracle Verification ===${NC}"
echo "Project ID: $PROJECT_ID"
echo "Proof CID: $PROOF_CID"
echo "Contract: $CONTRACT_ID"
echo ""

# Build if needed
if [ ! -f "../../target/release/pifp-oracle" ]; then
    echo -e "${YELLOW}Building oracle service...${NC}"
    cargo build --release
    echo ""
fi

# Run the oracle
echo -e "${GREEN}Starting verification...${NC}"
RUST_LOG=info cargo run --release -- \
    --project-id "$PROJECT_ID" \
    --proof-cid "$PROOF_CID" \
    $DRY_RUN_FLAG

echo ""
if [ -z "$DRY_RUN_FLAG" ]; then
    echo -e "${GREEN}✓ Verification complete! Funds have been released.${NC}"
else
    echo -e "${YELLOW}✓ Dry run complete. Remove --dry-run to submit the transaction.${NC}"
fi
