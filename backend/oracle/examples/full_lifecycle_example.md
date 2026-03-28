# Full PIFP Lifecycle Example

This document walks through a complete end-to-end example of the PIFP protocol, from project registration to fund release.

## Scenario

A charity wants to build 10 water wells in a rural area. They need $50,000 in funding and will provide photo evidence of completed wells as proof of impact.

## Step 1: Prepare Proof Artifact

Before registering the project, the charity prepares a template of what the proof will look like:

```json
{
  "project": "Rural Water Wells Initiative",
  "location": "Region XYZ",
  "wells_completed": 10,
  "evidence": [
    {
      "well_id": 1,
      "gps_coordinates": "12.3456, -78.9012",
      "photo_ipfs": "QmPhoto1...",
      "completion_date": "2024-03-15"
    }
  ],
  "verified_by": "Independent Auditor Inc.",
  "verification_date": "2024-03-20"
}
```

They compute the SHA-256 hash of this template:

```bash
echo -n '{"project":"Rural Water Wells Initiative",...}' | sha256sum
# Output: a1b2c3d4e5f6... (32 bytes)
```

## Step 2: Upload Proof Template to IPFS

```bash
ipfs add proof_template.json
# Output: QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG
```

## Step 3: Register Project on PIFP Contract

Using the Stellar CLI:

```bash
stellar contract invoke \
  --id CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM \
  --source CREATOR_SECRET_KEY \
  --network testnet \
  -- register_project \
  --creator GCREATOR... \
  --accepted-tokens '["CUSDC..."]' \
  --goal 50000000000 \
  --proof-hash a1b2c3d4e5f6... \
  --deadline 1735689600
```

Response:
```json
{
  "id": 42,
  "creator": "GCREATOR...",
  "accepted_tokens": ["CUSDC..."],
  "goal": 50000000000,
  "proof_hash": "a1b2c3d4e5f6...",
  "deadline": 1735689600,
  "status": "Funding",
  "donation_count": 0
}
```

## Step 4: Donors Fund the Project

Multiple donors contribute:

```bash
# Donor 1: $10,000
stellar contract invoke \
  --id CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM \
  --source DONOR1_SECRET_KEY \
  --network testnet \
  -- deposit \
  --project-id 42 \
  --donator GDONOR1... \
  --token CUSDC... \
  --amount 10000000000

# Donor 2: $25,000
stellar contract invoke \
  --id CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM \
  --source DONOR2_SECRET_KEY \
  --network testnet \
  -- deposit \
  --project-id 42 \
  --donator GDONOR2... \
  --token CUSDC... \
  --amount 25000000000

# Donor 3: $15,000
stellar contract invoke \
  --id CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM \
  --source DONOR3_SECRET_KEY \
  --network testnet \
  -- deposit \
  --project-id 42 \
  --donator GDONOR3... \
  --token CUSDC... \
  --amount 15000000000
```

After the third donation, the project reaches its goal and transitions to `Active` status.

## Step 5: Charity Completes the Work

Over the next 3 months, the charity builds the 10 water wells and collects evidence:

- GPS coordinates of each well
- Photos of completed wells
- Independent auditor verification
- Community testimonials

They compile this into the final proof document matching the template structure.

## Step 6: Upload Final Proof to IPFS

```bash
ipfs add final_proof.json
# Output: QmFinalProof123456789abcdefghijklmnopqrstuvwxyz
```

They verify the hash matches:

```bash
sha256sum final_proof.json
# Output: a1b2c3d4e5f6... ✓ MATCHES!
```

## Step 7: Oracle Verifies and Releases Funds

The oracle service is triggered (manually or automatically):

```bash
cd backend/oracle

# First, dry run to verify
./examples/verify_project.sh 42 QmFinalProof123456789abcdefghijklmnopqrstuvwxyz --dry-run
```

Output:
```
=== PIFP Oracle Verification ===
Project ID: 42
Proof CID: QmFinalProof123456789abcdefghijklmnopqrstuvwxyz
Contract: CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM

Starting verification...
INFO pifp_oracle: PIFP Oracle starting - Project ID: 42
INFO pifp_oracle::verifier: Fetching proof from: https://ipfs.io/ipfs/QmFinalProof...
INFO pifp_oracle::verifier: Downloaded 2048 bytes from IPFS
INFO pifp_oracle: Computed proof hash: a1b2c3d4e5f6...
WARN pifp_oracle: DRY RUN MODE - Transaction will not be submitted
INFO pifp_oracle: Would submit verify_and_release for project 42

✓ Dry run complete. Remove --dry-run to submit the transaction.
```

Everything looks good! Now submit for real:

```bash
./examples/verify_project.sh 42 QmFinalProof123456789abcdefghijklmnopqrstuvwxyz
```

Output:
```
=== PIFP Oracle Verification ===
Project ID: 42
Proof CID: QmFinalProof123456789abcdefghijklmnopqrstuvwxyz
Contract: CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM

Starting verification...
INFO pifp_oracle: PIFP Oracle starting - Project ID: 42
INFO pifp_oracle::verifier: Fetching proof from: https://ipfs.io/ipfs/QmFinalProof...
INFO pifp_oracle::verifier: Downloaded 2048 bytes from IPFS
INFO pifp_oracle: Computed proof hash: a1b2c3d4e5f6...
INFO pifp_oracle: Submitting verify_and_release transaction to contract
INFO pifp_oracle::chain: Building verify_and_release transaction for project 42
INFO pifp_oracle::chain: Simulating transaction...
INFO pifp_oracle::chain: ✓ Transaction simulation successful
INFO pifp_oracle::chain: Submitting transaction to network...
INFO pifp_oracle: ✓ Verification transaction submitted successfully!
INFO pifp_oracle: Transaction hash: abc123def456...
INFO pifp_oracle: Project 42 funds released

✓ Verification complete! Funds have been released.
```

## Step 8: Verify Fund Release

Check the project status:

```bash
stellar contract invoke \
  --id CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM \
  --network testnet \
  -- get_project \
  --id 42
```

Response:
```json
{
  "id": 42,
  "creator": "GCREATOR...",
  "accepted_tokens": ["CUSDC..."],
  "goal": 50000000000,
  "proof_hash": "a1b2c3d4e5f6...",
  "deadline": 1735689600,
  "status": "Completed",  // ✓ Status changed!
  "donation_count": 3
}
```

Check the creator's balance:

```bash
stellar contract invoke \
  --id CUSDC... \
  --network testnet \
  -- balance \
  --id GCREATOR...
```

The creator now has $50,000 USDC! 🎉

## Summary

| Step | Actor | Action | Result |
|------|-------|--------|--------|
| 1-2 | Charity | Prepare proof template and upload to IPFS | CID: QmYwAPJzv... |
| 3 | Charity | Register project with proof hash | Project ID: 42, Status: Funding |
| 4 | Donors | Deposit funds (3 donations totaling $50k) | Status: Active (goal reached) |
| 5 | Charity | Complete work and collect evidence | 10 wells built |
| 6 | Charity | Upload final proof to IPFS | CID: QmFinalProof... |
| 7 | Oracle | Verify proof and submit transaction | Status: Completed |
| 8 | Charity | Receive funds | $50,000 USDC transferred |

## Key Takeaways

1. **Trust-minimized**: Funds locked in contract, not controlled by any party
2. **Transparent**: All actions recorded on-chain with events
3. **Verifiable**: Proof hash ensures evidence matches expectations
4. **Automated**: Oracle service handles verification automatically
5. **Refundable**: If deadline passes without completion, donors can reclaim funds

## Error Scenarios

### Scenario A: Wrong Proof Submitted

If the charity submits a different proof that doesn't match the hash:

```bash
./examples/verify_project.sh 42 QmWrongProof...
```

Output:
```
ERROR pifp_oracle::chain: Contract error: Verification failed (proof hash mismatch) (code: 16)
```

Funds remain locked. Charity must submit the correct proof.

### Scenario B: Deadline Passes Without Completion

If the deadline passes and the project is not verified:

```bash
stellar contract invoke \
  --id CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM \
  --network testnet \
  -- expire_project \
  --project-id 42
```

Project status changes to `Expired`. Donors can now call `refund` to reclaim their funds.

### Scenario C: Oracle Tries to Verify Twice

If the oracle tries to verify an already-completed project:

```bash
./examples/verify_project.sh 42 QmFinalProof...
```

Output:
```
ERROR pifp_oracle::chain: Contract error: Project already completed (milestone already released) (code: 3)
```

Transaction rejected. Funds can only be released once.

## Next Steps

- Integrate oracle with event monitoring for automatic triggering
- Add multi-oracle quorum for increased security
- Implement ZK-STARK proofs for privacy-preserving verification
- Build frontend dashboard for project tracking
