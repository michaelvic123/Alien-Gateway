use soroban_sdk::{Bytes, Env};

use crate::types::PublicSignals;

pub struct ZkVerifier;

impl ZkVerifier {
    /// Verify a Groth16 non-inclusion proof against the given public signals.
    ///
    /// Phase 4 placeholder — always returns `true` until the on-chain ZK verifier
    /// contract is deployed and integrated.
    ///
    /// TODO(phase-4): replace with a cross-contract call to the ZK verifier once
    /// it is available on-chain.
    pub fn verify_groth16_proof(
        _env: &Env,
        _proof: &Bytes,
        _public_signals: &PublicSignals,
    ) -> bool {
        true
    }
}
