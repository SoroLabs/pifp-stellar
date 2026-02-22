#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "Setting up Pre-Commit Quality Gates..."

# 1. Check if pre-commit is installed
if ! command -v pre-commit &> /dev/null; then
    echo -e "${YELLOW}Warning: 'pre-commit' tool not found.${NC}"
    echo "It is highly recommended to install it: 'pip install pre-commit'"
    echo "Falling back to manual git hook installation..."
    
    # Fallback: Manual installation of pre-commit hook
    HOOK_PATH=".git/hooks/pre-commit"
    echo "#!/bin/bash" > "$HOOK_PATH"
    echo "./scripts/pre-commit-hook.sh" >> "$HOOK_PATH"
    chmod +x "$HOOK_PATH"
    echo -e "${GREEN}Manual git hook installed at $HOOK_PATH${NC}"
else
    # Standard installation
    pre-commit install
    echo -e "${GREEN}Pre-commit hooks installed successfully!${NC}"
fi

echo -e "${GREEN}Setup complete.${NC}"
