pub mod deposit;     // handles liquidity deposits and LP token minting
pub mod initialize;  // handles pool creation and config setup
pub mod withdraw;    // handles liquidity withdrawal and LP token burning
pub mod swap;        // handles token swaps through the constant product curve

// re-export all instruction types for use in lib.rs
pub use deposit::*;
pub use initialize::*;
pub use withdraw::*;
pub use swap::*;
