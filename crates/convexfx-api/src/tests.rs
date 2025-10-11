// Integration tests

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_app_state() {
        let state = AppState::new();
        // Test we can clone it
        let _state2 = state.clone();
    }
}


