use anchor_lang::prelude::*;

#[error_code]
pub enum WhitelistedError {
    #[msg("Whitelist admin does not match")]
    InvalidAdmin,
    #[msg("Transfer hook cannot be directly invoked")]
    NotTransferring,
    #[msg("Address is blocked from transferring tokens")]
    AddressBlocked,
}