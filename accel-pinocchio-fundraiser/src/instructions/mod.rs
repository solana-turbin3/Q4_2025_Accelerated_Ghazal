pub mod initialize;
pub mod contribute;
pub mod checker;
pub mod refund;
// pub mod make_2;

pub use initialize::*;
pub use contribute::*;
pub use checker::*;
pub use refund::*;
// pub use make_2::*;

pub enum FundRaiserInstrctions {
    Initialize = 0,
    Contribute = 1,
    Checker = 2,
    Refund = 3,
}

impl TryFrom<&u8> for FundRaiserInstrctions {
    type Error = pinocchio::program_error::ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(FundRaiserInstrctions::Initialize),
            1 => Ok(FundRaiserInstrctions::Contribute),
            2 => Ok(FundRaiserInstrctions::Checker),
            3 => Ok(FundRaiserInstrctions::Refund),
            _ => Err(pinocchio::program_error::ProgramError::InvalidInstructionData),
        }
    }
}