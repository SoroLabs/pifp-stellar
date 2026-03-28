#!/bin/bash
set -e

echo "Running DevOps Sanity Checks for PIFP Stellar Environment..."

# Check Rust
echo -n "Checking Rust... "
if command -v rustc >/dev/null 2>&1; then
    rustc --version
else
    echo "FAILED: rustc not found"
    exit 1
fi

# Check Cargo
echo -n "Checking Cargo... "
if command -v cargo >/dev/null 2>&1; then
    cargo --version
else
    echo "FAILED: cargo not found"
    exit 1
fi

# Check WebAssembly targets
echo -n "Checking WebAssembly targets... "
MISSING=""
if ! rustup target list | grep -q "wasm32-unknown-unknown (installed)"; then
    MISSING="wasm32-unknown-unknown"
fi
if ! rustup target list | grep -q "wasm32v1-none (installed)"; then
    [ -n "$MISSING" ] && MISSING="$MISSING and "
    MISSING="${MISSING}wasm32v1-none"
fi

if [ -z "$MISSING" ]; then
    echo "installed"
else
    echo -e "FAILED: $MISSING target(s) not found. Run: \n  rustup target add wasm32-unknown-unknown wasm32v1-none"
    exit 1
fi

# Check stellar-cli (Soroban CLI)
echo -n "Checking stellar-cli... "
if command -v stellar >/dev/null 2>&1 || command -v soroban >/dev/null 2>&1; then
    echo "installed"
else
    echo "FAILED: stellar-cli (or soroban-cli) not found"
    exit 1
fi

echo "All sanity checks passed successfully! Your container environment is ready."
