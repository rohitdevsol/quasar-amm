use quasar_lang::prelude::*;

#[account(discriminator = 1, set_inner)]
#[seeds(b"config", seed:u64)]
// NOTE: LP mint PDA uses raw seeds = [b"lp", config] in instruction files
// Quasar only supports one #[seeds] per struct, so the LP seeds cannot be defined here
pub struct Config {
    pub seed: u64, //  — part of PDA seeds, store it
    pub authority: Option<Address>, //  — who can lock/update the pool
    pub mint_x: Address, //  — validate vaults belong to correct mints
    pub mint_y: Address, //  — same
    pub fee_bps: u16, //  — every swap reads this
    pub locked: bool, //  — every instruction checks this
    pub config_bump: u8, //  — to sign as PDA without recomputing
    pub lp_bump: u8, //  — to sign as PDA for mint_lp
}
