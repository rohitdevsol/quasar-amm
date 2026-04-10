use quasar_lang::prelude::*;
use quasar_spl::{ AssociatedTokenProgram, Mint, Token };

use crate::{ events::PoolInitialized, state::Config };

#[derive(Accounts)]
#[instruction(seed:u64)]
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
    // Type is Token because .. AssociatedToken type is removed from quasar-spl ( maintainer verified ( _LOSTE ) )
    #[account(
        init,
        mut,
        payer = maker,
        associated_token::mint = mint_x,
        associated_token::authority = config ,
        associated_token::token_program = token_program, // must for token 2022 in quasar.. not needed here though
    )]
    pub vault_x: &'info Account<Token>,

    // Similar to comment on vault_x ( This one is for Y )
    #[account(
        init,
        mut,
        payer = maker,
        associated_token::mint = mint_y,
        associated_token::authority = config,
        associated_token::token_program = token_program, // must for token 2022 in quasar.. not needed here though
    )]
    pub vault_y: &'info Account<Token>,

    // main config PDA for this AMM pool, seeded by the provided seed value
    #[account(init, payer = maker, seeds = Config::seeds(seed), bump)]
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
    /// Sets up the pool config account with all required parameters
    /// * `fee` - swap fee in basis points (e.g. 30 = 0.3%)
    /// * `seed` - unique seed to derive the config PDA
    /// * `bumps` - PDA bump seeds from account validation
    #[inline(always)]
    pub fn initialize(
        &mut self,
        fee: u16,
        seed: u64,
        bumps: &InitializeBumps
    ) -> Result<(), ProgramError> {
        // Initialize the config account
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

    /// Emits a PoolInitialized event for indexers and frontends
    #[inline(always)]
    pub fn emit_event(&self) -> Result<(), ProgramError> {
        emit!(PoolInitialized {
            admin: *self.maker.address(),
            config: *self.config.address(),
            mint_x: *self.mint_x.address(),
            mint_y: *self.mint_y.address(),
        });
        Ok(())
    }
}
