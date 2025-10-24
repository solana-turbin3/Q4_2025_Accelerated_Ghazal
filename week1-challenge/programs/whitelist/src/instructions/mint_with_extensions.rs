use anchor_lang::prelude::*;
use anchor_spl::{token_2022::Token2022, token_interface::Mint};

#[derive(Accounts)]
pub struct InitializeMint<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        mint::decimals = 6,
        mint::authority = payer,
        extensions::transfer_hook::authority = payer,
        extensions::transfer_hook::program_id = crate::ID,
        // Add a second Token-2022 extension for uniqueness: Permanent Delegate
        extensions::permanent_delegate::delegate = payer,
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token2022>,
}

impl<'info> InitializeMint<'info> {
    pub fn handler() -> Result<()> {
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

    use crate::tests::constants::{PROGRAM_ID, SYSTEM_PROGRAM_ID, TOKEN_2022_PROGRAM_ID};
    use crate::tests::cpi::{InitializeMintAccounts, InitializeMintData};
    use crate::tests::utils::{build_and_send_transaction, init_wallet, setup};

    #[test]
    fn initialize_mint() {
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
            program_id: PROGRAM_ID,
        };

        let _ = build_and_send_transaction(litesvm, &[&admin, &mint], &admin.pubkey(), &[ix]);

        let mint_acc = litesvm.get_account(&mint.pubkey()).unwrap();
        assert!(mint_acc.owner == TOKEN_2022_PROGRAM_ID);

        // Verify PermanentDelegate extension got set to `admin`
        use spl_token_2022::extension::permanent_delegate::PermanentDelegate;
        use spl_token_2022::extension::{BaseStateWithExtensions, StateWithExtensionsOwned};
        use anchor_lang::prelude::Pubkey;
        use spl_pod::optional_keys::OptionalNonZeroPubkey;

        let state = StateWithExtensionsOwned::<spl_token_2022::state::Mint>::unpack(mint_acc.data).unwrap();
        let perm = state.get_extension::<PermanentDelegate>().unwrap();
        let delegate_opt: Option<Pubkey> = Option::<Pubkey>::from(perm.delegate);
        assert_eq!(delegate_opt, Some(admin.pubkey()));
    }
}
