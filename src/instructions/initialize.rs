use quasar_lang::prelude::*;
use quasar_spl::{ AssociatedTokenProgram, Mint, Token, TokenAccountState };

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
    // Type is Token because .. AssociatedToken is removed from quasar-spl ( maintainer verified )
    #[account(
        init,
        mut,
        payer = maker,
        associated_token::mint = mint_x,
        associated_token::authority = maker //TODO
    )]
    pub vault_x: &'info Account<Token>,

    // Similar to comment on vault_x ( This one is for Y )
    #[account(
        init,
        mut,
        payer = maker,
        associated_token::mint = mint_y,
        associated_token::authority = maker //TODO
    )]
    pub vault_y: &'info Account<Token>,

    // Program for Token creations
    pub token_program: &'info Program<Token>,

    // Program for account creations
    pub system_program: &'info Program<System>,

    // Program for ATA related operations
    pub associated_token_program: &'info Program<AssociatedTokenProgram>,
}

impl<'info> Initialize<'info> {
    #[inline(always)]
    pub fn initialize(&mut self) -> Result<(), ProgramError> {
        Ok(())
    }
}
