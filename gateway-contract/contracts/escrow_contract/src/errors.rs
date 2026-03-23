use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum EscrowError {
    InsufficientBalance = 1,
    PastReleaseTime = 2,
    Unauthorized = 3,
    InvalidAmount = 4,
    VaultNotFound = 5,
}
