use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConvexFxError {
    #[error("Insufficient balance: account={0}, asset={1}")]
    InsufficientBalance(String, String),

    #[error("Invalid amount: {0}")]
    InvalidAmount(String),

    #[error("Invalid order: {0}")]
    InvalidOrder(String),

    #[error("Asset not found: {0}")]
    AssetNotFound(String),

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Epoch not found: {0}")]
    EpochNotFound(u64),

    #[error("Order not found: {0}")]
    OrderNotFound(String),

    #[error("Invalid commitment: {0}")]
    InvalidCommitment(String),

    #[error("Solver error: {0}")]
    SolverError(String),

    #[error("Infeasible problem: {0}")]
    Infeasible(String),

    #[error("Convergence failed: {0}")]
    ConvergenceFailed(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, ConvexFxError>;


