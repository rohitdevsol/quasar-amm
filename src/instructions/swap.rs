use constant_product_curve::{ ConstantProduct, LiquidityPair };
use quasar_lang::prelude::*;
use quasar_spl::{ AssociatedTokenProgram, Mint, Token, TokenCpi };

use crate::{ errors::AmmError, instructions::WithdrawBumps, state::Config };

#[derive(Accounts)]
pub struct Swap<'info> {
    // user depositing the liquidity
    pub user: &'info mut Signer,

    // First mint for token X
    pub mint_x: &'info Account<Mint>,

    // Second mint for token Y
    pub mint_y: &'info Account<Mint>,

    // main config of amm
    #[account(
        //seeds = [b"config"], // , config.seed.get() -- facing issues here
        seeds = Config::seeds(config.seed),
        bump = config.config_bump,
        has_one = mint_x,
        has_one = mint_y
    )]
    pub config: &'info Account<Config>,

    // The LP provider token mint for this pool
    #[account(
        mut,
        seeds = Config::seeds(config),
        bump = config.lp_bump
    )]
    pub mint_lp: &'info Account<Mint>,

    // vault_x will hold all deposited token X (This is an ATA)
    // owner is config pda
    // Type is Token because .. AssociatedToken type is removed from quasar-spl ( maintainer verified ( _LOSTE ) )
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = config,
        associated_token::token_program = token_program
    )]
    pub vault_x: &'info Account<Token>,

    // Similar to comment on vault_x ( This one is for Y )
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = config ,
        associated_token::token_program = token_program
    )]
    pub vault_y: &'info Account<Token>,

    // Users token account for x token
    // mutable because we're transferring tokens from it
    #[account(
        mut , 
        associated_token::mint = mint_x,
        associated_token::authority = user,
        associated_token::token_program = token_program, // must for token 2022 in quasar.. not needed here though
    )]
    pub user_ata_x: &'info Account<Token>,

    // Users token account for Y token
    // mutable because we're transferring tokens from the it
    #[account(
        mut , 
        associated_token::mint = mint_y,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_ata_y: &'info Account<Token>,

    // Program for Token creations
    pub token_program: &'info Program<Token>,

    // Program for account creations
    pub system_program: &'info Program<System>,

    // Program for ATA related operations
    pub associated_token_program: &'info Program<AssociatedTokenProgram>,
}

impl<'info> Swap<'info> {
    pub fn swap(
        &mut self,
        is_x: bool,
        amount_in: u64,
        min_amount_out: u64,
        bumps: &SwapBumps
    ) -> Result<(), ProgramError> {
        require!(amount_in > 0, AmmError::InvalidAmount);
        require!(self.config.locked == false, AmmError::PoolLocked);

        // get the current pool status using the curve

        let mut curve = ConstantProduct::init(
            self.vault_x.amount(),
            self.vault_y.amount(),
            self.mint_lp.supply(),
            self.config.fee_bps.get(),
            None
        ).map_err(AmmError::from)?;

        // swap direction
        let p = match is_x {
            true => LiquidityPair::X,
            false => LiquidityPair::Y,
        };

        // calculate the swap amounts
        let swap_result = curve.swap(p, amount_in, min_amount_out).map_err(AmmError::from)?;

        // validate that the amounts are valid
        require!(swap_result.deposit != 0, AmmError::InvalidAmount);
        require!(swap_result.withdraw != 0, AmmError::InvalidAmount);

        self.deposit_token(is_x, swap_result.deposit)?;
        self.withdraw_token(!is_x, swap_result.withdraw, bumps)?;

        Ok(())
    }

    fn deposit_token(&mut self, is_x: bool, amount: u64) -> Result<(), ProgramError> {
        let (from, to, mint, decimals) = match is_x {
            true => (self.user_ata_x, self.vault_x, self.mint_x, self.mint_x.decimals()),
            false => (self.user_ata_y, self.vault_y, self.mint_y, self.mint_y.decimals()),
        };

        self.token_program.transfer_checked(from, mint, to, self.user, amount, decimals).invoke()
    }

    fn withdraw_token(
        &mut self,
        is_x: bool,
        amount: u64,
        bumps: &SwapBumps
    ) -> Result<(), ProgramError> {
        let (from, to, mint, decimals) = match is_x {
            true => (self.vault_x, self.user_ata_x, self.mint_x, self.mint_x.decimals()),
            false => (self.vault_y, self.user_ata_y, self.mint_y, self.mint_y.decimals()),
        };

        self.token_program
            .transfer_checked(from, mint, to, self.config, amount, decimals)
            .invoke_signed(&self.config_seeds(bumps))
    }
}
