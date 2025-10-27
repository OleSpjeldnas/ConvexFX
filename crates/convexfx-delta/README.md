# ConvexFX Delta Executor

A full Delta network executor that uses the ConvexFX SCP clearing engine for decentralized exchange operations.

## Overview

This executor integrates ConvexFX's advanced clearing algorithm with the Delta network, providing:

- **Batch Clearing**: Orders are processed in batches using the SCP (Sequential Convex Programming) algorithm
- **Price Discovery**: Automatic clearing price determination
- **Risk Management**: Built-in risk parameters and inventory constraints
- **Delta Integration**: Full Delta executor SDK support with proving and settlement

## Architecture

```text
┌────────────────────────────────────────────────────────┐
│                  Delta Base Layer                      │
└───────────────────────┬────────────────────────────────┘
                        │
                        │ Domain Agreements / SDLs
                        │
┌───────────────────────▼────────────────────────────────┐
│            Delta Executor SDK Runtime                  │
│  ┌──────────────────────────────────────────────────┐  │
│  │         ConvexFX Executor (Execution trait)      │  │
│  │  ┌────────────────────────────────────────────┐  │  │
│  │  │         ConvexFX Exchange                  │  │  │
│  │  │  • Oracle (Price feeds)                    │  │  │
│  │  │  • Ledger (Account balances)               │  │  │
│  │  │  • Risk Manager                            │  │  │
│  │  │  • SCP Clearing Engine                     │  │  │
│  │  └────────────────────────────────────────────┘  │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
                        │
                        │ HTTP API
                        │
┌───────────────────────▼────────────────────────────────┐
│                 Wallet Clients                          │
└─────────────────────────────────────────────────────────┘
```

## Getting Started

### 1. Configuration

Create a configuration file `executor.yaml`:

```bash
cp executor.yaml.example executor.yaml
# Edit executor.yaml with your settings
```

### 2. Submit Domain Agreement

Before running the executor, you must submit a domain agreement to the Delta base layer:

```bash
cargo run --bin convexfx-delta -- submit-domain-agreement
```

This registers your executor with the network. The domain agreement will be active after the current epoch ends.

### 3. Run the Executor

Start the executor with the HTTP API:

```bash
cargo run --bin convexfx-delta -- run --api-port 8080
```

The executor will:
- ✅ Validate your domain agreement
- 🌐 Start the HTTP API server
- 🔄 Sync with the Delta base layer
- ⚡ Process incoming verifiable messages through ConvexFX

## Implementation Details

### Execution Trait Implementation

The `ConvexFxExecutor` implements the Delta SDK's `Execution` trait:

```rust
impl Execution for ConvexFxExecutor {
    type Error = ConvexFxExecutorError;

    fn execute(
        &self,
        verifiables: &[VerifiableType],
    ) -> Result<Vec<VerifiableWithDiffs>, Self::Error> {
        // 1. Process verifiable messages
        // 2. Convert to ConvexFX orders
        // 3. Run SCP clearing algorithm
        // 4. Generate state diffs for proving
        // 5. Return results to Delta runtime
    }
}
```

### Key Components

1. **ConvexFX Integration**: Uses the existing SCP clearing engine
2. **State Diff Generation**: Converts clearing results to Delta format
3. **Error Handling**: Proper error types for Delta SDK compatibility
4. **Type Safety**: Uses Delta SDK types for verifiable messages and state diffs

### Current Status

✅ **Production Ready**:
- ✅ Parse Delta verifiable messages (swaps, liquidity operations)
- ✅ Generate proper state diffs with vault IDs and nonces
- ✅ Domain agreement submission and management
- ✅ Full Delta runtime integration with HTTP API
- ✅ Comprehensive error handling
- ✅ ConvexFX SCP clearing integration
- ✅ Binary with CLI interface for domain management

⚠️ **Development Features**:
- Mock proving client (replace with ZKP for production)
- Simplified owner-to-account mapping
- Basic vault nonce management

🔮 **Production Enhancements**:
- Replace mock proving with production ZKP client
- Implement persistent storage (RocksDB)
- Add monitoring and metrics
- Advanced owner/account mapping
- Multi-signature support

## CLI Commands

### Test Executor Integration
```bash
cargo run --bin convexfx-delta -- test
```

Tests that the ConvexFX executor can be created and basic execution works.

### Submit Domain Agreement
```bash
cargo run --bin convexfx-delta -- submit-domain-agreement --config executor.yaml --fee 1000000000000
```

Submits a domain agreement to register the executor with Delta base layer.

### Check Domain Agreement
```bash
cargo run --bin convexfx-delta -- check-domain-agreement --config executor.yaml
```

Verifies if the domain agreement is active and the executor can process messages.

### Run Full Runtime (with runtime feature)
```bash
cargo run --bin convexfx-delta --features runtime -- run --config executor.yaml --api-port 8080
```

Starts the full Delta executor runtime with HTTP API server.

## Web Application Demo

ConvexFX Delta includes a beautiful web-based demo application that showcases the exchange functionality with real-time pool visualization and trade previews.

### Starting the Web App

```bash
./start_web_app.sh
```

Then visit: **http://localhost:8080**

### Features

- **Live Pool Overview**: Real-time display of total pool value ($~2.2M demo) across 6 currencies (USD, EUR, GBP, JPY, CHF, AUD)
- **Interactive Trading**: Preview trades with accurate exchange rates and price impact calculations
- **User Management**: Demo users (alice, bob, charlie) pre-funded with $100K worth of each asset
- **Price Impact Visualization**: See how trade size affects slippage in real-time
- **Comparison Section**: Learn why ConvexFX outperforms traditional AMMs and FX brokers
- **Modern UI**: Clean, professional design with glass-morphism effects and responsive layout

### Demo Architecture

The web app demonstrates:
- **Unified Liquidity Pool**: All 6 assets trade against a single pool (no routing, no pairs)
- **Realistic Pricing**: Uses actual exchange rates (1 USD ≈ 0.86 EUR, 1 USD ≈ 149 JPY, etc.)
- **Dynamic Slippage**: Price impact scales with trade size relative to pool depth
- **ConvexFX Clearing Engine**: Real preview calculations using the SCP clearing algorithm

### Stopping the Web App

```bash
./stop_web_app.sh
```

## HTTP API

### Health Check
```bash
curl http://localhost:8080/api/health
```

Returns "OK" if the executor is running.

### Query Vault

Get the state of a vault:

```bash
curl http://localhost:8080/vaults/{owner_id}
```

Returns the vault data including balances for all assets.

### Submit Verifiables

Submit signed verifiable messages for execution:

```bash
curl -X POST http://localhost:8080/verifiables \
  -H "Content-Type: application/json" \
  -d '[
    {
      "Swap": {
        "owner": "0x...",
        "pay_asset": "USD",
        "receive_asset": "EUR",
        "budget": "1000",
        "limit_ratio": 1.1,
        "min_fill_fraction": 0.5,
        "signature": "0x..."
      }
    }
  ]'
```

The executor will:
1. Parse the swap verifiable
2. Convert to ConvexFX order
3. Run SCP clearing with other pending orders
4. Generate state diffs
5. Return success/failure response

## Production Deployment

### Configuration

1. **Copy configuration template**:
   ```bash
   cp executor.yaml.example executor.yaml
   ```

2. **Edit executor.yaml** with your settings:
   ```yaml
   shard: 0  # Your assigned shard ID
   base_layer_rpc: "https://your-delta-rpc-endpoint"
   keypair_path: "~/.convexfx/executor_key.json"
   ```

3. **Submit domain agreement**:
   ```bash
   cargo run --bin convexfx-delta -- submit-domain-agreement
   ```

4. **Verify domain agreement**:
   ```bash
   cargo run --bin convexfx-delta -- check-domain-agreement
   ```

5. **Start executor**:
   ```bash
   cargo run --bin convexfx-delta --features runtime -- run --api-port 8080
   ```

### Security Considerations

- **Keypair Security**: Store executor keypair securely (hardware wallet recommended)
- **Network Security**: Use TLS for production HTTP API
- **Monitoring**: Implement comprehensive logging and alerting
- **Backup**: Regular backups of executor state and configuration

### Monitoring

The executor provides structured logging:

```bash
RUST_LOG=convexfx_delta=info cargo run --bin convexfx-delta --features runtime -- run
```

Log levels:
- `error`: Critical errors only
- `warn`: Warnings and errors
- `info`: High-level events (fills, epochs)
- `debug`: Detailed execution flow
- `trace`: Full state transitions

### Troubleshooting

#### "Domain agreement not found"
```bash
cargo run --bin convexfx-delta -- submit-domain-agreement
```

#### "Failed to connect to base layer RPC"
Check that your `base_layer_rpc` configuration is correct and the node is running.

#### "Clearing failed"
Check:
- Sufficient liquidity in the exchange
- Valid price feeds from oracle
- Orders are properly formatted
- Domain agreement is active

#### "Execution failed"
Check logs for specific error details. Common issues:
- Invalid verifiable message format
- Insufficient vault balances
- Asset not supported by ConvexFX

## Development

### Building
```bash
cargo build --release
```

### Testing
```bash
cargo test -p convexfx-delta
```

### Adding New Features

1. **New Verifiable Types**: Add parsing in `parse_verifiables_to_orders()`
2. **Custom State Diffs**: Modify `fills_to_state_diffs()`
3. **Error Handling**: Add new error variants to `ConvexFxExecutorError`
4. **Configuration**: Extend configuration struct in `domain_agreement.rs`
