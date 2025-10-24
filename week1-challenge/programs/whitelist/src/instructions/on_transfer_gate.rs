use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::spl_token_2022::{
        extension::{
            transfer_hook::TransferHookAccount, BaseStateWithExtensionsMut,
            PodStateWithExtensionsMut,
        },
        pod::PodAccount,
    },
    token_interface::{Mint, TokenAccount},
};

use crate::{
    error::WhitelistedError,
    state::Whitelist,
    EXTRA_ACCOUNT_METAS_SEED, WHITELIST_SEED,
};

#[derive(Accounts)]
pub struct TransferHook<'info> {
    #[account(
        token::mint = mint,
        token::authority = owner,
    )]
    pub source_token: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        token::mint = mint,
    )]
    pub destination_token: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: source token account owner, can be SystemAccount or PDA owned by another program
    pub owner: UncheckedAccount<'info>,
    /// CHECK: ExtraAccountMetaList Account
    #[account(
        seeds = [EXTRA_ACCOUNT_METAS_SEED, mint.key().as_ref()],
        bump
    )]
    pub extra_account_meta_list: UncheckedAccount<'info>,
    #[account(
        seeds = [WHITELIST_SEED, owner.key().as_ref()],
        bump = whitelist.bump,
    )]
    pub whitelist: Account<'info, Whitelist>,
}

impl<'info> TransferHook<'info> {
    fn check_is_transferring(&mut self) -> Result<()> {
        let source_token_info = self.source_token.to_account_info();
        let mut account_data_ref = source_token_info.try_borrow_mut_data()?;

        let mut account = PodStateWithExtensionsMut::<PodAccount>::unpack(*account_data_ref)?;
        let account_extension = account.get_extension_mut::<TransferHookAccount>()?;

        require!(
            bool::from(account_extension.transferring),
            WhitelistedError::NotTransferring
        );

        Ok(())
    }

    pub fn handler(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
        ctx.accounts.check_is_transferring()?;

        // PDA-only path (default)
        require!(
            !ctx.accounts.whitelist.is_blocked,
            WhitelistedError::AddressBlocked
        );

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
    use spl_token_2022::instruction::transfer_checked;
    use spl_transfer_hook_interface::get_extra_account_metas_address;

    use crate::tests::constants::{
        MINT_DECIMALS, PROGRAM_ID, SYSTEM_PROGRAM_ID, TOKEN_2022_PROGRAM_ID,
    };
    use crate::tests::cpi::{
        InitializeConfigAccounts, InitializeConfigData, InitializeExtraAccountMetaListAccounts,
        InitializeExtraAccountMetaListData, InitializeMintAccounts, InitializeMintData,
        InitializeWhitelistAccounts, InitializeWhitelistData,
    };
    use crate::tests::pda::{get_config_pda, get_whitelist_pda};
    use crate::tests::utils::{build_and_send_transaction, fetch_account, init_wallet, setup};

    #[test]
    fn trigger_transfer_hook() {
        let (litesvm, default_payer) = &mut setup();

        let admin = init_wallet(litesvm, LAMPORTS_PER_SOL);
        let address1 = init_wallet(litesvm, LAMPORTS_PER_SOL);
        let address2 = init_wallet(litesvm, LAMPORTS_PER_SOL);
        let mint = Keypair::new();

        let config_pda = get_config_pda();
        let whitelist_pda = get_whitelist_pda(&address1.pubkey());
        let extra_account_meta_list_pda =
            get_extra_account_metas_address(&mint.pubkey(), &PROGRAM_ID);

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
        let pre_ata_2_bal = 0;

        MintToChecked::new(litesvm, &admin, &mint.pubkey(), &ata1, pre_ata_1_bal)
            .token_program_id(&TOKEN_2022_PROGRAM_ID)
            .send()
            .unwrap();

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

        let _ = build_and_send_transaction(litesvm, &[&address1], &address1.pubkey(), &[ix]);

        let post_ata_1_bal = fetch_account::<TokenAccount>(litesvm, &ata1).amount;

        assert_eq!(pre_ata_1_bal, post_ata_1_bal + amount);

        let post_ata_2_bal = fetch_account::<TokenAccount>(litesvm, &ata2).amount;

        assert_eq!(pre_ata_2_bal, post_ata_2_bal - amount);
    }
}
