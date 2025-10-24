use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Contributor {

     amount: [u8; 8],
}

impl Contributor {
    pub const LEN: usize = 8;


    pub fn from_account_info(account_info: &AccountInfo) -> Result<&mut Self, ProgramError> {
        let mut data: pinocchio::account_info::RefMut<'_, [u8]> = account_info.try_borrow_mut_data()?;
        if data.len() != Contributor::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        if (data.as_ptr() as usize) % core::mem::align_of::<Self>() != 0 {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(unsafe { &mut *(data.as_mut_ptr() as *mut Self) })
    }

    pub fn amount(&self) -> u64 {
        u64::from_le_bytes(self.amount)
    }

    pub fn set_amount(&mut self, amount: &u64) {
        self.amount=amount.to_le_bytes();
    }

}