use anchor_lang::prelude::*;

use crate::{Config, CONFIG_SEED};

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        seeds = [CONFIG_SEED],
        bump,
        space = Config::DISCRIMINATOR.len() + Config::INIT_SPACE,
    )]
    pub config: Account<'info, Config>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitializeConfig<'info> {
    pub fn handler(ctx: Context<InitializeConfig>) -> Result<()> {
        let InitializeConfig { admin, config, .. } = ctx.accounts;

        config.set_inner(Config { admin: admin.key(), bump: ctx.bumps.config });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use anchor_lang::prelude::instruction::Instruction;
    use anchor_lang::{InstructionData, ToAccountMetas};
    use solana_program::native_token::LAMPORTS_PER_SOL;
    use solana_signer::Signer;

    use crate::tests::constants::{PROGRAM_ID, SYSTEM_PROGRAM_ID};
    use crate::tests::cpi::{InitializeConfigAccounts, InitializeConfigData};
    use crate::tests::pda::get_config_pda;
    use crate::tests::utils::{build_and_send_transaction, fetch_account, init_wallet, setup};
    use crate::Config;

    #[test]
    fn initialize_config() {
        let (litesvm, _default_payer) = &mut setup();

        let admin = init_wallet(litesvm, LAMPORTS_PER_SOL);
        let config_pda = get_config_pda();

        let ix = Instruction {
            accounts: InitializeConfigAccounts {
                admin: admin.pubkey(),
                config: config_pda,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: InitializeConfigData {}.data(),
            program_id: PROGRAM_ID,
        };

        let _ = build_and_send_transaction(litesvm, &[&admin], &admin.pubkey(), &[ix]);

        let config_acc = fetch_account::<Config>(litesvm, &config_pda);

        assert_eq!(config_acc.admin, admin.pubkey());
    }
}
