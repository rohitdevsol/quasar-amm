use constant_product_curve::ConstantProduct;
use quasar_lang::prelude::*;
use quasar_spl::{ AssociatedTokenProgram, Mint, Token, TokenCpi };

use crate::{ errors::AmmError, events::LiquidityRemoved, state::Config };

#[derive(Accounts)]
pub struct Withdraw<'info> {
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

    // User's token account for LP tokens
    // Will be created if it doesn't exist, user pays for creation
    // Mutable because we're minting LP tokens to it
    #[account(
        init_if_needed,
        mut,
        payer = user,
        associated_token::mint = mint_lp,
        associated_token::authority = user,
        associated_token::token_program = token_program
    )]
    pub user_ata_lp: &'info Account<Token>,

    // Program for Token creations
    pub token_program: &'info Program<Token>,

    // Program for account creations
    pub system_program: &'info Program<System>,

    // Program for ATA related operations
    pub associated_token_program: &'info Program<AssociatedTokenProgram>,
}

impl<'info> Withdraw<'info> {
    /// * `amount` - Amount of LP tokens to burn
    /// * `min_x` - Minimum amount of token X user expects to receive
    /// * `min_y` - Minimum amount of token Y user expects to receive
    #[inline(always)]
    pub fn withdraw(
        &mut self,
        amount: u64,
        min_x: u64,
        min_y: u64,
        bumps: &WithdrawBumps
    ) -> Result<(), ProgramError> {
        require!(self.config.locked == false, AmmError::PoolLocked);

        // ensure that user is requesting to burn some LP tokens
        require!(amount != 0, AmmError::InvalidAmount);

        // calculate the token amounts to withdraw

        let (x, y) = match
            self.mint_lp.supply() == 0 &&
            self.vault_x.amount() == 0 &&
            self.vault_y.amount() == 0
        {
            // edge case : use minimum amount if pool is empty
            // shouldn't happen in normal operation
            true => (min_x, min_y),

            // normal case: calculate proportional amount based on LP token share
            false => {
                let amounts = ConstantProduct::xy_withdraw_amounts_from_l(
                    self.vault_x.amount(),
                    self.vault_y.amount(),
                    self.mint_lp.supply(),
                    amount,
                    6
                ).unwrap();
                (amounts.x, amounts.y)
            }
        };

        // Slippage protection: ensure calculated amounts meet user's minimum requirements
        require!(x >= min_x && y >= min_y, AmmError::SlippageExceeded);

        // Burn LP tokens from user's account first
        self.burn_lp_tokens(amount)?;

        // Transfer calculated amounts of both tokens to user
        self.withdraw_tokens(x, true, bumps)?; // Transfer token X
        self.withdraw_tokens(y, false, bumps)?; // Transfer token Y
        Ok(())
    }

    #[inline(always)]
    pub fn burn_lp_tokens(&mut self, amount: u64) -> Result<(), ProgramError> {
        /// burn lp tokens from the user lp account
        self.token_program.burn(self.user_ata_lp, self.mint_lp, self.user, amount).invoke()?;
        Ok(())
    }

    #[inline(always)]
    pub fn withdraw_tokens(
        &mut self,
        amount: u64,
        is_x: bool,
        bumps: &WithdrawBumps
    ) -> Result<(), ProgramError> {
        let (
            to, // Vault token account
            from, // User's token account
            mint, // Token mint
            decimals, // Token decimal places
        ) = match is_x {
            true => (self.user_ata_x, self.vault_x, self.mint_x, self.mint_x.decimals()),
            false => (self.user_ata_y, self.vault_y, self.mint_y, self.mint_y.decimals()),
        };

        /// Transfers tokens from vault to user's account
        self.token_program
            .transfer_checked(from, mint, to, self.config, amount, decimals)
            .invoke_signed(&self.config_seeds(bumps))
    }

    #[inline(always)]
    pub fn emit_event(&self) -> Result<(), ProgramError> {
        emit!(LiquidityRemoved {
            config: *self.config.address(),
            user: *self.user.address(),
        });
        Ok(())
    }
}
