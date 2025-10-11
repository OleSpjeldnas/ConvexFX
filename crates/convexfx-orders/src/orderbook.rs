use convexfx_types::{ConvexFxError, EpochId, OrderId, PairOrder, Result};
use std::collections::BTreeMap;

use crate::commitment::{verify_commitment, Commitment, CommitmentHash};
use crate::validation::validate_order;

/// Record of a committed order (before reveal)
#[derive(Debug, Clone)]
struct CommitRecord {
    commitment: Commitment,
    revealed: bool,
}

/// Order book for a single epoch with commit-reveal
#[derive(Debug, Clone)]
pub struct OrderBook {
    pub epoch_id: EpochId,
    commits: BTreeMap<CommitmentHash, CommitRecord>,
    revealed: BTreeMap<OrderId, (PairOrder, CommitmentHash)>,
    frozen: bool,
}

impl OrderBook {
    /// Create a new order book for an epoch
    pub fn new(epoch_id: EpochId) -> Self {
        OrderBook {
            epoch_id,
            commits: BTreeMap::new(),
            revealed: BTreeMap::new(),
            frozen: false,
        }
    }

    /// Submit a commitment (during collect phase)
    pub fn commit(&mut self, commitment: Commitment) -> Result<()> {
        if self.frozen {
            return Err(ConvexFxError::InvalidOrder(
                "order book is frozen".to_string(),
            ));
        }

        if commitment.epoch_id != self.epoch_id {
            return Err(ConvexFxError::InvalidOrder(format!(
                "commitment for wrong epoch: expected {}, got {}",
                self.epoch_id, commitment.epoch_id
            )));
        }

        // Check for duplicate commitment
        if self.commits.contains_key(&commitment.hash) {
            return Err(ConvexFxError::InvalidCommitment(
                "commitment already exists".to_string(),
            ));
        }

        let hash = commitment.hash.clone();
        self.commits.insert(
            hash,
            CommitRecord {
                commitment,
                revealed: false,
            },
        );

        Ok(())
    }

    /// Reveal an order (during reveal phase)
    pub fn reveal(&mut self, order: PairOrder, salt: &[u8]) -> Result<OrderId> {
        if self.frozen {
            return Err(ConvexFxError::InvalidOrder(
                "order book is frozen".to_string(),
            ));
        }

        // Validate order
        validate_order(&order)?;

        // Compute commitment from order and salt
        let computed_hash = crate::commitment::compute_commitment(&order, salt)?;

        // Check that commitment exists
        let record = self.commits.get_mut(&computed_hash).ok_or_else(|| {
            ConvexFxError::InvalidCommitment("commitment not found".to_string())
        })?;

        // Check not already revealed
        if record.revealed {
            return Err(ConvexFxError::InvalidCommitment(
                "commitment already revealed".to_string(),
            ));
        }

        // Verify commitment
        if !verify_commitment(&computed_hash, &order, salt)? {
            return Err(ConvexFxError::InvalidCommitment(
                "commitment verification failed".to_string(),
            ));
        }

        // Mark as revealed
        record.revealed = true;

        let order_id = order.id.clone();
        self.revealed
            .insert(order_id.clone(), (order, computed_hash));

        Ok(order_id)
    }

    /// Freeze the order book and return revealed orders in deterministic order
    /// Orders are sorted by (commitment_hash, order_id) for determinism
    pub fn freeze(mut self) -> Vec<PairOrder> {
        self.frozen = true;

        // Collect all revealed orders with their commitment hashes
        let mut orders_with_hashes: Vec<(CommitmentHash, PairOrder)> = self
            .revealed
            .into_iter()
            .map(|(_, (order, hash))| (hash, order))
            .collect();

        // Sort by commitment hash first, then by order ID for stable ordering
        orders_with_hashes.sort_by(|(hash1, order1), (hash2, order2)| {
            hash1.cmp(hash2).then_with(|| order1.id.cmp(&order2.id))
        });

        // Extract just the orders
        orders_with_hashes.into_iter().map(|(_, order)| order).collect()
    }

    /// Get count of commitments
    pub fn commitment_count(&self) -> usize {
        self.commits.len()
    }

    /// Get count of revealed orders
    pub fn revealed_count(&self) -> usize {
        self.revealed.len()
    }

    /// Check if frozen
    pub fn is_frozen(&self) -> bool {
        self.frozen
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use convexfx_types::{AccountId, Amount, AssetId};

    fn create_test_order(id: &str) -> PairOrder {
        PairOrder {
            id: id.to_string(),
            trader: AccountId::new("trader1"),
            pay: AssetId::USD,
            receive: AssetId::EUR,
            budget: Amount::from_units(1000),
            limit_ratio: None,
            min_fill_fraction: None,
            metadata: serde_json::json!({}),
        }
    }

    #[test]
    fn test_commit_reveal_flow() {
        let mut book = OrderBook::new(1);

        let order = create_test_order("order1");
        let salt = b"salt123";
        let hash = crate::commitment::compute_commitment(&order, salt).unwrap();

        // Commit
        book.commit(Commitment {
            hash: hash.clone(),
            epoch_id: 1,
            timestamp_ms: 1000,
        })
        .unwrap();

        assert_eq!(book.commitment_count(), 1);
        assert_eq!(book.revealed_count(), 0);

        // Reveal
        let order_id = book.reveal(order, salt).unwrap();
        assert_eq!(order_id, "order1");
        assert_eq!(book.revealed_count(), 1);
    }

    #[test]
    fn test_reveal_without_commit() {
        let mut book = OrderBook::new(1);
        let order = create_test_order("order1");
        let salt = b"salt123";

        // Reveal without commit should fail
        let result = book.reveal(order, salt);
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_commit() {
        let mut book = OrderBook::new(1);
        let order = create_test_order("order1");
        let salt = b"salt123";
        let hash = crate::commitment::compute_commitment(&order, salt).unwrap();

        let commitment = Commitment {
            hash: hash.clone(),
            epoch_id: 1,
            timestamp_ms: 1000,
        };

        book.commit(commitment.clone()).unwrap();

        // Second commit should fail
        let result = book.commit(commitment);
        assert!(result.is_err());
    }

    #[test]
    fn test_freeze_ordering() {
        let mut book = OrderBook::new(1);

        let order1 = create_test_order("order1");
        let order2 = create_test_order("order2");

        let salt1 = b"salt1";
        let salt2 = b"salt2";

        let hash1 = crate::commitment::compute_commitment(&order1, salt1).unwrap();
        let hash2 = crate::commitment::compute_commitment(&order2, salt2).unwrap();

        // Commit in one order
        book.commit(Commitment {
            hash: hash2.clone(),
            epoch_id: 1,
            timestamp_ms: 2000,
        })
        .unwrap();
        book.commit(Commitment {
            hash: hash1.clone(),
            epoch_id: 1,
            timestamp_ms: 1000,
        })
        .unwrap();

        // Reveal in another order
        book.reveal(order2.clone(), salt2).unwrap();
        book.reveal(order1.clone(), salt1).unwrap();

        // Freeze and check deterministic ordering (by commitment hash)
        let frozen = book.freeze();
        assert_eq!(frozen.len(), 2);

        // Orders should be sorted by commitment hash
        let expected_order = if hash1 < hash2 {
            vec!["order1", "order2"]
        } else {
            vec!["order2", "order1"]
        };

        assert_eq!(frozen[0].id, expected_order[0]);
        assert_eq!(frozen[1].id, expected_order[1]);
    }
}


