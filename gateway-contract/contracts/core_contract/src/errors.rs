use soroban_sdk::contracterror;

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
