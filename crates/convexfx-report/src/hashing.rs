use sha2::{Digest, Sha256};

/// Hash reference (hex-encoded SHA256)
pub type HashRef = String;

/// Compute SHA256 hash of data
pub fn compute_hash(data: &[u8]) -> HashRef {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex::encode(result)
}

/// Compute hash of JSON-serialized data
pub fn compute_json_hash<T: serde::Serialize>(data: &T) -> Result<HashRef, serde_json::Error> {
    let json = serde_json::to_vec(data)?;
    Ok(compute_hash(&json))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash() {
        let data = b"hello world";
        let hash = compute_hash(data);
        assert_eq!(hash.len(), 64); // SHA256 hex is 64 chars
    }

    #[test]
    fn test_deterministic() {
        let data = b"test";
        let hash1 = compute_hash(data);
        let hash2 = compute_hash(data);
        assert_eq!(hash1, hash2);
    }
}


