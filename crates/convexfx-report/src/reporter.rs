use convexfx_types::EpochId;
use serde::{Deserialize, Serialize};

use crate::hashing::{compute_json_hash, HashRef};

/// Epoch report with input/output hashes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochReport {
    pub epoch_id: EpochId,
    pub input_hash: HashRef,
    pub output_hash: HashRef,
    pub report_data: ReportData,
}

/// Report data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportData {
    pub inputs: serde_json::Value,
    pub outputs: serde_json::Value,
}

/// Reporter trait
pub trait Reporter {
    fn publish(&self, epoch_id: EpochId, inputs: &serde_json::Value, outputs: &serde_json::Value) -> Result<EpochReport, String>;
}

/// Simple in-memory reporter
pub struct MemoryReporter;

impl MemoryReporter {
    pub fn new() -> Self {
        MemoryReporter
    }
}

impl Default for MemoryReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Reporter for MemoryReporter {
    fn publish(
        &self,
        epoch_id: EpochId,
        inputs: &serde_json::Value,
        outputs: &serde_json::Value,
    ) -> Result<EpochReport, String> {
        let input_hash = compute_json_hash(inputs).map_err(|e| e.to_string())?;
        let output_hash = compute_json_hash(outputs).map_err(|e| e.to_string())?;

        Ok(EpochReport {
            epoch_id,
            input_hash,
            output_hash,
            report_data: ReportData {
                inputs: inputs.clone(),
                outputs: outputs.clone(),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_reporter() {
        let reporter = MemoryReporter::new();

        let inputs = serde_json::json!({"epoch": 1});
        let outputs = serde_json::json!({"fills": []});

        let report = reporter.publish(1, &inputs, &outputs).unwrap();

        assert_eq!(report.epoch_id, 1);
        assert_eq!(report.input_hash.len(), 64);
        assert_eq!(report.output_hash.len(), 64);
    }
}


