# PIFP Execution Engine

This crate provides a low-latency off-chain execution engine for AMM arbitrage
monitoring on Stellar.

It is intentionally self-contained so it can run without Horizon:

- WebSocket snapshot ingestion from an RPC-compatible endpoint
- Bellman-Ford route discovery across pool graphs
- Fee-bump planning for next-ledger execution

## Usage

Snapshot file mode:

```bash
cargo run -p pifp-execution-engine -- --snapshot-file snapshots.json
```

WebSocket mode:

```bash
cargo run -p pifp-execution-engine -- --ws-url ws://localhost:8000
```

The JSON message format accepts either a single pool snapshot or a batch of
snapshots:

```json
{
  "pool_id": "pool-1",
  "base_asset": "A",
  "quote_asset": "B",
  "base_reserve": 1000.0,
  "quote_reserve": 1015.0,
  "fee_bps": 30,
  "updated_ledger": 123456
}
```

