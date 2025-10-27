use pinocchio::{
    account_info::AccountInfo, msg, sysvars::{clock::Clock, Sysvar}, ProgramResult
};
use crate::{error::FundRaiserError, state::fundraiser::FundRaiser,constants};
use crate::state::contributor::Contributor;
use pinocchio_token::state::Mint;
pub fn process_contribute_instruction(
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {


    msg!("Processing Contribute instruction");

    let [
        contributor,
        mint_to_raise,
        fundraiser,
        contributor_account,
        contributor_ata,
        vault,
        _token_program,
        _system_program,
       // _rent_sysvar @ ..
    ] = accounts else {
        return Err(pinocchio::program_error::ProgramError::NotEnoughAccountKeys);
    };

    // Accept provided contributor account (tests pre-create a program-owned account)
    msg!("contribute: post-destructure");

    // Derive canonical PDA: [b"contributor", fundraiser, contributor]
    let seeds_raw = [
        b"contributor".as_ref(),
        fundraiser.key().as_ref(),
        contributor.key().as_ref(),
    ];
    msg!("contribute: deriving PDA");
    let (expected_pda, bump) = pinocchio::pubkey::find_program_address(&seeds_raw, &crate::ID);
    msg!("contribute: derived contributor PDA");
    // Create if needed (init_if_needed) only when empty and the key matches the canonical PDA
    if (contributor_account.lamports() == 0 || contributor_account.data_is_empty())
        && expected_pda == *contributor_account.key()
    {
        msg!("contribute: creating contributor PDA");
        let bump_arr = [bump];
        let seeds = [
            pinocchio::instruction::Seed::from(b"contributor"),
            pinocchio::instruction::Seed::from(fundraiser.key()),
            pinocchio::instruction::Seed::from(contributor.key()),
            pinocchio::instruction::Seed::from(&bump_arr),
        ];
        let signer = pinocchio::instruction::Signer::from(&seeds);
        pinocchio_system::instructions::CreateAccount {
            from: contributor,
            to: contributor_account,
            lamports: pinocchio::sysvars::rent::Rent::get()?.minimum_balance(Contributor::LEN),
            space: Contributor::LEN as u64,
            owner: &crate::ID,
        }
        .invoke_signed(&[signer])?;

        let st = Contributor::from_account_info(contributor_account)?;
        st.set_amount(&0);
    }
    msg!("contribute: init_if_needed handled");

    ///
    let contributor_state = Contributor::from_account_info(contributor_account)?;
    let fundraiser_state = FundRaiser::from_account_info(fundraiser)?;
    msg!("contribute: loaded accounts");
    
    let amount = u64::from_le_bytes(data[0..8].try_into().unwrap());

 // Check if the amount to contribute meets the minimum amount required
    let mint = Mint::from_account_info(mint_to_raise)?;
    msg!("contribute: loaded mint");
            let decimals = mint.decimals();
            let min_unit = 10_u64.pow(decimals as u32);
            if amount < min_unit {
            return Err(FundRaiserError::ContributionTooSmall.into());
            }


        // Check if the amount to contribute is less than the maximum allowed contribution
        let max_per_contributor = (fundraiser_state.amount_to_raise() * constants::MAX_CONTRIBUTION_PERCENTAGE) / constants::PERCENTAGE_SCALER;
        if amount > max_per_contributor {
            return Err(FundRaiserError::ContributionTooBig.into());
            }

        // Check if the fundraising duration has been reached
        let current_time = Clock::get()?.unix_timestamp; // i64
        let started = fundraiser_state.time_started() as i64;
        let elapsed_days = ((current_time - started) / constants::SECONDS_TO_DAYS) as u8;
        if fundraiser_state.duration() <= elapsed_days {
            return Err(FundRaiserError::FundraiserEnded.into());
            }

        // Check if the maximum contributions per contributor have been reached
        let new_total = contributor_state.amount().saturating_add(amount);
           if new_total > max_per_contributor {
            return Err(FundRaiserError::MaximumContributionsReached.into());
            }

 // Transfer the funds from the contributor to the vault
 if !contributor.is_signer() { return Err(pinocchio::program_error::ProgramError::MissingRequiredSignature); }
  pinocchio_token::instructions::Transfer {
        from: contributor_ata,
        to: vault,
        authority: contributor,
        amount: amount,
    }.invoke()?;


    let new_total = contributor_state.amount().saturating_add(amount);
    contributor_state.set_amount(&new_total);
    let raised = fundraiser_state.current_amount().saturating_add(amount);
    fundraiser_state.set_current_amount(&raised);

      Ok(())
}
