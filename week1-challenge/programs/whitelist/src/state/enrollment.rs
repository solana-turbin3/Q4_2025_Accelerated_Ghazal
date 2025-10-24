use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Whitelist {
    pub address: Pubkey,
    pub is_blocked: bool,
    pub bump: u8,
}