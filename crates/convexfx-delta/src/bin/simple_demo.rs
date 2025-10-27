//! Simple ConvexFX Delta Demo
//!
//! A basic interactive demo that shows the core concepts:
//! - User registration with initial funding
//! - Balance checking and token transfers
//! - ConvexFX order execution demonstration

use clap::Parser;
use convexfx_delta::{DemoApp, DeltaIntegrationError};
use std::collections::BTreeMap;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    match args.command {
        Command::Register { user_id } => {
            let app = DemoApp::new().unwrap();
            match app.register_user(&user_id) {
                Ok(()) => {
                    println!("✅ User '{}' registered with initial funding", user_id);
                    println!("💡 Run 'balance {}' to check balances", user_id);
                }
                Err(e) => eprintln!("❌ Failed to register user: {}", e),
            }
        }

        Command::Balance { user_id } => {
            let app = DemoApp::new().unwrap();

            match app.get_balance(&user_id, "USD") {
                Ok(usd_balance) => println!("💰 {} USD balance: {}", user_id, usd_balance),
                Err(e) => eprintln!("❌ Failed to get USD balance: {}", e),
            }

            match app.get_balance(&user_id, "EUR") {
                Ok(eur_balance) => println!("💰 {} EUR balance: {}", user_id, eur_balance),
                Err(e) => eprintln!("❌ Failed to get EUR balance: {}", e),
            }

            match app.get_balance(&user_id, "JPY") {
                Ok(jpy_balance) => println!("💰 {} JPY balance: {}", user_id, jpy_balance),
                Err(e) => eprintln!("❌ Failed to get JPY balance: {}", e),
            }
        }

        Command::Transfer {
            from_user,
            to_user,
            amount,
            asset
        } => {
            let app = DemoApp::new().unwrap();

            // Check sender has sufficient balance
            match app.get_balance(&from_user, &asset) {
                Ok(balance) => {
                    if balance < amount {
                        eprintln!("❌ Insufficient balance: {} has {} {} but needs {}", from_user, balance, asset, amount);
                        return;
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to check balance: {}", e);
                    return;
                }
            }

            match app.transfer(&from_user, &to_user, amount, &asset) {
                Ok(()) => {
                    println!("✅ Successfully transferred {} {} from {} to {}", amount, asset, from_user, to_user);
                    println!("💡 Check balances with 'balance {}' and 'balance {}'", from_user, to_user);
                }
                Err(e) => eprintln!("❌ Failed to process transfer: {}", e),
            }
        }

        Command::ExecuteOrders => {
            let app = DemoApp::new().unwrap();

            // Create some demo orders
            let orders = vec![
                convexfx_types::PairOrder {
                    id: "demo_order_1".to_string(),
                    trader: convexfx_types::AccountId::new("alice".to_string()),
                    pay: convexfx_types::AssetId::USD,
                    receive: convexfx_types::AssetId::EUR,
                    budget: convexfx_types::Amount::from_f64(1000.0).unwrap(),
                    limit_ratio: Some(1.1),
                    min_fill_fraction: Some(0.5),
                    metadata: serde_json::json!({"demo": true}),
                }
            ];

            match app.execute_orders(orders) {
                Ok((fills, _state_diffs)) => {
                    println!("✅ Executed {} fills through ConvexFX clearing", fills.len());
                    for fill in fills {
                        println!("   Order {}: {} {} → {} {} @ {:.4}",
                            fill.order_id,
                            fill.pay_units,
                            fill.pay_asset,
                            fill.recv_units,
                            fill.recv_asset,
                            fill.recv_units / fill.pay_units);
                    }
                }
                Err(e) => eprintln!("❌ Failed to execute orders: {}", e),
            }
        }

        Command::Balances => {
            let app = DemoApp::new().unwrap();

            match app.get_all_balances() {
                Ok(balances) => {
                    println!("📊 All User Balances:");
                    for (user_id, assets) in balances {
                        println!("  {}: {:?}", user_id, assets);
                    }
                }
                Err(e) => eprintln!("❌ Failed to get balances: {}", e),
            }
        }

        Command::Demo => {
            let app = DemoApp::new().unwrap();

            println!("🚀 Running complete demo scenario...");

            // Register users
            app.register_user("alice").unwrap();
            app.register_user("bob").unwrap();

            println!("✅ Registered users Alice and Bob");

            // Show initial balances
            println!("\n📊 Initial Balances:");
            show_user_balance(&app, "alice");
            show_user_balance(&app, "bob");

            // Transfer USD from Alice to Bob
            app.transfer("alice", "bob", 1000, "USD").unwrap();

            println!("\n📊 After Transfer:");
            show_user_balance(&app, "alice");
            show_user_balance(&app, "bob");

            // Execute some orders
            let orders = vec![
                convexfx_types::PairOrder {
                    id: "demo_order_1".to_string(),
                    trader: convexfx_types::AccountId::new("alice".to_string()),
                    pay: convexfx_types::AssetId::USD,
                    receive: convexfx_types::AssetId::EUR,
                    budget: convexfx_types::Amount::from_f64(500.0).unwrap(),
                    limit_ratio: Some(1.1),
                    min_fill_fraction: Some(0.5),
                    metadata: serde_json::json!({"demo": true}),
                }
            ];

            match app.execute_orders(orders) {
                Ok((fills, _state_diffs)) => {
                    println!("\n✅ Executed {} fills through ConvexFX clearing", fills.len());
                    for fill in fills {
                        println!("   Order {}: {} {} → {} {} @ {:.4}",
                            fill.order_id,
                            fill.pay_units,
                            fill.pay_asset,
                            fill.recv_units,
                            fill.recv_asset,
                            fill.recv_units / fill.pay_units);
                    }
                }
                Err(e) => eprintln!("❌ Failed to execute orders: {}", e),
            }

            // Show final balances
            println!("\n📊 Final Balances:");
            show_user_balance(&app, "alice");
            show_user_balance(&app, "bob");
        }
    }
}

fn show_user_balance(app: &DemoApp, user_id: &str) {
    match app.get_balance(user_id, "USD") {
        Ok(usd) => print!("  {}: USD={}, ", user_id, usd),
        Err(_) => print!("  {}: USD=unknown, ", user_id),
    }
    match app.get_balance(user_id, "EUR") {
        Ok(eur) => print!("EUR={}, ", eur),
        Err(_) => print!("EUR=unknown, "),
    }
    match app.get_balance(user_id, "JPY") {
        Ok(jpy) => println!("JPY={}", jpy),
        Err(_) => println!("JPY=unknown"),
    }
}

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser)]
enum Command {
    /// Register a new user
    Register { user_id: String },

    /// Check a user's vault balance
    Balance { user_id: String },

    /// Transfer tokens between users
    Transfer {
        from_user: String,
        to_user: String,
        amount: i64,
        asset: String,
    },

    /// Execute orders through ConvexFX clearing
    ExecuteOrders,

    /// Show all user balances
    Balances,

    /// Run a complete demo scenario
    Demo,
}
