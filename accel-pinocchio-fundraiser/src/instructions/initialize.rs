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
       // _rent_sysvar @ ..
    ] = accounts else {
        return Err(pinocchio::program_error::ProgramError::NotEnoughAccountKeys);
    };
   
    let bump = data[0];
    let seed = [b"fundraiser".as_ref(), maker.key().as_slice(), &[bump]];
    //let seeds = &seed[..];

    let fundraiser_account_pda = derive_address(&seed, None, &crate::ID);
    log(&fundraiser_account_pda);
    log(&fundraiser.key());


    assert_eq!(fundraiser_account_pda, *fundraiser.key());
    // if fundraiser_account_pda != *fundraiser.key() {
    // return Err(pinocchio::program_error::ProgramError::InvalidSeeds);
    // }
   
    let mut i = 1;
    let amount_to_raise = u64::from_le_bytes(data[i..i+8].try_into().unwrap()); i += 8;
    let current_amount = u64::from_le_bytes(data[i..i+8].try_into().unwrap()); i += 8;
    let time_started = u64::from_le_bytes(data[i..i+8].try_into().unwrap()); i += 8;
    let duration = data[i];
   
    let bump = [bump.to_le()];
    let seed = [Seed::from(b"fundraiser"), Seed::from(maker.key()), Seed::from(&bump)];
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
            fundraiser_state.set_current_amount(&current_amount);
            fundraiser_state.set_time_started(&time_started); 
            fundraiser_state.set_duration(duration);  
            fundraiser_state.bump = data[0];
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