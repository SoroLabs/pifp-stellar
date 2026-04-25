# PIFP Formal Verifier

This crate provides a bounded symbolic verification pipeline for proposed
Soroban WASM upgrades.

It does three things:

1. Parses the contract WASM and translates supported bytecode into symbolic
   constraints.
2. Checks protocol invariants with Z3.
3. Fails closed if a counterexample is found or if the WASM uses unsupported
   instructions.

## Usage

```bash
cargo run -p pifp-formal-verifier -- \
  --wasm target/wasm32-unknown-unknown/release/pifp_protocol.wasm
```

The default invariant profile targets the current PIFP contract semantics:

- total supply must never exceed the configured cap
- completed projects remain terminal
- verified projects cannot revert to funding
- deadlines and goals remain positive

## Notes

- The checker is bounded. Use `--max-loop-unroll` to tune exploration depth.
- Opaque calls and memory effects are modeled conservatively so the verifier
  can continue without needing full whole-program synthesis.

