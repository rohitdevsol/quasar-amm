#![cfg_attr(not(test), no_std)]

use quasar_lang::prelude::*;
mod instructions;
mod state;
use instructions::*;
declare_id!("BrVtjP4pUzeirhX744pzT1A7i7ZisbeMeu5NdE9aPQPG");

// #[derive(Accounts)]
// pub struct Initialize<'info> {
//     pub payer: &'info mut Signer,
//     pub system_program: &'info Program<System>,
// }

// impl<'info> Initialize<'info> {
//     #[inline(always)]
//     pub fn initialize(&self) -> Result<(), ProgramError> {
//         Ok(())
//     }
// }

#[program]
mod quasar_amm {
    use super::*;

    #[instruction(discriminator = 0)]
    pub fn initialize(ctx: Ctx<Initialize>, fee: u16, seed: u64) -> Result<(), ProgramError> {
        ctx.accounts.initialize(fee, seed, &ctx.bumps)
    }
}

#[cfg(test)]
mod tests;
