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

    // init_if_needed for maker_ata: create ATA for (maker, mint) if missing
    if maker_ata.lamports() == 0 || maker_ata.data_is_empty() {
        pinocchio_associated_token_account::instructions::Create {
            funding_account: maker,
            account: maker_ata,
            wallet: maker,
            mint: _mint_to_raise,
            token_program: _token_program,
            system_program: _system_program,
        }
        .invoke()?;
    }

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
        amount: vault_acc.amount(),
    }
    .invoke_signed(&[signer])?;


  Ok(())
}
