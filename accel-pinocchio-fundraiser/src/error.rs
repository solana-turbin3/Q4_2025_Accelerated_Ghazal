use pinocchio::program_error::ProgramError;
#[derive(Debug, Clone, PartialEq)]
pub enum FundRaiserError {
    
    TargetNotMet,
    TargetMet,
    ContributionTooBig,
    ContributionTooSmall,
    MaximumContributionsReached,
    FundraiserNotEnded,
    FundraiserEnded,
    InvalidAmount
    }

impl From<FundRaiserError> for ProgramError {
    fn from(error: FundRaiserError) -> Self {
        match error {
        
            FundRaiserError::TargetNotMet => ProgramError::Custom(1000),
            FundRaiserError::TargetMet => ProgramError::Custom(1001),
             FundRaiserError::ContributionTooBig => ProgramError::Custom(1002),
            FundRaiserError::ContributionTooSmall => ProgramError::Custom(1003),
             FundRaiserError::MaximumContributionsReached => ProgramError::Custom(1004),
            FundRaiserError::FundraiserNotEnded => ProgramError::Custom(1005),
             FundRaiserError::FundraiserEnded => ProgramError::Custom(1006),
            FundRaiserError::InvalidAmount => ProgramError::Custom(1007),
            
             }
    }
}

impl FundRaiserError {
    
    pub fn message(&self) -> &'static str {
        match self {
            
            FundRaiserError::TargetNotMet => "The amount to raise has not been met",
            FundRaiserError::TargetMet => "The amount to raise has been achieved",
            FundRaiserError::ContributionTooBig => "The contribution is too big",
            FundRaiserError::ContributionTooSmall => "The contribution is too small",
            FundRaiserError::MaximumContributionsReached => "The maximum amount to contribute has been reached",
            FundRaiserError::FundraiserNotEnded => "The fundraiser has not ended yet",
            FundRaiserError::FundraiserEnded => "The fundraiser has ended",
            FundRaiserError::InvalidAmount => "Invalid total amount. i should be bigger than 3",
            

             }
    }
}