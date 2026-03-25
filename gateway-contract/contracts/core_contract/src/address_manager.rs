use soroban_sdk::{contracttype, panic_with_error, Address, Bytes, BytesN, Env};

use crate::errors::ChainAddressError;
use crate::events::CHAIN_ADD;
use crate::registration::DataKey as CommitmentKey;
use crate::types::ChainType;

/// Storage key for chain addresses.
/// Keyed by (username_hash, chain) → raw address bytes.
#[contracttype]
#[derive(Clone)]
pub enum ChainAddrKey {
    ChainAddress(BytesN<32>, ChainType),
}

pub struct AddressManager;

impl AddressManager {
    /// Link an external chain address to a registered username commitment.
    ///
    /// - `caller`        – must be the registered owner of `username_hash`
    /// - `username_hash` – 32-byte Poseidon commitment of the username
    /// - `chain`         – target chain (`Evm`, `Bitcoin`, or `Solana`)
    /// - `address`       – raw address bytes (ASCII string representation)
    ///
    /// Emits `CHAIN_ADD` event with `(username_hash, chain, address)`.
    pub fn add_chain_address(
        env: Env,
        caller: Address,
        username_hash: BytesN<32>,
        chain: ChainType,
        address: Bytes,
    ) {
        // 1. Authenticate the caller.
        caller.require_auth();

        // 2. Verify the commitment is registered and caller is the owner.
        let owner_key = CommitmentKey::Commitment(username_hash.clone());
        let owner: Address = env
            .storage()
            .persistent()
            .get(&owner_key)
            .unwrap_or_else(|| panic_with_error!(&env, ChainAddressError::NotRegistered));

        if owner != caller {
            panic_with_error!(&env, ChainAddressError::Unauthorized);
        }

        // 3. Validate the address format for the given chain.
        if !Self::validate_address(&chain, &address) {
            panic_with_error!(&env, ChainAddressError::InvalidAddress);
        }

        // 4. Persist the chain address.
        let key = ChainAddrKey::ChainAddress(username_hash.clone(), chain.clone());
        env.storage().persistent().set(&key, &address);

        // 5. Emit event.
        #[allow(deprecated)]
        env.events()
            .publish((CHAIN_ADD,), (username_hash, chain, address));
    }

    /// Retrieve the stored chain address for a given commitment and chain type.
    /// Returns `None` if not set.
    pub fn get_chain_address(
        env: Env,
        username_hash: BytesN<32>,
        chain: ChainType,
    ) -> Option<Bytes> {
        let key = ChainAddrKey::ChainAddress(username_hash, chain);
        env.storage().persistent().get(&key)
    }

    // ── Validation helpers ──────────────────────────────────────────────────

    fn validate_address(chain: &ChainType, address: &Bytes) -> bool {
        let len = address.len();
        match chain {
            // EVM: "0x" prefix + 40 hex chars = 42 ASCII bytes.
            ChainType::Evm => {
                len == 42
                    && address.get(0) == Some(0x30) // '0'
                    && address.get(1) == Some(0x78) // 'x'
            }
            // Bitcoin: legacy (25–34 chars), P2SH (34), or bech32 (42–62).
            ChainType::Bitcoin => (25..=62).contains(&len),
            // Solana: base58-encoded public key, typically 32–44 chars.
            ChainType::Solana => (32..=44).contains(&len),
        }
    }
}
