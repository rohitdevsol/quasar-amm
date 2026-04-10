use quasar_lang::prelude::*;
use constant_product_curve::CurveError;

#[error_code]
pub enum AmmError {
    // Default Error
    DefaultError,

    // Offer Expired
    OfferExpired,

    // Pool locked
    PoolLocked,

    //Slippage exceeded
    SlippageExceeded,

    //Overflow detecte
    Overflow,

    //Underflow detecte
    Underflow,

    //Invalid token
    InvalidToken,

    //Actual liquidity is less than minimum
    LiquidityLessThanMinimum,

    //No liquidity in pool
    NoLiquidityInPool,

    //Bump error
    BumpError,

    //Curve error
    CurveError,

    //Fee is greater than 100%. This is not a very good deal
    InvalidFee,

    //Invalid update authority
    InvalidAuthority,

    //No update authority set
    NoAuthoritySet,

    //Invalid amount
    InvalidAmount,

    //Invalid precision
    InvalidPrecision,

    //Insufficient balance
    InsufficientBalance,

    //Zero balance
    ZeroBalance,
}

// maps constant_product_curve errors to our custom AMM error codes
impl From<CurveError> for AmmError {
    fn from(error: CurveError) -> AmmError {
        match error {
            CurveError::InvalidPrecision => AmmError::InvalidPrecision,
            CurveError::Overflow => AmmError::Overflow,
            CurveError::Underflow => AmmError::Underflow,
            CurveError::InvalidFeeAmount => AmmError::InvalidFee,
            CurveError::InsufficientBalance => AmmError::InsufficientBalance,
            CurveError::ZeroBalance => AmmError::ZeroBalance,
            CurveError::SlippageLimitExceeded => AmmError::SlippageExceeded,
        }
    }
}
