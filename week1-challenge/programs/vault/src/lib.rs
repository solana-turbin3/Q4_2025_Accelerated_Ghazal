pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;
pub mod tests;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

// Re-export the `whitelist` crate for tests (so `crate::whitelist::...` resolves)
#[cfg(test)]
pub use whitelist as whitelist;

declare_id!("EmLpgnrx4SB1kQaEZnTMnHEUVsVTJAcuxyPabds52Jed");

#[program]
pub mod vault {
    use super::*;

    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        InitializeVault::handler(ctx)
    }

    pub fn deposit<'c: 'info, 'info>(
        ctx: Context<'_, '_, 'c, 'info, Deposit<'info>>,
        amount: u64,
    ) -> Result<()> {
        Deposit::handler(ctx, amount)
    }

    pub fn withdraw<'c: 'info, 'info>(
        ctx: Context<'_, '_, 'c, 'info, Withdraw<'info>>,
        amount: u64,
    ) -> Result<()> {
        Withdraw::handler(ctx, amount)
    }
}
