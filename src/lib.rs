#![cfg_attr(not(test), no_std)]

use quasar_lang::prelude::*;

// core modules
mod instructions; // all instruction handlers (initialize, deposit, withdraw, swap, toggle)
mod errors;       // custom AMM error codes
mod state;        // on-chain account state definitions
mod events;       // event structs emitted for indexers

use instructions::*;

// program ID deployed on-chain
declare_id!("BrVtjP4pUzeirhX744pzT1A7i7ZisbeMeu5NdE9aPQPG");

#[program]
mod quasar_amm {
    use super::*;

    // creates a new AMM pool with the given fee and seed
    #[instruction(discriminator = 0)]
    pub fn initialize(ctx: Ctx<Initialize>, fee: u16, seed: u64) -> Result<(), ProgramError> {
        ctx.accounts.initialize(fee, seed, &ctx.bumps)?;
        ctx.accounts.emit_event()
    }

    // adds liquidity to the pool and mints LP tokens to the user
    #[instruction(discriminator = 1)]
    pub fn deposit(
        ctx: Ctx<Deposit>,
        amount: u64,
        max_x: u64,
        max_y: u64
    ) -> Result<(), ProgramError> {
        ctx.accounts.deposit(amount, max_x, max_y, &ctx.bumps)?;
        ctx.accounts.emit_event()
    }

    // removes liquidity by burning LP tokens and returning both tokens
    #[instruction(discriminator = 2)]
    pub fn withdraw(
        ctx: Ctx<Withdraw>,
        amount: u64,
        min_x: u64,
        min_y: u64
    ) -> Result<(), ProgramError> {
        ctx.accounts.withdraw(amount, min_x, min_y, &ctx.bumps)?;
        ctx.accounts.emit_event()
    }

    // swaps one token for another through the constant product curve
    #[instruction(discriminator = 3)]
    pub fn swap(
        ctx: Ctx<Swap>,
        is_x: bool,
        amount_in: u64,
        min_amount_out: u64
    ) -> Result<(), ProgramError> {
        ctx.accounts.swap(is_x, amount_in, min_amount_out, &ctx.bumps)?;
        ctx.accounts.emit_event()
    }

}

#[cfg(test)]
mod tests;
