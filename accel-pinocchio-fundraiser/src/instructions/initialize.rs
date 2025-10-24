use pinocchio::{
    account_info::AccountInfo, instruction::{Seed, Signer}, msg, pubkey::log, sysvars::{rent::Rent, Sysvar}, ProgramResult
};
use pinocchio_pubkey::derive_address;
use pinocchio_system::instructions::CreateAccount;

use crate::state::fundraiser::FundRaiser;
pub fn process_initialize_instruction(
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {


    msg!("Processing Initialize instruction");

    let [
        maker,
        mint_to_raise,
        fundraiser,
        vault,
        system_program,
        token_program,
        _associated_token_program,
    ] = accounts else {
        return Err(pinocchio::program_error::ProgramError::NotEnoughAccountKeys);
    };
   
    //let bump = data[0];
    let mut bump_opt: Option<u8> = None;
    for b in 0u8..=u8::MAX {
    let seed = [b"fundraiser".as_ref(), maker.key().as_slice(), &[b]];
    if derive_address(&seed, None, &crate::ID) == *fundraiser.key() {
    bump_opt = Some(b);
    break;
    }
    }
    let bump = bump_opt.ok_or(pinocchio::program_error::ProgramError::InvalidArgument)?;
    //
    let seed = [b"fundraiser".as_ref(), maker.key().as_slice(), &[bump]];
    let fundraiser_account_pda = derive_address(&seed, None, &crate::ID);
    log(&fundraiser_account_pda);
    log(&fundraiser.key());
    assert_eq!(fundraiser_account_pda, *fundraiser.key());
 
    let mut i = 0;
    let amount_to_raise = u64::from_le_bytes(data[i..i+8].try_into().unwrap()); i += 8;
    let duration = data[i];
   
    //let bump = [bump.to_le()];
    let bump_arr = [bump];
    let seed = [Seed::from(b"fundraiser"), Seed::from(maker.key()), Seed::from(&bump_arr)];//&bump)];
    let seeds = Signer::from(&seed);

    if fundraiser.owner() != &crate::ID {
        CreateAccount {
            from: maker,
            to: fundraiser,
            lamports: Rent::get()?.minimum_balance(FundRaiser::LEN),
            space: FundRaiser::LEN as u64,
            owner: &crate::ID,
        }.invoke_signed(&[seeds.clone()])?;


        {
            let fundraiser_state = FundRaiser::from_account_info(fundraiser)?;
        
            fundraiser_state.set_maker(maker.key());
            fundraiser_state.set_mint_to_raise(mint_to_raise.key());
            fundraiser_state.set_amount_to_raise(&amount_to_raise);
            fundraiser_state.set_current_amount(&0);
            fundraiser_state.set_time_started(&(pinocchio::sysvars::clock::Clock::get()?.unix_timestamp as u64));
            fundraiser_state.set_duration(duration);
            fundraiser_state.bump = bump;
        }
    }
    else {
        return Err(pinocchio::program_error::ProgramError::IllegalOwner);
    }

    pinocchio_associated_token_account::instructions::Create {
        funding_account: maker,
        account: vault,
        wallet: fundraiser,
        mint: mint_to_raise,
        token_program: token_program,
        system_program: system_program,
    }.invoke()?;


    Ok(())
}

