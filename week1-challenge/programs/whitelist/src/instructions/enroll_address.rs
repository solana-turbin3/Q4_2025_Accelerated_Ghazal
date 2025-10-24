use anchor_lang::prelude::*;

use crate::{Whitelist, WHITELIST_SEED};

#[derive(Accounts)]
pub struct InitializeWhitelist<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = Whitelist::DISCRIMINATOR.len() + Whitelist::INIT_SPACE,
        seeds = [WHITELIST_SEED, whitelisted_address.key().as_ref()],
        bump
    )]
    pub whitelist: Account<'info, Whitelist>,
    /// CHECK: Authority of token account
    pub whitelisted_address: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitializeWhitelist<'info> {
    pub fn handler(ctx: Context<InitializeWhitelist>) -> Result<()> {
        let InitializeWhitelist {
            whitelist,
            whitelisted_address,
            ..
        } = ctx.accounts;

        whitelist.set_inner(Whitelist {
            address: whitelisted_address.key(),
            is_blocked: false,
            bump: ctx.bumps.whitelist,
        });

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
    use crate::tests::cpi::{
        InitializeConfigAccounts, InitializeConfigData, InitializeWhitelistAccounts,
        InitializeWhitelistData,
    };
    use crate::tests::pda::{get_config_pda, get_whitelist_pda};
    use crate::tests::utils::{build_and_send_transaction, fetch_account, init_wallet, setup};
    use crate::Whitelist;

    #[test]
    fn initialize_whitelist() {
        let (litesvm, _default_payer) = &mut setup();

        let admin = init_wallet(litesvm, LAMPORTS_PER_SOL);
        let address1 = init_wallet(litesvm, LAMPORTS_PER_SOL);
        let address2 = init_wallet(litesvm, LAMPORTS_PER_SOL);
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

        for address in [&address1, &address2] {
            let whitelist_pda = get_whitelist_pda(&address.pubkey());

            let ix = Instruction {
                accounts: InitializeWhitelistAccounts {
                    payer: admin.pubkey(),
                    whitelist: whitelist_pda,
                    whitelisted_address: address.pubkey(),
                    system_program: SYSTEM_PROGRAM_ID,
                }
                .to_account_metas(None),
                data: InitializeWhitelistData {}.data(),
                program_id: PROGRAM_ID,
            };

            let _ = build_and_send_transaction(litesvm, &[&admin], &admin.pubkey(), &[ix]);

            let whitelist_acc = fetch_account::<Whitelist>(litesvm, &whitelist_pda);

            assert_eq!(whitelist_acc.address, address.pubkey());
        }
    }
}