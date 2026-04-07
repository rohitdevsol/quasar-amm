use quasar_lang::prelude::*;
use quasar_spl::{ AssociatedTokenProgram, Mint, Token };

use crate::state::Config;

#[derive(Accounts)]
pub struct Initialize<'info> {
    // The person/admin who is going to make the pool
    // mut -> because it will pay while creating accounts
    pub maker: &'info mut Signer,

    // First mint for token X
    pub mint_x: &'info Account<Mint>,

    // Second mint for token Y
    pub mint_y: &'info Account<Mint>,

    // vault_x will hold all deposited token X (This is an ATA)
    // owner is config pda
    // Type is Token because .. AssociatedToken is removed from quasar-spl ( maintainer verified )
    #[account(
        init,
        mut,
        payer = maker,
        associated_token::mint = mint_x,
        associated_token::authority = config 
    )]
    pub vault_x: &'info Account<Token>,

    // Similar to comment on vault_x ( This one is for Y )
    #[account(
        init,
        mut,
        payer = maker,
        associated_token::mint = mint_y,
        associated_token::authority = config 
    )]
    pub vault_y: &'info Account<Token>,

    #[account(init, payer = maker, seeds = [b"config", maker], bump)]
    pub config: &'info mut Account<Config>,

    // liquidity provider token mint
    // users receive this token as reciepents when providing liquidity
    #[account(
        init,
        payer = maker,
        seeds = [b"lp", config],
        bump,
        mint::decimals = 6,
        mint::authority = config,
        mint::freeze_authority = config
    )]
    pub mint_lp: &'info mut Account<Mint>,

    // Program for Token creations
    pub token_program: &'info Program<Token>,

    // Program for account creations
    pub system_program: &'info Program<System>,

    // Program for ATA related operations
    pub associated_token_program: &'info Program<AssociatedTokenProgram>,
}

impl<'info> Initialize<'info> {
    #[inline(always)]
    pub fn initialize(
        &mut self,
        fee: u16,
        seed: u64,
        bumps: &InitializeBumps
    ) -> Result<(), ProgramError> {
        self.config.set_inner(
            seed, // part of pda seeds
            None, // no one can update the pool .. TODO
            *self.mint_x.address(), // validate vaults belong to the correct mint
            *self.mint_y.address(),
            fee, // for every swap
            false, // every instruction will check it
            bumps.config, // to sign as pda
            bumps.mint_lp // to sign as pda for mint_lp
        );
        Ok(())
    }
}
