use pinocchio::{
    account_info::AccountInfo, instruction::{Seed, Signer}, msg, ProgramResult
};
use pinocchio_token::state::TokenAccount;
use crate::{error::FundRaiserError, state::fundraiser::FundRaiser};

pub fn process_checker_instruction(
    accounts: &[AccountInfo],
    _data: &[u8],
) -> ProgramResult {

    msg!("Processing Checker instruction");

    let [
        maker,
        _mint_to_raise,
        fundraiser,
        vault,
        maker_ata,
        _token_program,
        _system_program,
         _associated_token_program,
       // _rent_sysvar @ ..
    ] = accounts else {
        return Err(pinocchio::program_error::ProgramError::NotEnoughAccountKeys);
    };

 let fundraiser_state = FundRaiser::from_account_info(fundraiser)?;

  
let vault_acc = TokenAccount::from_account_info(vault)?;
          let vault_amount = vault_acc.amount();
          if vault_amount < fundraiser_state.amount_to_raise() {
            return Err(FundRaiserError::TargetNotMet.into());
            }
         

 // Transfer the funds from the vault to the maker
    let bump_arr = [fundraiser_state.bump.to_le()];
    let seed = [Seed::from(b"fundraiser"), Seed::from(maker.key()), Seed::from(&bump_arr)];
    let signer = Signer::from(&seed);

    pinocchio_token::instructions::Transfer {
        from: vault,
        to: maker_ata,
        authority: fundraiser,
        amount: vault.lamports(),
    }
    .invoke_signed(&[signer])?;


  Ok(())
}