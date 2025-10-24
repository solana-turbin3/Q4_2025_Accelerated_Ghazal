use pinocchio::{
    account_info::AccountInfo, instruction::{Seed, Signer}, msg, sysvars::{clock::Clock, Sysvar}, ProgramResult
};
use pinocchio_token::state::TokenAccount;
use crate::{error::FundRaiserError, state::fundraiser::FundRaiser,constants};
use crate::state::contributor::Contributor;

pub fn process_refund_instruction(
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    msg!("Processing Refund instruction");

let [
        contributor,
        maker,
        mint_to_raise,
        fundraiser,
        contributor_account,
        contributor_ata,
        vault,
        token_program,
        system_program,
       
    ] = accounts else {
        return Err(pinocchio::program_error::ProgramError::NotEnoughAccountKeys);
    };

    // Read and validate state within a limited scope to avoid holding borrows over CPI
    let (contrib_amount, bump) = {
        let contributor_state = Contributor::from_account_info(contributor_account)?;
        let fundraiser_state = FundRaiser::from_account_info(fundraiser)?;

        // Check if the fundraising duration has been reached
        let now = Clock::get()?.unix_timestamp; // i64
        let started = fundraiser_state.time_started() as i64;
        let elapsed_days = ((now - started) / (crate::constants::SECONDS_TO_DAYS as i64)) as u8;
        if elapsed_days < fundraiser_state.duration() {
            return Err(FundRaiserError::FundraiserNotEnded.into());
        }

        // Ensure target not met
        {
            let vault_acc = TokenAccount::from_account_info(vault)?;
            let vault_amount = vault_acc.amount();
            if vault_amount >= fundraiser_state.amount_to_raise() {
                return Err(FundRaiserError::TargetMet.into());
            }
        }

        (contributor_state.amount(), fundraiser_state.bump)
    };

    // Transfer the funds from the vault to the contributor (no outstanding borrows now)
    let bump_arr = [bump];
    let seed = [Seed::from(b"fundraiser"), Seed::from(maker.key()), Seed::from(&bump_arr)];
    let signer = Signer::from(&seed);

    pinocchio_token::instructions::Transfer {
        from: vault,
        to: contributor_ata,
        authority: fundraiser,
        amount: contrib_amount,
    }
    .invoke_signed(&[signer])?;

    // Update the fundraiser state by reducing the amount contributed
    {
        let fundraiser_state = FundRaiser::from_account_info(fundraiser)?;
        let new_total = fundraiser_state.current_amount().saturating_sub(contrib_amount);
        fundraiser_state.set_current_amount(&new_total);
    }

  Ok(())
}
