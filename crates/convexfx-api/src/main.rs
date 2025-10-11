use convexfx_api::{create_app, AppState};
use tracing_subscriber;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create app state
    let state = AppState::new();

    // Create the app
    let app = create_app(state);

    // Bind to address
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("ConvexFX API server running on http://127.0.0.1:3000");

    // Run the server
    axum::serve(listener, app).await.unwrap();
}


