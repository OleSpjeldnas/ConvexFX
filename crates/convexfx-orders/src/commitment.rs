use convexfx_types::{ConvexFxError, PairOrder, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Commitment hash (32-byte Blake2b or SHA256)
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CommitmentHash(pub String);

impl CommitmentHash {
    /// Parse from hex string
    pub fn from_hex(hex: &str) -> Result<Self> {
        // Validate hex format
        if hex.len() != 64 {
            return Err(ConvexFxError::InvalidCommitment(
                "commitment hash must be 64 hex characters".to_string(),
            ));
        }
        hex::decode(hex).map_err(|_| {
            ConvexFxError::InvalidCommitment("invalid hex encoding".to_string())
        })?;
        Ok(CommitmentHash(hex.to_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CommitmentHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Commitment containing hash and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commitment {
    pub hash: CommitmentHash,
    pub epoch_id: u64,
    pub timestamp_ms: u64,
}

/// Compute commitment hash: H(order_json || salt)
pub fn compute_commitment(order: &PairOrder, salt: &[u8]) -> Result<CommitmentHash> {
    let order_json = serde_json::to_string(order).map_err(|e| {
        ConvexFxError::SerializationError(format!("failed to serialize order: {}", e))
    })?;

    let mut hasher = Sha256::new();
    hasher.update(order_json.as_bytes());
    hasher.update(salt);
    let hash_bytes = hasher.finalize();
    let hash_hex = hex::encode(hash_bytes);

    Ok(CommitmentHash(hash_hex))
}

/// Verify a commitment against order and salt
pub fn verify_commitment(
    commitment: &CommitmentHash,
    order: &PairOrder,
    salt: &[u8],
) -> Result<bool> {
    let computed = compute_commitment(order, salt)?;
    Ok(computed == *commitment)
}

#[cfg(test)]
mod tests {
    use super::*;
    use convexfx_types::{AccountId, Amount, AssetId};

    #[test]
    fn test_commitment_computation() {
        let order = PairOrder {
            id: "test".to_string(),
            trader: AccountId::new("trader1"),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::from_units(1000),
            limit_ratio: None,
            min_fill_fraction: None,
            metadata: serde_json::json!({}),
        };

        let salt = b"random_salt_12345";
        let commitment = compute_commitment(&order, salt).unwrap();

        assert_eq!(commitment.0.len(), 64); // SHA256 hex
        assert!(verify_commitment(&commitment, &order, salt).unwrap());

        // Different salt should produce different commitment
        let commitment2 = compute_commitment(&order, b"different_salt").unwrap();
        assert_ne!(commitment, commitment2);
    }

    #[test]
    fn test_commitment_verification_fails() {
        let order = PairOrder {
            id: "test".to_string(),
            trader: AccountId::new("trader1"),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::from_units(1000),
            limit_ratio: None,
            min_fill_fraction: None,
            metadata: serde_json::json!({}),
        };

        let salt = b"salt1";
        let commitment = compute_commitment(&order, salt).unwrap();

        // Wrong salt
        assert!(!verify_commitment(&commitment, &order, b"wrong_salt").unwrap());

        // Modified order
        let mut modified_order = order.clone();
        modified_order.budget = Amount::from_units(2000);
        assert!(!verify_commitment(&commitment, &modified_order, salt).unwrap());
    }
}


