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

# Check WebAssembly target
echo -n "Checking WebAssembly target... "
if rustup target list | grep -q "wasm32-unknown-unknown (installed)"; then
    echo "installed"
else
    echo "FAILED: wasm32-unknown-unknown target not found"
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
