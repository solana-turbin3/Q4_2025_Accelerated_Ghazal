use anchor_lang::prelude::*;
use anchor_lang::prelude::ProgramError;
use anchor_lang::Discriminator;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface, TransferChecked, close_account},
};

use crate::state::Escrow;

#[derive(Accounts)]
pub struct OpenDispute<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    // Escrow state and vault (funds are locked here)
    #[account(
        mut,
        has_one = maker,
        has_one = mint_a,
        has_one = mint_b,
        seeds = [b"escrow", maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump,
    )]
    pub escrow: Account<'info, Escrow>,

    /// Maker/taker for reference in callback
    pub maker: SystemAccount<'info>,
    pub taker: SystemAccount<'info>,

    pub mint_a: InterfaceAccount<'info, Mint>,
    pub mint_b: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    // Oracle CPI accounts
    /// CHECK: PDA owned/validated inside oracle program
    #[account(mut)]
    pub interaction: AccountInfo<'info>,
    pub context_account: Account<'info, solana_gpt_oracle::ContextAccount>,
    /// CHECK: checked by address
    #[account(address = solana_gpt_oracle::ID)]
    pub oracle_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,

    // Token programs (forwarded to callback metas)
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> OpenDispute<'info> {
    pub fn open(&self, text: String) -> Result<()> {
        // Provide callback discriminator and account metas required to resolve
        let disc: [u8; 8] = crate::instruction::ResolveDispute::DISCRIMINATOR
            .try_into()
            .expect("discriminator must be 8 bytes");

        let metas: Vec<solana_gpt_oracle::AccountMeta> = vec![
            // maker/taker must be mutable (rent from close_account goes here)
            solana_gpt_oracle::AccountMeta { pubkey: self.maker.key(), is_signer: false, is_writable: true },
            solana_gpt_oracle::AccountMeta { pubkey: self.taker.key(), is_signer: false, is_writable: true },
            solana_gpt_oracle::AccountMeta { pubkey: self.mint_a.key(), is_signer: false, is_writable: false },
            solana_gpt_oracle::AccountMeta { pubkey: self.mint_b.key(), is_signer: false, is_writable: false },
            solana_gpt_oracle::AccountMeta { pubkey: self.escrow.key(), is_signer: false, is_writable: true },
            solana_gpt_oracle::AccountMeta { pubkey: self.vault.key(), is_signer: false, is_writable: true },
            // Destination ATAs for resolution
            // Maker A ATA
            solana_gpt_oracle::AccountMeta { pubkey: anchor_spl::associated_token::get_associated_token_address(&self.maker.key(), &self.mint_a.key()), is_signer: false, is_writable: true },
            // Taker A ATA
            solana_gpt_oracle::AccountMeta { pubkey: anchor_spl::associated_token::get_associated_token_address(&self.taker.key(), &self.mint_a.key()), is_signer: false, is_writable: true },
            // Order must match ResolveDisputeCtx: system_program, associated_token_program, token_program
            solana_gpt_oracle::AccountMeta { pubkey: self.system_program.key(), is_signer: false, is_writable: false },
            solana_gpt_oracle::AccountMeta { pubkey: self.associated_token_program.key(), is_signer: false, is_writable: false },
            solana_gpt_oracle::AccountMeta { pubkey: self.token_program.key(), is_signer: false, is_writable: false },
        ];

        let cpi_program = self.oracle_program.to_account_info();
        let cpi_accounts = solana_gpt_oracle::cpi::accounts::InteractWithLlm {
            payer: self.payer.to_account_info(),
            interaction: self.interaction.clone(),
            context_account: self.context_account.to_account_info(),
            system_program: self.system_program.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        solana_gpt_oracle::cpi::interact_with_llm(
            cpi_ctx,
            text,
            crate::ID,
            disc,
            Some(metas),
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct ResolveDisputeCtx<'info> {
    // Must be present and is_signer in callback
    pub identity: Account<'info, solana_gpt_oracle::Identity>,

    /// Parties and mint
    #[account(mut)]
    pub maker: SystemAccount<'info>,
    #[account(mut)]
    pub taker: SystemAccount<'info>,
    pub mint_a: InterfaceAccount<'info, Mint>,
    pub mint_b: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        has_one = maker,
        has_one = mint_a,
        has_one = mint_b,
        seeds = [b"escrow", maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump,
    )]
    pub escrow: Account<'info, Escrow>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = maker,
    )]
    pub maker_mint_a_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = taker,
    )]
    pub taker_mint_a_ata: InterfaceAccount<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> ResolveDisputeCtx<'info> {
    pub fn resolve(&mut self, response: String) -> Result<()> {
        // Ensure the callback is authorized by the oracle program
        if !self.identity.to_account_info().is_signer {
            return Err(ProgramError::InvalidAccountData.into());
        }

        // Decide winner based on response string
        let to_taker = response.to_lowercase().contains("taker");

        let cpi_program = self.token_program.to_account_info();
        let (to_account, to_decimals) = if to_taker {
            (self.taker_mint_a_ata.to_account_info(), self.mint_a.decimals)
        } else {
            (self.maker_mint_a_ata.to_account_info(), self.mint_a.decimals)
        };

        let amount = self.vault.amount;
        let transfer_accounts = TransferChecked {
            from: self.vault.to_account_info(),
            mint: self.mint_a.to_account_info(),
            to: to_account,
            authority: self.escrow.to_account_info(),
        };
        let maker_key = self.maker.key();
        let seeds: &[&[&[u8]]] = &[&[
            b"escrow",
            maker_key.as_ref(),
            &self.escrow.seed.to_le_bytes(),
            &[self.escrow.bump],
        ]];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, transfer_accounts, seeds);
        transfer_checked(cpi_ctx, amount, to_decimals)?;

        // Close vault to the recipient (refund rent)
        let cpi_program = self.token_program.to_account_info();
        let dest = if to_taker {
            self.taker.to_account_info()
        } else {
            self.maker.to_account_info()
        };
        let close_accounts = CloseAccount {
            account: self.vault.to_account_info(),
            destination: dest,
            authority: self.escrow.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, close_accounts, seeds);
        close_account(cpi_ctx)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct ResolveDisputeMock<'info> {
    /// CHECK: mock – not enforcing signer in tests
    pub identity: Account<'info, solana_gpt_oracle::Identity>,

    #[account(mut)]
    pub maker: SystemAccount<'info>,
    #[account(mut)]
    pub taker: SystemAccount<'info>,
    pub mint_a: InterfaceAccount<'info, Mint>,
    pub mint_b: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        has_one = maker,
        has_one = mint_a,
        has_one = mint_b,
        seeds = [b"escrow", maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump,
    )]
    pub escrow: Account<'info, Escrow>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = maker,
    )]
    pub maker_mint_a_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = taker,
    )]
    pub taker_mint_a_ata: InterfaceAccount<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> ResolveDisputeMock<'info> {
    pub fn resolve_mock(&mut self, response: String) -> Result<()> {
        // No identity signature required (mock)
        let to_taker = response.to_lowercase().contains("taker");

        let cpi_program = self.token_program.to_account_info();
        let (to_account, to_decimals) = if to_taker {
            (self.taker_mint_a_ata.to_account_info(), self.mint_a.decimals)
        } else {
            (self.maker_mint_a_ata.to_account_info(), self.mint_a.decimals)
        };

        let amount = self.vault.amount;
        let transfer_accounts = TransferChecked {
            from: self.vault.to_account_info(),
            mint: self.mint_a.to_account_info(),
            to: to_account,
            authority: self.escrow.to_account_info(),
        };
        let maker_key = self.maker.key();
        let seeds: &[&[&[u8]]] = &[&[
            b"escrow",
            maker_key.as_ref(),
            &self.escrow.seed.to_le_bytes(),
            &[self.escrow.bump],
        ]];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, transfer_accounts, seeds);
        transfer_checked(cpi_ctx, amount, to_decimals)?;

        // Close vault to chosen recipient
        let cpi_program = self.token_program.to_account_info();
        let dest = if to_taker {
            self.taker.to_account_info()
        } else {
            self.maker.to_account_info()
        };
        let close_accounts = CloseAccount {
            account: self.vault.to_account_info(),
            destination: dest,
            authority: self.escrow.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, close_accounts, seeds);
        close_account(cpi_ctx)?;

        Ok(())
    }
}
