use pinocchio::{account_info::AccountInfo, entrypoint, pubkey::Pubkey, ProgramResult};

use crate::instructions::FundRaiserInstrctions;

mod state;
mod instructions;
mod error;
mod constants;

entrypoint!(process_instruction);

pinocchio_pubkey::declare_id!("9rcdaF2bdQVq3TjrL756VqcZWWYgLdZXJX79soxNoUjr");

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    assert_eq!(program_id, &ID);

    let (discriminator, data) = instruction_data
        .split_first()
        .ok_or(pinocchio::program_error::ProgramError::InvalidInstructionData)?;

    match FundRaiserInstrctions::try_from(discriminator)? {
        FundRaiserInstrctions::Initialize =>
            instructions::process_initialize_instruction(accounts, data)?,
        FundRaiserInstrctions::Contribute =>
            instructions::process_contribute_instruction(accounts, data)?,
        FundRaiserInstrctions::Checker =>
            instructions::process_checker_instruction(accounts, data)?,
        FundRaiserInstrctions::Refund =>
            instructions::process_refund_instruction(accounts, data)?,
        _ => return Err(pinocchio::program_error::ProgramError::InvalidInstructionData),
    }
    Ok(())
}

#[cfg(test)]
mod tests;

