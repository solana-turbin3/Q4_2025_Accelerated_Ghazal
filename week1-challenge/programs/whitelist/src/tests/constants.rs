use anchor_lang::prelude::Pubkey;

pub static SYSTEM_PROGRAM_ID: Pubkey = anchor_lang::system_program::ID;
pub static TOKEN_PROGRAM_ID: Pubkey = anchor_spl::token::ID;
pub static TOKEN_2022_PROGRAM_ID: Pubkey = anchor_spl::token_2022::ID;
pub static ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey = spl_associated_token_account::ID;

pub static PROGRAM_ID: Pubkey = crate::ID;
pub const MINT_DECIMALS: u8 = 6;