# ConvexFX Delta Web Application

A beautiful, modern web interface for the ConvexFX Delta executor demo.

## Features

- 🔐 **User Registration & Wallet Management** - Create and manage user wallets
- 💰 **Token Balance Viewing** - See all user balances in real-time
- 💸 **Token Transfers** - Send tokens between users
- 📊 **Exchange Metrics** - View liquidity pools, volume, and fees
- 📈 **Trade Execution** - Execute trades with price impact preview
- 🎨 **Modern UI** - Clean, responsive design with dark theme

## Quick Start

### Option 1: Using the Launch Scripts (Recommended)

Start the server:
```bash
./start_web_app.sh
```

Stop the server:
```bash
./stop_web_app.sh
```

### Option 2: Manual Start

```bash
cargo run --bin web_app --features runtime
```

The server will start on `http://localhost:8080`

**Note**: The web server runs continuously (it doesn't "stall" - it's waiting for requests). Press `Ctrl+C` to stop it.

## API Endpoints

### Health Check
```bash
curl http://localhost:8080/api/health
```

### Get User Info
```bash
curl http://localhost:8080/api/user/{user_id}
```

### Get Exchange Metrics
```bash
curl http://localhost:8080/api/metrics
```

### Preview Trade
```bash
curl -X POST http://localhost:8080/api/trade/preview \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "alice",
    "from_asset": "USD",
    "to_asset": "EUR",
    "amount": 1000.0
  }'
```

### Execute Transfer
```bash
curl -X POST http://localhost:8080/api/transfer \
  -H "Content-Type: application/json" \
  -d '{
    "from_user": "alice",
    "to_user": "bob",
    "asset": "USD",
    "amount": 100
  }'
```

## Web Interface

Open your browser and navigate to:
- **Main Interface**: http://localhost:8080
- **API Metrics**: http://localhost:8080/api/metrics
- **Health Check**: http://localhost:8080/api/health

## Architecture

The web application consists of:

1. **Backend** (`crates/convexfx-delta/src/bin/web_app.rs`)
   - Actix-Web HTTP server
   - REST API endpoints
   - Integration with DemoApp for state management

2. **Frontend** (`crates/convexfx-delta/static/`)
   - `index.html` - Main HTML structure
   - `styles.css` - Modern, dark-themed styling
   - `script.js` - Interactive UI logic

3. **Demo App** (`crates/convexfx-delta/src/demo_app.rs`)
   - Simplified vault management
   - Token balance tracking
   - Order execution via ConvexFX clearing engine

## Development

### Build Only
```bash
cargo build --bin web_app --features runtime
```

### Run with Logs
```bash
RUST_LOG=debug cargo run --bin web_app --features runtime
```

### Check for Errors
```bash
cargo check --bin web_app --features runtime
```

## Troubleshooting

### "Server is stalling"
The server is **not** stalling - web servers run continuously to serve requests. This is normal behavior. Use `Ctrl+C` or the stop script to terminate it.

### Port Already in Use
If port 8080 is already in use:
```bash
# Find and kill the process
lsof -ti:8080 | xargs kill -9

# Or use the stop script
./stop_web_app.sh
```

### Check Logs
```bash
tail -f logs/web_app.log
```

## Related

- **CLI Demo**: `cargo run --bin simple_demo --features demo -- demo`
- **Main Exchange**: See `/crates/convexfx-exchange/`
- **Delta Integration**: See `/crates/convexfx-delta/`

