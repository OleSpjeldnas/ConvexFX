// Integration tests for simulation

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_scenario_creation() {
        let scenario = Scenario::default_scenario();
        assert_eq!(scenario.config.name, "default");
    }
}


