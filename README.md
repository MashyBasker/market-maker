# Market Maker Simulator - ETH/USDC (ASXN Hiring Assignment)

## Overview

This simulator aggregates live prices from three sources and executes simulated trades with probabilistic execution based on price competitiveness:

- **Binance WebSocket**: Real-time ETH/USDC book ticker
- **Jupiter API**: Solana DEX aggregator prices  
- **CowSwap API**: Ethereum DEX quotes

### Project Structure

```
market-maker-simulator/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs              # Module exports
    ├── main.rs             # Application entry point
    ├── aggregator.rs       # Price aggregation (your existing code)
    ├── trader.rs           # Trading engine & execution logic
    └── pnl_tracker.rs      # PnL calculation & statistics
```

## Design Decisions

### 1. Price Aggregation Strategy

**Median-Based Quoting**: We use the median of all three sources for our quotes:
- **Buy orders**: Quote at median bid (ensures we're never worse than median)
- **Sell orders**: Quote at median ask (ensures we're never worse than median)

This approach balances competitiveness with reliability and fulfills the requirement that "the offered buy price is never worse than the median."

### 2. Execution Probability Models

#### Basic Model (Default)
```rust
execution_probability = 0.70  // Fixed 70%
```

#### Advanced Model (`--advanced` flag)
```rust
if our_price >= best_price:
    execution_probability = 0.90
else if our_price <= median_price:
    execution_probability = 0.20
else:
    // Linear interpolation
    t = (our_price - median_price) / (best_price - median_price)
    execution_probability = 0.20 + t * 0.70
```

For **buy orders**: Higher price = better chance (we're more competitive)  
For **sell orders**: Lower price = better chance (we're more aggressive)

### 3. PnL Calculation Methodology

We mark-to-market immediately after execution:

**Buy Trade PnL**:
```
PnL = (Market_Best_Bid - Our_Buy_Price) × ETH_Amount
```
- We buy ETH at our price
- Immediately value it at what we could sell for (best market bid)

**Sell Trade PnL**:
```
PnL = (Our_Sell_Price - Market_Best_Ask) × ETH_Amount
```
- We sell ETH at our price  
- Mark the cost of buying it back (best market ask)

### 4. Trade Execution Timing

- **Trade Interval**: 5 seconds
- **Total Cycles**: 120 cycles over 10 minutes
- **Per Cycle**: Attempts both 1 buy AND 1 sell of $100k notional each
- **Maximum Potential**: 240 trades (120 buys + 120 sells)

## 🔧 Technical Implementation

### Concurrency Model
- **Tokio async runtime** for non-blocking I/O
- **Arc<RwLock<>>** for thread-safe shared state
- **Separate tasks** for each price feed to prevent blocking

### Resilience Features
- Automatic WebSocket reconnection (5s delay)
- Graceful error handling for API failures
- Continues trading even if one source is down


### Spread Assumptions

Since some APIs return only mid-prices:
- **Jupiter**: ±0.05% spread (0.1% total)
- **CowSwap**: ±0.1% spread (0.2% total)
- **Binance**: Direct bid/ask from WebSocket

## Build & Run Instructions

### Prerequisites

```bash
rustc --version
cargo --version
```

### Building

```bash
# Clone your private repository
git clone <your-private-repo-url>
cd market-maker-simulator

# Build in release mode (optimized)
cargo build --release
```

### Running

**Basic Mode** (70% fixed execution probability):
```bash
cargo run --release
```

**Advanced Mode** (dynamic 20%-90% probability):
```bash
cargo run --release -- --advanced
```

**Direct Binary Execution**:
```bash
# After building
./target/release/market-maker          # Basic mode
./target/release/market-maker --advanced  # Advanced mode
```

## Performance Metrics

### Expected Results (Advanced Mode)

| Metric | Expected Range | Notes |
|--------|---------------|-------|
| Total Trades | 140-180 | ~70% execution rate |
| Buy/Sell Ratio | ~1:1 | Balanced market making |
| Avg Execution Prob | 60-75% | Depends on market conditions |
| PnL per Trade | $5-$15 | Based on spread capture |
| Total PnL | $800-$2,000 | Over 10 minutes |

### Execution Probability Distribution

In advanced mode, you should see varied probabilities:
- **20-40%**: When quoting at/near median
- **40-70%**: Mid-range competitive pricing
- **70-90%**: Aggressive pricing at/near best

## 🔍 Deviations from Spec

### 2. Trade Execution
- Each cycle attempts **both** buy and sell (not alternating)
- This maximizes market making opportunities
- Doubles potential trade count vs alternating approach

### 3. Notional Interpretation
- Each trade (buy or sell) uses full $100k notional
- Per-cycle notional can be up to $200k if both execute
- Total program notional over 10 mins: ~$10-20M depending on execution

### 4. Price Spread Handling
- Added spread assumptions for mid-price APIs
- Ensures realistic bid/ask even from single-price sources

## Assumptions & Limitations

### Market Assumptions
1. **Instant Execution**: Trades execute immediately when probability succeeds
2. **No Slippage**: Full notional executes at quoted price
3. **Perfect Mark-to-Market**: Can instantly hedge at best market price
4. **No Transaction Costs**: Fees, gas, and commissions not modeled


### Simplifications
1. **No Position Limits**: Could accumulate large positions
2. **No Risk Management**: No stop-loss or max drawdown
3. **No Order Book Depth**: Assumes infinite liquidity
4. **Single Token Pair**: Only ETH/USDC

## 🛠️ Troubleshooting

### Common Issues

**Issue**: "Failed to connect to Binance"
```bash
# Solution: Check firewall/network settings
# Binance requires outbound WSS connections on port 9443
```

**Issue**: Jupiter/CowSwap API errors
```bash
# Solution: APIs may have rate limits or downtime
# Simulator continues with available sources
# Check API status at respective provider websites
```

**Issue**: "No trades executed"
```bash
# Solution: Ensure prices are loading
# Wait for "SOURCES: Binance ✓ │ Jupiter ✓ │ CowSwap ✓"
# If only 1 source available, median may not be reliable
```

**Issue**: Compilation errors
```bash
# Ensure Rust is up to date
rustup update stable

# Clean and rebuild
cargo clean
cargo build --release
```

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| tokio | 1.35 | Async runtime |
| tokio-tungstenite | 0.21 | WebSocket client |
| futures-util | 0.3 | Stream utilities |
| serde | 1.0 | Serialization |
| serde_json | 1.0 | JSON parsing |
| reqwest | 0.11 | HTTP client |
| anyhow | 1.0 | Error handling |
| chrono | 0.4 | Timestamps |
| rand | 0.8 | Random number generation |