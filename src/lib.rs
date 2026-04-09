#![cfg_attr(not(test), no_std)]

use quasar_lang::prelude::*;
mod instructions;
mod errors;
mod state;
mod events;
use instructions::*;
declare_id!("BrVtjP4pUzeirhX744pzT1A7i7ZisbeMeu5NdE9aPQPG");

#[program]
mod quasar_amm {
    use super::*;

    #[instruction(discriminator = 0)]
    pub fn initialize(ctx: Ctx<Initialize>, fee: u16, seed: u64) -> Result<(), ProgramError> {
        ctx.accounts.initialize(fee, seed, &ctx.bumps)?;
        ctx.accounts.emit_event()
    }

    #[instruction(discriminator = 1)]
    pub fn deposit(
        ctx: Ctx<Deposit>,
        amount: u64,
        max_x: u64,
        max_y: u64
    ) -> Result<(), ProgramError> {
        ctx.accounts.deposit(amount, max_x, max_y, &ctx.bumps)
    }

    #[instruction(discriminator = 2)]
    pub fn withdraw(
        ctx: Ctx<Withdraw>,
        amount: u64,
        min_x: u64,
        min_y: u64
    ) -> Result<(), ProgramError> {
        ctx.accounts.withdraw(amount, min_x, min_y, &ctx.bumps)
    }

    #[instruction(discriminator = 3)]
    pub fn swap(
        ctx: Ctx<Swap>,
        is_x: bool,
        amount_in: u64,
        min_amount_out: u64
    ) -> Result<(), ProgramError> {
        ctx.accounts.swap(is_x, amount_in, min_amount_out, &ctx.bumps)
    }

    #[instruction(discriminator = 4)]
    pub fn toggle_pool(ctx: Ctx<TogglePool>) -> Result<(), ProgramError> {
        ctx.accounts.toggle_pool()
    }
}

#[cfg(test)]
mod tests;
