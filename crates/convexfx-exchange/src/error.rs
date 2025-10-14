use thiserror::Error;

pub type Result<T> = std::result::Result<T, ExchangeError>;

/// Errors that can occur during exchange operations
#[derive(Debug, Error)]
pub enum ExchangeError {
    #[error("Asset error: {0}")]
    Asset(#[from] convexfx_types::ConvexFxError),

    #[error("Ledger error: {0}")]
    Ledger(String),

    #[error("Oracle error: {0}")]
    Oracle(String),

    #[error("Order error: {0}")]
    Order(String),

    #[error("Clearing error: {0}")]
    Clearing(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("System error: {0}")]
    System(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Insufficient liquidity: {0}")]
    InsufficientLiquidity(String),

    #[error("Order validation failed: {0}")]
    OrderValidation(String),
}

