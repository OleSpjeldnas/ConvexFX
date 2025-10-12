use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;

use crate::handlers;
use crate::state::AppState;

/// Create the API application router
pub fn create_app(state: AppState) -> Router {
    Router::new()
        // Health and info
        .route("/health", get(handlers::health))
        .route("/v1/info", get(handlers::info))

        // Orders
        .route("/v1/orders/submit", post(handlers::submit_order))
        .route("/v1/orders/reveal", post(handlers::reveal_order))
        .route("/v1/orders/commit", post(handlers::submit_commitment))

        // Prices and market data
        .route("/v1/prices", get(handlers::get_prices))

        // Epochs
        .route("/v1/epochs", get(handlers::list_epochs))
        .route("/v1/epochs/current", get(handlers::get_epoch))
        .route("/v1/epochs/:epoch_id", get(handlers::get_epoch_by_id))

        // System status
        .route("/v1/status", get(handlers::get_system_status))

        // Asset management
        .route("/v1/assets", get(handlers::list_assets))
        .route("/v1/assets", post(handlers::add_asset))

        // Liquidity management
        .route("/v1/liquidity", get(handlers::get_liquidity))
        .route("/v1/liquidity", post(handlers::provide_liquidity))

        .layer(CorsLayer::permissive())
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_app() {
        let state = AppState::new();
        let _app = create_app(state);
        // Just testing it compiles and creates
    }
}


