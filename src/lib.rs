#![cfg_attr(not(test), no_std)]
#![allow(dead_code, unused)] // REMOVE AT LAST

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
}

#[cfg(test)]
mod tests;
