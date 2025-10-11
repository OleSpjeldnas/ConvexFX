use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use convexfx_types::{AssetId, AccountId, PairOrder, Amount};
use sha2::{Sha256, Digest};
use hex;

use crate::state::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

#[derive(Serialize)]
pub struct InfoResponse {
    pub name: String,
    pub description: String,
}

/// Health check endpoint
pub async fn health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Info endpoint
pub async fn info() -> impl IntoResponse {
    Json(InfoResponse {
        name: "ConvexFX".to_string(),
        description: "Sequential Convex Programming FX AMM".to_string(),
    })
}

#[derive(Deserialize)]
pub struct CommitRequest {
    pub epoch_hint: u64,
}

#[derive(Serialize)]
pub struct CommitResponse {
    pub accepted: bool,
    pub epoch: u64,
}


#[derive(Deserialize)]
pub struct OrderSubmissionRequest {
    pub pay_asset: String,
    pub receive_asset: String,
    pub budget: String, // Amount as string for JSON
    pub limit_ratio: Option<f64>,
    pub min_fill_fraction: Option<f64>,
}

#[derive(Serialize)]
pub struct OrderSubmissionResponse {
    pub order_id: String,
    pub commitment_hash: String,
    pub accepted: bool,
}

#[derive(Deserialize)]
pub struct OrderRevealRequest {
    pub order_id: String,
    pub pay_asset: String,
    pub receive_asset: String,
    pub budget: String,
    pub trader: String,
    pub limit_ratio: Option<f64>,
    pub min_fill_fraction: Option<f64>,
}

#[derive(Serialize)]
pub struct OrderRevealResponse {
    pub accepted: bool,
    pub epoch_id: u64,
}

#[derive(Serialize)]
pub struct PriceResponse {
    pub asset: String,
    pub price: f64,
    pub log_price: f64,
}

#[derive(Serialize)]
pub struct PricesResponse {
    pub prices: Vec<PriceResponse>,
    pub epoch_id: u64,
}

#[derive(Serialize)]
pub struct EpochListResponse {
    pub epochs: Vec<EpochInfo>,
}

#[derive(Serialize)]
pub struct EpochInfo {
    pub epoch_id: u64,
    pub state: String,
    pub order_count: usize,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

#[derive(Serialize)]
pub struct SystemStatusResponse {
    pub status: String,
    pub current_epoch: u64,
    pub total_accounts: usize,
    pub total_orders_pending: usize,
    pub solver_backend: String,
    pub uptime_seconds: u64,
}

/// Submit a new order (creates commitment)
pub async fn submit_order(
    State(state): State<AppState>,
    Json(req): Json<OrderSubmissionRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Parse assets
    let pay_asset = match AssetId::from_str(&req.pay_asset) {
        Some(asset) => asset,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid pay asset"}))),
    };

    let receive_asset = match AssetId::from_str(&req.receive_asset) {
        Some(asset) => asset,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid receive asset"}))),
    };

    // Parse budget
    let budget = match Amount::from_string(&req.budget) {
        Ok(amount) => amount,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid budget format"}))),
    };

    // Create order
    let order = PairOrder {
        id: format!("order_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()),
        trader: AccountId::new("api_user"), // TODO: Get from auth
        pay: pay_asset,
        receive: receive_asset,
        budget,
        limit_ratio: req.limit_ratio,
        min_fill_fraction: req.min_fill_fraction,
        metadata: serde_json::json!({}),
    };

    // Create commitment hash
    let mut hasher = Sha256::new();
    let commitment_data = format!("{:?}", order);
    hasher.update(commitment_data.as_bytes());
    let hash = hasher.finalize();
    let commitment_hash = hex::encode(hash);

    // Store commitment in orderbook
    let mut orderbook = state.orderbook.lock().unwrap();
    use convexfx_orders::{Commitment, CommitmentHash};

    // Create a proper commitment hash
    let commitment_hash_obj = CommitmentHash::from_hex(&commitment_hash).unwrap();

    match orderbook.commit(Commitment {
        hash: commitment_hash_obj,
        epoch_id: 1, // TODO: Get current epoch
        timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
    }) {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({
            "order_id": order.id,
            "commitment_hash": commitment_hash,
            "accepted": true
        }))),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Failed to commit order"}))),
    }
}

/// Reveal an order (submit actual order details)
pub async fn reveal_order(
    State(_state): State<AppState>,
    Json(_req): Json<OrderRevealRequest>,
) -> impl IntoResponse {
    // This would validate the commitment and add the order to the current epoch
    // For now, return success
    (StatusCode::OK, Json(OrderRevealResponse {
        accepted: true,
        epoch_id: 1,
    }))
}

/// Get current prices from oracle
pub async fn get_prices(
    State(state): State<AppState>,
) -> impl IntoResponse {
    use convexfx_oracle::Oracle;

    let oracle = state.oracle.lock().unwrap();
    let prices = oracle.current_prices().unwrap();

    let mut price_list = Vec::new();
    for asset in AssetId::all() {
        let y = prices.get_ref(*asset);
        price_list.push(PriceResponse {
            asset: asset.to_string(),
            price: y.exp(),
            log_price: y,
        });
    }

    Json(PricesResponse {
        prices: price_list,
        epoch_id: *state.current_epoch.lock().unwrap(),
    })
}

/// List epochs
pub async fn list_epochs(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    // Placeholder: return list of recent epochs
    let epochs = vec![
        EpochInfo {
            epoch_id: 1,
            state: "COMPLETED".to_string(),
            order_count: 5,
            start_time: Some("2025-01-01T00:00:00Z".to_string()),
            end_time: Some("2025-01-01T00:01:00Z".to_string()),
        }
    ];

    Json(EpochListResponse { epochs })
}

/// Get epoch details by ID
pub async fn get_epoch_by_id(
    State(_state): State<AppState>,
    Path(_epoch_id): Path<u64>,
) -> impl IntoResponse {
    // Placeholder: return epoch details
    Json(EpochInfo {
        epoch_id: 1,
        state: "COMPLETED".to_string(),
        order_count: 0,
        start_time: None,
        end_time: None,
    })
}

/// Get system status and metrics
pub async fn get_system_status(
    State(state): State<AppState>,
) -> impl IntoResponse {
    use convexfx_ledger::Ledger;

    let ledger = state.ledger.lock().unwrap();
    let orderbook = state.orderbook.lock().unwrap();

    Json(SystemStatusResponse {
        status: "healthy".to_string(),
        current_epoch: *state.current_epoch.lock().unwrap(),
        total_accounts: ledger.list_accounts().len(),
        total_orders_pending: orderbook.commitment_count(),
        solver_backend: "clarabel".to_string(),
        uptime_seconds: 3600, // TODO: Track actual uptime
    })
}

/// Submit commitment endpoint (legacy)
pub async fn submit_commitment(
    State(_state): State<AppState>,
    Json(req): Json<CommitRequest>,
) -> impl IntoResponse {
    // Placeholder: accept all commitments
    (
        StatusCode::OK,
        Json(CommitResponse {
            accepted: true,
            epoch: req.epoch_hint,
        }),
    )
}

#[derive(Serialize)]
pub struct EpochResponse {
    pub epoch_id: u64,
    pub state: String,
}

/// Get current epoch info
pub async fn get_epoch(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    // Placeholder response
    Json(EpochResponse {
        epoch_id: 1,
        state: "COLLECT".to_string(),
    })
}
