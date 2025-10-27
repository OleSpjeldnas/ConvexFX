//! ConvexFX Delta Web Application
//!
//! A beautiful web interface for the ConvexFX Delta executor demo.
//! Features:
//! - User registration and wallet management
//! - Token balance viewing and transfers
//! - Real-time exchange metrics and pricing
//! - Trade execution and settlement
//! - Pool visualization and analytics

use actix_web::{web, App, HttpResponse, HttpServer, Result};
use actix_web::middleware::Logger;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use convexfx_delta::DemoApp;

#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(msg: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg),
        }
    }
}

#[derive(Debug, Serialize)]
struct UserInfo {
    user_id: String,
    balances: BTreeMap<String, i64>,
    total_value_usd: f64,
}

#[derive(Debug, Serialize)]
struct ExchangeMetrics {
    total_liquidity_usd: f64,
    active_pools: usize,
    total_volume_24h: f64,
    average_price_impact: f64,
    pools: Vec<PoolInfo>,
}

#[derive(Debug, Serialize)]
struct PoolInfo {
    assets: Vec<String>,
    liquidity_usd: f64,
    volume_24h: f64,
    fees_24h: f64,
    price_impact: f64,
}

#[derive(Debug, Serialize)]
struct TradePreview {
    from_amount: f64,
    to_amount: f64,
    price_impact: f64,
    fees: f64,
    route: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TransferRequest {
    from_user: String,
    to_user: String,
    amount: f64,
    asset: String,
}

#[derive(Debug, Deserialize)]
struct TradeRequest {
    from_asset: String,
    to_asset: String,
    amount: f64,
    user_id: String,
}

#[derive(Debug, Deserialize)]
struct RegisterUserRequest {
    user_id: String,
}

async fn health() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(ApiResponse::success("ConvexFX Delta Executor is running")))
}

async fn get_pool_info(
    app: web::Data<std::sync::Arc<DemoApp>>,
) -> Result<HttpResponse> {
    let pool_liquidity = app.get_pool_liquidity();
    
    #[derive(Serialize)]
    struct PoolInfo {
        total_liquidity: BTreeMap<String, f64>,
        assets: Vec<AssetInfo>,
    }
    
    #[derive(Serialize)]
    struct AssetInfo {
        asset: String,
        amount: f64,
        value_usd: f64,
    }
    
    // Calculate total value in USD for each asset
    let mut assets = Vec::new();
    let exchange_rates = BTreeMap::from([
        ("USD", 1.0),
        ("EUR", 1.0 / 0.86),
        ("GBP", 1.0 / 0.77),
        ("JPY", 1.0 / 149.0),
        ("CHF", 1.0 / 0.88),
        ("AUD", 1.0 / 1.5),
    ]);
    
    for (asset, &amount) in &pool_liquidity {
        let rate = exchange_rates.get(asset.as_str()).copied().unwrap_or(1.0);
        assets.push(AssetInfo {
            asset: asset.clone(),
            amount,
            value_usd: amount * rate,
        });
    }
    
    let info = PoolInfo {
        total_liquidity: pool_liquidity,
        assets,
    };
    
    Ok(HttpResponse::Ok().json(ApiResponse::success(info)))
}

async fn get_user_info(
    path: web::Path<String>,
    app: web::Data<std::sync::Arc<DemoApp>>,
) -> Result<HttpResponse> {
    let user_id = path.into_inner();

    let mut balances = BTreeMap::new();
    let assets = vec!["USD", "EUR", "GBP", "JPY", "CHF", "AUD"];
    
    for asset in &assets {
        if let Ok(balance) = app.get_balance(&user_id, asset) {
            balances.insert(asset.to_string(), balance);
        }
    }

    if balances.is_empty() {
        return Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error("User not registered".to_string())));
    }

    // Calculate total value (simplified exchange rates)
    let total_value_usd = 
        *balances.get("USD").unwrap_or(&0) as f64 +
        (*balances.get("EUR").unwrap_or(&0) as f64 * 1.1) +
        (*balances.get("GBP").unwrap_or(&0) as f64 * 1.3) +
        (*balances.get("JPY").unwrap_or(&0) as f64 * 0.009) +
        (*balances.get("CHF").unwrap_or(&0) as f64 * 1.15) +
        (*balances.get("AUD").unwrap_or(&0) as f64 * 0.7);

    let user_info = UserInfo {
        user_id,
        balances,
        total_value_usd,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(user_info)))
}

async fn register_user(
    req: web::Json<RegisterUserRequest>,
    app: web::Data<std::sync::Arc<DemoApp>>,
) -> Result<HttpResponse> {
    match app.register_user(&req.user_id) {
        Ok(_) => {
            Ok(HttpResponse::Ok().json(ApiResponse::success(format!("User {} registered successfully", req.user_id))))
        }
        Err(e) => {
            Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(e.to_string())))
        }
    }
}


async fn preview_trade(
    req: web::Json<TradeRequest>,
    app: web::Data<std::sync::Arc<DemoApp>>,
) -> Result<HttpResponse> {
    // Use the actual clearing engine to preview the trade
    match app.preview_trade(&req.from_asset, &req.to_asset, req.amount as i64) {
        Ok((to_amount, price_impact)) => {
            let fees = req.amount * 0.003; // 0.3% fee
            
            let preview = TradePreview {
                from_amount: req.amount,
                to_amount, // Already f64, no conversion needed!
                price_impact,
                fees,
                route: vec![req.from_asset.clone(), req.to_asset.clone()],
            };

            Ok(HttpResponse::Ok().json(ApiResponse::success(preview)))
        }
        Err(e) => {
            Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(e.to_string())))
        }
    }
}

async fn serve_index() -> Result<HttpResponse> {
    let html = include_str!("../../static/index.html");
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}

async fn serve_static_file(path: web::Path<String>) -> Result<HttpResponse> {
    let filename = path.into_inner();
    
    // Handle binary files (images)
    if filename == "assets/images/logo.png" {
        let logo_bytes = include_bytes!("../../static/assets/images/logo.png");
        return Ok(HttpResponse::Ok()
            .content_type("image/png")
            .body(logo_bytes.as_ref()));
    }
    
    // Handle text files
    let content = match filename.as_str() {
        "styles.css" => include_str!("../../static/styles.css"),
        "script.js" => include_str!("../../static/script.js"),
        _ => return Ok(HttpResponse::NotFound().body("File not found")),
    };

    let content_type = match filename.as_str() {
        "styles.css" => "text/css",
        "script.js" => "application/javascript",
        _ => "text/plain",
    };

    Ok(HttpResponse::Ok()
        .content_type(content_type)
        .body(content))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    println!("🔧 Initializing demo app...");
    let app = match DemoApp::new() {
        Ok(app) => {
            println!("✅ Demo app initialized successfully");
            std::sync::Arc::new(app)
        }
        Err(e) => {
            eprintln!("❌ Failed to create demo app: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
        }
    };

    println!("🚀 Starting ConvexFX Delta Web Application");
    println!("📊 Exchange metrics available at: http://localhost:8080/api/metrics");
    println!("👤 User management at: http://localhost:8080/api/user/alice");
    println!("🌐 Web interface at: http://localhost:8080");
    println!();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app.clone()))
            .wrap(Logger::default())
            .route("/", web::get().to(serve_index))
            .route("/static/{filename:.*}", web::get().to(serve_static_file))
            .route("/api/health", web::get().to(health))
            .route("/api/pool", web::get().to(get_pool_info))
            .route("/api/user/register", web::post().to(register_user))
            .route("/api/user/{user_id}", web::get().to(get_user_info))
            .route("/api/trade/preview", web::post().to(preview_trade))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
