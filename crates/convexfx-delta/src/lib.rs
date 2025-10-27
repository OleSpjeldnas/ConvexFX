use convexfx_types::{AccountId, AssetId, Amount, PairOrder};
use convexfx_exchange;
use delta_base_sdk::{
    vaults::{OwnerId, VaultId},
    crypto::{HashDigest},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// Re-export Delta SDK types for compatibility
pub use delta_verifiable::types::{VerifiableType, VerifiableWithDiffs};

/// Delta state management and vault operations
pub mod state;
/// SDL (State Diff List) generation from ConvexFX results
pub mod sdl_generator;
/// Full Delta executor implementation using ConvexFX
pub mod executor;
/// Demo application for local vault management and signed message processing
pub mod demo_app;

pub use state::*;
pub use sdl_generator::*;
pub use executor::*;
pub use demo_app::*;

/// Error types for Delta integration
#[derive(Debug, thiserror::Error)]
pub enum DeltaIntegrationError {
    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    #[error("Asset not found: {0}")]
    AssetNotFound(String),

    #[error("Insufficient vault balance")]
    InsufficientBalance,

    #[error("Delta SDK error: {0}")]
    DeltaSdk(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("ConvexFX error: {0}")]
    ConvexFx(String),

    #[error("Exchange error: {0}")]
    Exchange(#[from] convexfx_exchange::ExchangeError),

    #[error("Signature error: {0}")]
    Signature(String),
    
    #[error("Clearing failed: {0}")]
    ClearingFailed(String),
}

/// Result type for Delta integration operations
pub type Result<T> = std::result::Result<T, DeltaIntegrationError>;
