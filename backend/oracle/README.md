# PIFP Oracle Service

Standalone Rust service that verifies proof-of-impact and triggers fund release on the PIFP Soroban contract.

## Overview

The Oracle service is the critical off-chain component that:

1. **Fetches** proof artifacts from IPFS using the provided CID
2. **Computes** SHA-256 hash of the proof data
3. **Submits** `verify_and_release` transaction to the Soroban contract

## Architecture

```
┌─────────────┐
│   Oracle    │
│   Service   │
└──────┬──────┘
       │
       ├──► IPFS Gateway (fetch proof)
       │
       └──► Soroban RPC (submit verification)
                │
                ▼
         ┌──────────────┐
         │ PIFP Contract│
         │verify_and_   │
         │  release()   │
         └──────────────┘
```

## Installation

```bash
cd backend/oracle
cargo build --release
```

## Configuration

Create a `.env` file in the `backend/oracle` directory:

```env
# Required
RPC_URL=https://soroban-testnet.stellar.org
CONTRACT_ID=C...  # Your deployed PIFP contract address
ORACLE_SECRET_KEY=S...  # Oracle's Stellar secret key

# Optional (with defaults)
HORIZON_URL=https://horizon-testnet.stellar.org
IPFS_GATEWAY=https://ipfs.io
NETWORK_PASSPHRASE=Test SDF Network ; September 2015
TIMEOUT_SECS=30
```

### Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `RPC_URL` | No | Soroban RPC endpoint (default: testnet) |
| `HORIZON_URL` | No | Horizon API endpoint (default: testnet) |
| `CONTRACT_ID` | **Yes** | PIFP contract address (C...) |
| `ORACLE_SECRET_KEY` | **Yes** | Oracle's signing key (S...) |
| `IPFS_GATEWAY` | No | IPFS gateway URL (default: ipfs.io) |
| `NETWORK_PASSPHRASE` | No | Network passphrase (default: testnet) |
| `TIMEOUT_SECS` | No | Request timeout in seconds (default: 30) |

## Usage

### Basic Verification

```bash
cargo run -- \
  --project-id 42 \
  --proof-cid QmXxx...
```

### Dry Run Mode

Test without submitting the transaction:

```bash
cargo run -- \
  --project-id 42 \
  --proof-cid QmXxx... \
  --dry-run
```

This will:
- Fetch the proof from IPFS
- Compute the hash
- Log the result without submitting to the blockchain

### With Logging

```bash
RUST_LOG=info cargo run -- \
  --project-id 42 \
  --proof-cid QmXxx...
```

Log levels: `error`, `warn`, `info`, `debug`, `trace`

## CLI Options

```
pifp-oracle - Verify proofs and release funds

USAGE:
    pifp-oracle --project-id <ID> --proof-cid <CID> [--dry-run]

OPTIONS:
    --project-id <ID>     Project ID to verify
    --proof-cid <CID>     IPFS CID of the proof artifact
    --dry-run             Compute hash and log without submitting transaction
    -h, --help            Print help information
```

## Error Handling

The service handles various error scenarios:

### Network Errors
- IPFS gateway unreachable
- RPC endpoint timeout
- Connection failures

**Action**: Retry with exponential backoff (implement in production)

### Proof Errors
- CID not found (404)
- Empty proof artifact
- Proof too large (>100MB)

**Action**: Verify CID is correct and artifact is uploaded

### Contract Errors
- **Project not found** (Error code 1): Invalid project ID
- **Already completed** (Error code 3): Funds already released
- **Not authorized** (Error code 6): Oracle role not granted
- **Project expired** (Error code 14): Deadline passed
- **Verification failed** (Error code 16): Proof hash mismatch

**Action**: Check project status and proof hash before retrying

## Testing

Run unit tests:

```bash
cargo test
```

Run with verbose output:

```bash
cargo test -- --nocapture
```

## Production Deployment

### Security Considerations

1. **Key Management**
   - Store `ORACLE_SECRET_KEY` in a secure vault (e.g., HashiCorp Vault, AWS Secrets Manager)
   - Never commit secrets to version control
   - Use hardware security modules (HSM) for production keys

2. **Network Security**
   - Use private RPC endpoints to avoid rate limiting
   - Implement request signing for authenticated IPFS gateways
   - Enable TLS certificate validation

3. **Monitoring**
   - Log all verification attempts with timestamps
   - Alert on repeated failures
   - Track transaction costs and success rates

### Recommended Improvements

- [ ] Implement retry logic with exponential backoff
- [ ] Add transaction fee estimation
- [ ] Support batch verification for multiple projects
- [ ] Integrate with monitoring systems (Prometheus, Datadog)
- [ ] Add webhook notifications on success/failure
- [ ] Implement proper XDR transaction building and signing
- [ ] Add support for custom IPFS pinning services
- [ ] Cache proof hashes to avoid redundant downloads

## Integration with PIFP Protocol

### Prerequisites

1. Deploy the PIFP contract to Soroban
2. Initialize the contract with `init(super_admin)`
3. Grant Oracle role to your oracle address:
   ```bash
   stellar contract invoke \
     --id <CONTRACT_ID> \
     --source <ADMIN_KEY> \
     -- set_oracle \
     --caller <ADMIN_ADDRESS> \
     --oracle <ORACLE_ADDRESS>
   ```

### Workflow

1. **Project Registration**: Creator registers project with `proof_hash`
2. **Funding**: Donors deposit funds into the project
3. **Work Completion**: Implementer uploads proof to IPFS
4. **Oracle Verification**: Run this service with the IPFS CID
5. **Fund Release**: Contract releases funds to creator on successful verification

## Troubleshooting

### "Missing required environment variable: CONTRACT_ID"
- Ensure `.env` file exists in `backend/oracle/`
- Check that `CONTRACT_ID` is set and starts with 'C'

### "IPFS fetch failed: connection timeout"
- Verify IPFS gateway is accessible
- Try alternative gateway (e.g., `https://cloudflare-ipfs.com`)
- Increase `TIMEOUT_SECS`

### "Contract error: Not authorized (code: 6)"
- Verify oracle address has Oracle role
- Check that `ORACLE_SECRET_KEY` matches the granted oracle address

### "Contract error: Verification failed (code: 16)"
- Proof hash mismatch - verify the correct CID was provided
- Ensure proof artifact hasn't been modified since registration

## License

See [LICENSE](../../LICENSE) in the repository root.
