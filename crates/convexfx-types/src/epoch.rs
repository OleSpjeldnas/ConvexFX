use serde::{Deserialize, Serialize};

/// Epoch identifier (sequential counter)
pub type EpochId = u64;

/// Epoch state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpochState {
    /// Collecting commitments
    Collect,
    /// Revealing orders (optional phase)
    Reveal,
    /// Solving the optimization problem
    Solving,
    /// Solution published
    Published,
    /// Settlement in progress
    Settling,
    /// Epoch completed
    Completed,
}

impl EpochState {
}


