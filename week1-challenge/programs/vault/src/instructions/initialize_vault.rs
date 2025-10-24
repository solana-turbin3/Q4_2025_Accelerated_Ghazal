use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::Token2022,
    token_interface::{Mint, TokenAccount},
};

use crate::{Vault, VAULT_SEED};

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = Vault::DISCRIMINATOR.len() + Vault::INIT_SPACE,
        seeds = [VAULT_SEED],
        bump,
    )]
    pub vault: Account<'info, Vault>,
    #[account(mint::token_program = token_program)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint,
        associated_token::authority = vault,
        associated_token::token_program = token_program,
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> InitializeVault<'info> {
    pub fn handler(ctx: Context<InitializeVault>) -> Result<()> {
        ctx.accounts.vault.set_inner(Vault {
            bump: ctx.bumps.vault,
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use anchor_lang::prelude::instruction::Instruction;
    use anchor_lang::{InstructionData, ToAccountMetas};
    use solana_keypair::Keypair;
    use solana_program::native_token::LAMPORTS_PER_SOL;
    use solana_signer::Signer;
    use spl_associated_token_account::get_associated_token_address_with_program_id;

    use crate::tests::constants::{
        ASSOCIATED_TOKEN_PROGRAM_ID, PROGRAM_ID, SYSTEM_PROGRAM_ID, TOKEN_2022_PROGRAM_ID,
        WHITELIST_PROGRAM_ID,
    };
    use crate::tests::cpi::{
        InitializeMintAccounts, InitializeMintData, InitializeVaultAccounts, InitializeVaultData,
    };
    use crate::tests::pda::get_vault_pda;
    use crate::tests::utils::{build_and_send_transaction, init_wallet, setup};

    #[test]
    fn initialize_a_vault() {
        let (litesvm, _default_payer) = &mut setup();

        let admin = init_wallet(litesvm, LAMPORTS_PER_SOL);
        let mint = Keypair::new();

        let ix = Instruction {
            accounts: InitializeMintAccounts {
                mint: mint.pubkey(),
                payer: admin.pubkey(),
                system_program: SYSTEM_PROGRAM_ID,
                token_program: TOKEN_2022_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: InitializeMintData {}.data(),
            program_id: WHITELIST_PROGRAM_ID,
        };

        let _ = build_and_send_transaction(litesvm, &[&admin, &mint], &admin.pubkey(), &[ix]);

        let vault_pda = get_vault_pda();
        let vault_ata = get_associated_token_address_with_program_id(
            &vault_pda,
            &mint.pubkey(),
            &TOKEN_2022_PROGRAM_ID,
        );

        let ix = Instruction {
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
        };

        let _ = build_and_send_transaction(litesvm, &[&admin], &admin.pubkey(), &[ix]);

        let vault_acc = litesvm.get_account(&vault_pda);

        assert!(vault_acc.is_some());
    }
}