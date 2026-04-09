use constant_product_curve::ConstantProduct;
use quasar_lang::prelude::*;
use quasar_spl::{ AssociatedTokenProgram, Mint, Token, TokenCpi };

use crate::{ errors::AmmError, state::Config };

#[derive(Accounts)]
pub struct Deposit<'info> {
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

impl<'info> Deposit<'info> {
    pub fn deposit(
        &mut self,
        amount: u64, // amount of LP tokens to mint to the user
        max_x: u64, // max amount of token x user is willing to deposit
        max_y: u64, // max amount of token y user is willing to deposit
        bump: &DepositBumps
    ) -> Result<(), ProgramError> {
        // ensure that the pool is not locked
        require!(self.config.locked == false, AmmError::PoolLocked);

        // user should ask for more than 0 lp tokens to mint
        require!(amount != 0, AmmError::InvalidAmount);

        // check if this is the first deposit
        let (x, y) = match
            self.mint_lp.supply() == 0 &&
            self.vault_x.amount() == 0 &&
            self.vault_y.amount() == 0
        {
            // first deposit in the pool... just push it
            true => (max_x, max_y),

            // maintaing the curve for every deposit after the first one
            false => {
                let amounts = ConstantProduct::xy_deposit_amounts_from_l(
                    self.vault_x.amount(),
                    self.vault_y.amount(),
                    self.mint_lp.supply(),
                    amount,
                    6
                ).unwrap();

                (amounts.x, amounts.y)
            }
        };

        require!(x <= max_x && y <= max_y, AmmError::SlippageExceeded);

        // deposit token from user to vault (X)

        self.deposit_tokens(true, x)?;
        // deposit token from user to vault (X)
        self.deposit_tokens(false, y)?;
        // Mint LP tokens to user as proof of liquidity provision
        self.mint_lp_tokens(amount, bump)?;

        Ok(())
    }

    pub fn deposit_tokens(&self, is_x: bool, amount: u64) -> Result<(), ProgramError> {
        let (
            from, // User's token account
            to, // Vault token account
            mint, // Token mint
            decimals, // Token decimal places
        ) = match is_x {
            true => (self.user_ata_x, self.vault_x, self.mint_x, self.mint_x.decimals()),
            false => (self.user_ata_y, self.vault_y, self.mint_y, self.mint_y.decimals()),
        };

        self.token_program.transfer_checked(from, mint, to, self.user, amount, decimals).invoke()
    }

    pub fn mint_lp_tokens(&self, amount: u64, bump: &DepositBumps) -> Result<(), ProgramError> {
        self.token_program
            .mint_to(self.mint_lp, self.user_ata_lp, self.config, amount)
            .invoke_signed(&self.config_seeds(bump))

        // bump.config_seeds() -> OLD API
        // new changes about defining and accessing seeds were pushed on April 8 2026
    }
}
