use anchor_lang::prelude::*;

pub mod instructions;
pub mod state;

use crate::instructions::*;
declare_id!("8cCWwrHTFe5V8xDxXZEkrfUquoq3bWUhukEyjPexbJ7y");

#[program]
pub mod escrow {
    use super::*;
    
   
    pub fn make(ctx: Context<Make>, seed: u64, receive_amount: u64, deposit_amount: u64,) -> Result<()> {
        ctx.accounts.init_escrow(seed, receive_amount, &ctx.bumps)?;
        ctx.accounts.deposit(deposit_amount)?;
         Ok(())
     }
 
     pub fn take(ctx: Context<Take>) -> Result<()> {
         ctx.accounts.transfer_to_maker()?;
         ctx.accounts.transfer_to_taker()?;
         ctx.accounts.close_vault()?;
         Ok(())
     }
 
     pub fn refund(ctx: Context<Refund>) -> Result<()> {
         ctx.accounts.refund()?;
         ctx.accounts.close()?;
         Ok(())
     }

     pub fn open_dispute(ctx: Context<OpenDispute>, text: String) -> Result<()> {
         ctx.accounts.open(text)
     }

     pub fn resolve_dispute(ctx: Context<ResolveDisputeCtx>, response: String) -> Result<()> {
         ctx.accounts.resolve(response)
     }

     // mock-only resolver for local tests
     pub fn resolve_dispute_mock(ctx: Context<ResolveDisputeMock>, response: String) -> Result<()> {
         ctx.accounts.resolve_mock(response)
     }
  }
