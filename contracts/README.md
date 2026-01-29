# Nestera Smart Contracts: Off-Chain Oracle Architecture

This directory contains the Soroban smart contracts for Nestera, implementing a cost-efficient "Off-Chain Oracle" architecture. This allows users to pay for gas fees while ensuring all minting requests are cryptographically authorized by the Admin.

## How it Works

1.  **Admin Authorization**: The Admin generates a cryptographic signature for a `MintPayload` off-chain using their Ed25519 private key.
2.  **User Submission**: The user receives the payload and signature and submits them to the `mint` function on-chain.
3.  **On-Chain Verification**: The contract verifies the signature against the stored Admin public key before allowing the minting process to proceed.

## Admin: Signing Payloads Off-Chain

The Admin must sign the `MintPayload` using an Ed25519 private key. The payload must be serialized to XDR format to ensure consistency with the on-chain verification.

### Example (Rust)
Using the `ed25519-dalek` library:

```rust
use ed25519_dalek::{Signer, SigningKey};
use soroban_sdk::xdr::ToXdr;

// 1. Create the payload
let payload = MintPayload {
    user: user_address,
    amount: 100,
    timestamp: current_time,
    expiry_duration: 3600,
};

// 2. Serialize to XDR
let payload_bytes = payload.to_xdr(&env);

// 3. Sign with Admin private key
let signature = signing_key.sign(&payload_bytes);
```

## User: Submitting Minting Requests

Users call the `mint` function themselves, providing the authorized payload and the signature.

### Example (Stellar CLI)

```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source <USER_IDENTITY> \
  --network testnet \
  -- mint \
  --payload '{ "user": "...", "amount": 100, "timestamp": 1737511200, "expiry_duration": 3600 }' \
  --signature <64_BYTE_HEX_SIGNATURE>
```

## Security & Validation

- **Signature Verification**: The contract uses `env.crypto().ed25519_verify()` to ensure the signature is valid.
- **Expiry Protection**: Each payload includes a `timestamp` and `expiry_duration`. The contract panics if the current ledger time exceeds the expiry.
- **Tamper Resistance**: Any change to the payload (e.g., increasing the amount) will result in an invalid signature and a contract panic.

## Development

### Building
```bash
cargo build --target wasm32-unknown-unknown --release
```

### Testing
```bash
cargo test
```
