use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FundRaiser {

     maker: [u8; 32],
     mint_to_raise: [u8; 32],
     amount_to_raise: [u8; 8],
     current_amount: [u8; 8],
     time_started: [u8; 8],
     duration: [u8; 1],
    pub bump: u8,
}

impl FundRaiser {
    pub const LEN: usize = 32 + 32 + 8 + 8 + 8 + 1 + 1;

    pub fn from_account_info(account_info: &AccountInfo) -> Result<&mut Self, ProgramError> {
        let mut data: pinocchio::account_info::RefMut<'_, [u8]> = account_info.try_borrow_mut_data()?;
        if data.len() != FundRaiser::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        if (data.as_ptr() as usize) % core::mem::align_of::<Self>() != 0 {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(unsafe { &mut *(data.as_mut_ptr() as *mut Self) })
    }


    pub fn maker(&self) -> pinocchio::pubkey::Pubkey {
        pinocchio::pubkey::Pubkey::from(self.maker)
    }

    pub fn set_maker(&mut self, maker: &pinocchio::pubkey::Pubkey) {
        self.maker.copy_from_slice(maker.as_ref());
    }

    pub fn mint_to_raise(&self) -> pinocchio::pubkey::Pubkey {
        pinocchio::pubkey::Pubkey::from(self.mint_to_raise)
    }

    pub fn set_mint_to_raise(&mut self, mint_to_raise: &pinocchio::pubkey::Pubkey) {
        self.mint_to_raise.copy_from_slice(mint_to_raise.as_ref());
    }

    pub fn amount_to_raise(&self) -> u64 {
        u64::from_le_bytes(self.amount_to_raise)
    }

    pub fn set_amount_to_raise(&mut self, amount_to_raise: &u64) {
        self.amount_to_raise=amount_to_raise.to_le_bytes();
    }

    pub fn current_amount(&self) -> u64 {
        u64::from_le_bytes(self.current_amount)
    }

    pub fn set_current_amount(&mut self, current_amount: &u64) {
        self.current_amount=current_amount.to_le_bytes();
    }
    pub fn time_started(&self) -> u64 {
        u64::from_le_bytes(self.time_started)
    }

    pub fn set_time_started(&mut self, time_started: &u64) {
        self.time_started=time_started.to_le_bytes();
    }
    pub fn duration(&self) -> u8 {
        u8::from_le_bytes(self.duration)
    }

    pub fn set_duration(&mut self, duration: u8) {
        self.duration = duration.to_le_bytes();
    }

}