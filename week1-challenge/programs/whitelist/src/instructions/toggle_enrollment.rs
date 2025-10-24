use anchor_lang::prelude::*;

use crate::{error::WhitelistedError, Config, Whitelist, CONFIG_SEED, WHITELIST_SEED};

#[derive(Accounts)]
pub struct UpdateWhitelist<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = admin @ WhitelistedError::InvalidAdmin,
    )]
    pub config: Account<'info, Config>,
    #[account(
        mut,
        seeds = [WHITELIST_SEED, whitelisted_address.key().as_ref()],
        bump = whitelist.bump,
    )]
    pub whitelist: Account<'info, Whitelist>,
    /// CHECK: Authority of token account
    pub whitelisted_address: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> UpdateWhitelist<'info> {
    pub fn handler(ctx: Context<UpdateWhitelist>, is_blocked: bool) -> Result<()> {
        let UpdateWhitelist { whitelist, .. } = ctx.accounts;

        whitelist.is_blocked = is_blocked;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use anchor_lang::prelude::instruction::Instruction;
    use anchor_lang::prelude::AccountMeta;
    use anchor_lang::{InstructionData, ToAccountMetas};
    use litesvm_token::{CreateAssociatedTokenAccount, MintToChecked};
    use solana_keypair::Keypair;
    use solana_program::native_token::LAMPORTS_PER_SOL;
    use solana_signer::Signer;
    use spl_token_2022::instruction::transfer_checked;
    use spl_transfer_hook_interface::get_extra_account_metas_address;

    use crate::tests::constants::{
        MINT_DECIMALS, PROGRAM_ID, SYSTEM_PROGRAM_ID, TOKEN_2022_PROGRAM_ID,
    };
    use crate::tests::cpi::{
        InitializeConfigAccounts, InitializeConfigData, InitializeExtraAccountMetaListAccounts,
        InitializeExtraAccountMetaListData, InitializeMintAccounts, InitializeMintData,
        InitializeWhitelistAccounts, InitializeWhitelistData, UpdateWhitelistAccounts,
        UpdateWhitelistData,
    };
    use crate::tests::pda::{get_config_pda, get_whitelist_pda};
    use crate::tests::utils::{build_and_send_transaction, init_wallet, setup};

    #[test]
    fn block_whitelist() {
        let (litesvm, default_payer) = &mut setup();

        let admin = init_wallet(litesvm, LAMPORTS_PER_SOL);
        let address1 = init_wallet(litesvm, LAMPORTS_PER_SOL);
        let address2 = init_wallet(litesvm, LAMPORTS_PER_SOL);
        let mint = Keypair::new();

        let config_pda = get_config_pda();
        let whitelist_pda = get_whitelist_pda(&address1.pubkey());
        let extra_account_meta_list_pda =
            get_extra_account_metas_address(&mint.pubkey(), &PROGRAM_ID);
        let whitelisted_address = address1.pubkey();

        let ixs = vec![
            Instruction {
                accounts: InitializeConfigAccounts {
                    admin: admin.pubkey(),
                    config: config_pda,
                    system_program: SYSTEM_PROGRAM_ID,
                }
                .to_account_metas(None),
                data: InitializeConfigData {}.data(),
                program_id: PROGRAM_ID,
            },
            Instruction {
                accounts: InitializeWhitelistAccounts {
                    payer: admin.pubkey(),
                    whitelist: whitelist_pda,
                    whitelisted_address: address1.pubkey(),
                    system_program: SYSTEM_PROGRAM_ID,
                }
                .to_account_metas(None),
                data: InitializeWhitelistData {}.data(),
                program_id: PROGRAM_ID,
            },
            Instruction {
                accounts: InitializeMintAccounts {
                    mint: mint.pubkey(),
                    payer: admin.pubkey(),
                    system_program: SYSTEM_PROGRAM_ID,
                    token_program: TOKEN_2022_PROGRAM_ID,
                }
                .to_account_metas(None),
                data: InitializeMintData {}.data(),
                program_id: PROGRAM_ID,
            },
            Instruction {
                accounts: InitializeExtraAccountMetaListAccounts {
                    extra_account_meta_list: extra_account_meta_list_pda,
                    mint: mint.pubkey(),
                    payer: admin.pubkey(),
                    system_program: SYSTEM_PROGRAM_ID,
                }
                .to_account_metas(None),
                data: InitializeExtraAccountMetaListData {}.data(),
                program_id: PROGRAM_ID,
            },
        ];

        let _ = build_and_send_transaction(litesvm, &[&admin, &mint], &admin.pubkey(), &ixs);

        let ata1 = CreateAssociatedTokenAccount::new(litesvm, &default_payer, &mint.pubkey())
            .owner(&address1.pubkey())
            .token_program_id(&TOKEN_2022_PROGRAM_ID)
            .send()
            .unwrap();
        let ata2 = CreateAssociatedTokenAccount::new(litesvm, &default_payer, &mint.pubkey())
            .owner(&address2.pubkey())
            .token_program_id(&TOKEN_2022_PROGRAM_ID)
            .send()
            .unwrap();

        let pre_ata_1_bal = 10;

        MintToChecked::new(litesvm, &admin, &mint.pubkey(), &ata1, pre_ata_1_bal)
            .token_program_id(&TOKEN_2022_PROGRAM_ID)
            .send()
            .unwrap();

        let ix = Instruction {
            accounts: UpdateWhitelistAccounts {
                admin: admin.pubkey(),
                config: config_pda,
                system_program: SYSTEM_PROGRAM_ID,
                whitelist: whitelist_pda,
                whitelisted_address,
            }
            .to_account_metas(None),
            data: UpdateWhitelistData { is_blocked: true }.data(),
            program_id: PROGRAM_ID,
        };

        let _ = build_and_send_transaction(litesvm, &[&admin], &admin.pubkey(), &[ix]);

        let amount = 3;

        let mut ix = transfer_checked(
            &TOKEN_2022_PROGRAM_ID,
            &ata1,
            &mint.pubkey(),
            &ata2,
            &address1.pubkey(),
            &[&address1.pubkey()],
            amount,
            MINT_DECIMALS,
        )
        .unwrap();

        ix.accounts.push(AccountMeta {
            pubkey: whitelist_pda,
            is_signer: false,
            is_writable: false,
        });

        ix.accounts.push(AccountMeta {
            pubkey: extra_account_meta_list_pda,
            is_signer: false,
            is_writable: false,
        });

        ix.accounts.push(AccountMeta {
            pubkey: PROGRAM_ID,
            is_signer: false,
            is_writable: false,
        });

        let tx_meta = build_and_send_transaction(litesvm, &[&address1], &address1.pubkey(), &[ix]);

        assert!(tx_meta.is_err());
    }
}