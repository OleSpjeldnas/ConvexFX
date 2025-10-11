// Integration tests for report

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_report_determinism() {
        let reporter = MemoryReporter::new();

        let inputs = serde_json::json!({"key": "value"});
        let outputs = serde_json::json!({"result": 42});

        let report1 = reporter.publish(1, &inputs, &outputs).unwrap();
        let report2 = reporter.publish(1, &inputs, &outputs).unwrap();

        // Same inputs should produce same hashes
        assert_eq!(report1.input_hash, report2.input_hash);
        assert_eq!(report1.output_hash, report2.output_hash);
    }
}


