use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ContractError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    RootMismatch = 3,
    InvalidProof = 4,
    DuplicateCommitment = 5,
#[repr(u32)]
pub enum CoreError {
    /// The requested resource was not found.
    NotFound = 1,
    /// The SMT root has not been set yet.
    RootNotSet = 2,
    /// Commitment is already registered.
    DuplicateCommitment = 3,
    /// public_signals.old_root does not match the current on-chain SMT root.
    StaleRoot = 4,
    /// The supplied Groth16 proof is invalid.
    InvalidProof = 5,
    /// The username is registered but has no primary Stellar address linked.
    NoAddressLinked = 6,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ChainAddressError {
    /// Caller is not the owner of the username commitment.
    Unauthorized = 1,
    /// The username commitment is not registered.
    NotRegistered = 2,
    /// The address format is invalid for the given chain type.
    InvalidAddress = 3,
}
