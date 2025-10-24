use anchor_lang::prelude::Pubkey;

use crate::{tests::constants::PROGRAM_ID, CONFIG_SEED, WHITELIST_SEED};

pub fn get_config_pda() -> Pubkey {
    Pubkey::find_program_address(&[CONFIG_SEED], &PROGRAM_ID).0
}

pub fn get_whitelist_pda(address: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[WHITELIST_SEED, address.as_ref()], &PROGRAM_ID).0
}