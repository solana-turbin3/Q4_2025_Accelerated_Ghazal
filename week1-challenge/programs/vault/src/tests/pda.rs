use anchor_lang::prelude::Pubkey;

use crate::tests::constants::{PROGRAM_ID, WHITELIST_PROGRAM_ID, WHITELIST_SEED};
use crate::VAULT_SEED;
use whitelist::constants::CONFIG_SEED;

pub fn get_vault_pda() -> Pubkey {
    Pubkey::find_program_address(&[VAULT_SEED], &PROGRAM_ID).0
}

pub fn get_config_pda() -> Pubkey {
    Pubkey::find_program_address(&[CONFIG_SEED], &WHITELIST_PROGRAM_ID).0
}

pub fn get_whitelist_pda(address: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[WHITELIST_SEED, address.as_ref()], &WHITELIST_PROGRAM_ID).0
}
