use quasar_lang::prelude::*;

// Solana accounts only store the CURRENT state — they have no history.
// Once a swap happens and balances update, the old balances are gone forever.
// Events are the only way to log "what happened" permanently on-chain,
// so frontends and indexers can show trade history, price charts, and user activity
// without having to watch every transaction manually.

// Emitted when a new pool is created
#[event(discriminator = 0)]
pub struct PoolInitialized {
    pub config: Address, // which pool (use this to filter all events for a pool)
    pub admin: Address, // who created it
    pub mint_x: Address,
    pub mint_y: Address,
}

// Emitted when someone adds liquidity
#[event(discriminator = 1)]
pub struct LiquidityAdded {
    pub config: Address, // which pool
    pub user: Address, // who deposited
}

// Emitted when someone removes liquidity
#[event(discriminator = 2)]
pub struct LiquidityRemoved {
    pub config: Address,
    pub user: Address,
}

// Emitted on every swap
#[event(discriminator = 3)]
pub struct Swapped {
    pub config: Address,
    pub user: Address,
}

// Emitted when pool is locked or unlocked
#[event(discriminator = 4)]
pub struct PoolLockToggled {
    pub config: Address,
    pub locked: bool, // true = just locked, false = just unlocked
    pub authority: Address, // who toggled it
}
