pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;
pub mod tests;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

use spl_discriminator::discriminator::SplDiscriminate;
use spl_transfer_hook_interface::instruction::{
    ExecuteInstruction, InitializeExtraAccountMetaListInstruction,
};

declare_id!("5ydZBsgy14hPhvnfoWFg8fpghR91T6uJvtKknbd7yuBK");

#[program]
pub mod whitelist {

    use super::*;

    pub fn initialize_config(ctx: Context<InitializeConfig>) -> Result<()> {
        InitializeConfig::handler(ctx)
    }

    pub fn initialize_whitelist(ctx: Context<InitializeWhitelist>) -> Result<()> {
        InitializeWhitelist::handler(ctx)
    }

    pub fn update_whitelist(ctx: Context<UpdateWhitelist>, is_blocked: bool) -> Result<()> {
        UpdateWhitelist::handler(ctx, is_blocked)
    }

    pub fn initialize_mint(_ctx: Context<InitializeMint>) -> Result<()> {
        InitializeMint::handler()
    }

    #[instruction(discriminator = InitializeExtraAccountMetaListInstruction::SPL_DISCRIMINATOR_SLICE)]
    pub fn initialize_extra_account_meta_list(
        ctx: Context<InitializeExtraAccountMetaList>,
    ) -> Result<()> {
        InitializeExtraAccountMetaList::handler(ctx)
    }

    #[instruction(discriminator = ExecuteInstruction::SPL_DISCRIMINATOR_SLICE)]
    pub fn transfer_hook(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
        TransferHook::handler(ctx, amount)
    }
}