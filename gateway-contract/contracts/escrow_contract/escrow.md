# 🏺 Escrow Contract

The Escrow contract provides a secure mechanism for scheduling and executing payments between identities on the Alien Gateway. It uses a vault-based system where funds are reserved before being released.

## Features

- **Scheduled Payments**: Lock funds with a future release timestamp.
- **Privacy-First**: Uses commitment identifiers (`BytesN<32>`) for senders and recipients.
- **Vault Security**: Funds are held in a contract-managed vault with strict ownership verification.

## Interface

### `schedule_payment`

Schedules a payment from a user's vault to another identity.

```rust
pub fn schedule_payment(
    env: Env,
    from: BytesN<32>,
    to: BytesN<32>,
    amount: i128,
    release_at: u64,
) -> u32
```

**Parameters:**

- `from`: The commitment identifying the sender's vault.
- `to`: The commitment identifying the recipient.
- `amount`: The amount of tokens to reserve.
- `release_at`: The ledger timestamp at or after which the payment can be executed.

**Requirements:**

- The caller must be authenticated as the owner of the `from` vault.
- `amount` must be greater than 0.
- `amount` must be less than or equal to the current vault balance.
- `release_at` must be in the future (relative to the current ledger timestamp).

**Returns:**

- `payment_id`: A unique, incrementing identifier for the scheduled payment.

## Security

- **Authentication**: All sensitive operations require authority from the vault owner via `vault.owner.require_auth()`.
- **Integrity**: Funds are immediately reserved in the vault balance at scheduling time by decrementing `VaultState.balance`, preventing double-spending.
- **Validation**: Strict checks on timestamp and amount ensure no past-dated or invalid payments are created.
