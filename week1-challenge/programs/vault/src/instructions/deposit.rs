use anchor_lang::{prelude::*, solana_program::program::invoke};
use anchor_spl::{
    token_2022::Token2022,
    token_interface::{Mint, TokenAccount},
};
use spl_tlv_account_resolution::state::ExtraAccountMetaList;
use spl_token_2022::instruction::transfer_checked;
use spl_transfer_hook_interface::{
    error::TransferHookError,
    get_extra_account_metas_address,
    instruction::{execute, ExecuteInstruction},
};

use crate::{Vault, VAULT_SEED, WHITELIST_PROGRAM_ID};

#[derive(Accounts)]
pub struct Deposit<'info> {
    pub depositor: Signer<'info>,
    #[account(
        seeds = [VAULT_SEED],
        bump = vault.bump
    )]
    pub vault: Account<'info, Vault>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub depositor_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = vault,
        associated_token::token_program = token_program
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Program<'info, Token2022>,
}

impl<'info> Deposit<'info> {
    pub fn handler<'c: 'info>(
        ctx: Context<'_, '_, 'c, 'info, Deposit<'info>>,
        amount: u64,
    ) -> Result<()> {
        let Deposit {
            depositor,
            depositor_token_account,
            mint,
            token_program,
            vault_token_account,
            ..
        } = ctx.accounts;

        let remaining_accounts = ctx.remaining_accounts;

        let validate_state_pubkey =
            get_extra_account_metas_address(&mint.key(), &WHITELIST_PROGRAM_ID);
        let validate_state_info = remaining_accounts
            .iter()
            .find(|&x| *x.key == validate_state_pubkey)
            .ok_or(TransferHookError::IncorrectAccount)
            .unwrap();

        let program_info = remaining_accounts
            .iter()
            .find(|&x| x.key == &WHITELIST_PROGRAM_ID)
            .ok_or(TransferHookError::IncorrectAccount)
            .unwrap();

        let mut execute_instruction = execute(
            &WHITELIST_PROGRAM_ID,
            &depositor_token_account.key(),
            &mint.key(),
            &vault_token_account.key(),
            &depositor.key(),
            amount,
        );
        let mut execute_account_infos = vec![
            depositor_token_account.to_account_info(),
            mint.to_account_info(),
            vault_token_account.to_account_info(),
            depositor.to_account_info(),
            validate_state_info.clone(),
        ];

        ExtraAccountMetaList::add_to_cpi_instruction::<ExecuteInstruction>(
            &mut execute_instruction,
            &mut execute_account_infos,
            &validate_state_info.try_borrow_data()?,
            remaining_accounts,
        )?;

        let depositor_key = depositor.key();
        let whitelist_seeds: &[&[u8]] = &[b"whitelist", depositor_key.as_ref()];
        let whitelist_pda = Pubkey::find_program_address(whitelist_seeds, &WHITELIST_PROGRAM_ID).0;

        let mut ix = transfer_checked(
            &token_program.key(),
            &depositor_token_account.key(),
            &mint.key(),
            &vault_token_account.key(),
            &depositor.key(),
            &[&depositor.key()],
            amount,
            mint.decimals,
        )
        .unwrap();

        let ctx_account_infos = &mut ctx.accounts.to_account_infos();

        // only append whitelist, whitelisted_address already included
        ix.accounts.push(AccountMeta {
            pubkey: whitelist_pda,
            is_signer: false,
            is_writable: false,
        });
        ctx_account_infos.extend_from_slice(&execute_account_infos[5..]);

        ix.accounts.push(AccountMeta {
            pubkey: validate_state_info.key(),
            is_signer: false,
            is_writable: false,
        });
        ctx_account_infos.push(validate_state_info.clone());

        ix.accounts.push(AccountMeta {
            pubkey: program_info.key(),
            is_signer: false,
            is_writable: false,
        });
        ctx_account_infos.push(program_info.clone());

        invoke(&ix, &ctx_account_infos)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use anchor_lang::prelude::instruction::Instruction;
    use anchor_lang::prelude::AccountMeta;
    use anchor_lang::{InstructionData, ToAccountMetas};
    use anchor_spl::token_interface::TokenAccount;
    use litesvm_token::{CreateAssociatedTokenAccount, MintToChecked};
    use solana_keypair::Keypair;
    use solana_program::native_token::LAMPORTS_PER_SOL;
    use solana_signer::Signer;
    use spl_associated_token_account::get_associated_token_address_with_program_id;
    use spl_transfer_hook_interface::get_extra_account_metas_address;

    use crate::tests::constants::{
        ASSOCIATED_TOKEN_PROGRAM_ID, PROGRAM_ID, SYSTEM_PROGRAM_ID, TOKEN_2022_PROGRAM_ID,
        WHITELIST_PROGRAM_ID,
    };
    use crate::tests::cpi::{
        DepositAccounts, DepositData, InitializeConfigAccounts, InitializeConfigData,
        InitializeExtraAccountMetaListAccounts, InitializeExtraAccountMetaListData,
        InitializeMintAccounts, InitializeMintData, InitializeVaultAccounts, InitializeVaultData,
        InitializeWhitelistAccounts, InitializeWhitelistData,
    };
    use crate::tests::pda::{get_config_pda, get_vault_pda, get_whitelist_pda};
    use crate::tests::utils::{build_and_send_transaction, fetch_account, init_wallet, setup};

    #[test]
    fn deposit_into_vault() {
        let (litesvm, default_payer) = &mut setup();

        let admin = init_wallet(litesvm, LAMPORTS_PER_SOL);
        let depositor = init_wallet(litesvm, LAMPORTS_PER_SOL);
        let mint = Keypair::new();

        let vault_pda = get_vault_pda();
        let vault_ata = get_associated_token_address_with_program_id(
            &vault_pda,
            &mint.pubkey(),
            &TOKEN_2022_PROGRAM_ID,
        );
        let config_pda = get_config_pda();
        let extra_account_meta_list_pda =
            get_extra_account_metas_address(&mint.pubkey(), &WHITELIST_PROGRAM_ID);
        let whitelisted_address = depositor.pubkey();
        let whitelist_pda = get_whitelist_pda(&whitelisted_address);

        let amount = 2;

        let ixs = vec![
            Instruction {
                accounts: InitializeConfigAccounts {
                    admin: admin.pubkey(),
                    config: config_pda,
                    system_program: SYSTEM_PROGRAM_ID,
                }
                .to_account_metas(None),
                data: InitializeConfigData {}.data(),
                program_id: WHITELIST_PROGRAM_ID,
            },
            Instruction {
                accounts: InitializeWhitelistAccounts {
                    payer: admin.pubkey(),
                    whitelist: whitelist_pda,
                    whitelisted_address: depositor.pubkey(),
                    system_program: SYSTEM_PROGRAM_ID,
                }
                .to_account_metas(None),
                data: InitializeWhitelistData {}.data(),
                program_id: WHITELIST_PROGRAM_ID,
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
                program_id: WHITELIST_PROGRAM_ID,
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
                program_id: WHITELIST_PROGRAM_ID,
            },
            Instruction {
                accounts: InitializeVaultAccounts {
                    associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                    mint: mint.pubkey(),
                    payer: admin.pubkey(),
                    system_program: SYSTEM_PROGRAM_ID,
                    token_program: TOKEN_2022_PROGRAM_ID,
                    vault: vault_pda,
                    vault_token_account: vault_ata,
                }
                .to_account_metas(None),
                data: InitializeVaultData {}.data(),
                program_id: PROGRAM_ID,
            },
        ];

        let _ = build_and_send_transaction(litesvm, &[&admin, &mint], &admin.pubkey(), &ixs);

        let depositor_ata =
            CreateAssociatedTokenAccount::new(litesvm, &default_payer, &mint.pubkey())
                .owner(&depositor.pubkey())
                .token_program_id(&TOKEN_2022_PROGRAM_ID)
                .send()
                .unwrap();

        MintToChecked::new(litesvm, &admin, &mint.pubkey(), &depositor_ata, amount)
            .token_program_id(&TOKEN_2022_PROGRAM_ID)
            .send()
            .unwrap();

        let mut ix = Instruction {
            accounts: DepositAccounts {
                depositor: depositor.pubkey(),
                depositor_token_account: depositor_ata,
                mint: mint.pubkey(),
                token_program: TOKEN_2022_PROGRAM_ID,
                vault: vault_pda,
                vault_token_account: vault_ata,
            }
            .to_account_metas(None),
            data: DepositData { amount }.data(),
            program_id: PROGRAM_ID,
        };

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
            pubkey: WHITELIST_PROGRAM_ID,
            is_signer: false,
            is_writable: false,
        });

        let _ = build_and_send_transaction(litesvm, &[&depositor], &depositor.pubkey(), &[ix]);

        let post_depositor_ata_bal = fetch_account::<TokenAccount>(litesvm, &depositor_ata).amount;

        assert_eq!(post_depositor_ata_bal, 0);

        let vault_ata_bal = fetch_account::<TokenAccount>(litesvm, &vault_ata).amount;

        assert_eq!(vault_ata_bal, amount);
    }
}
