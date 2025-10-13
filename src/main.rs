use anyhow::Result;
use market_maker_simulator::{PnLTracker, PriceAggregator, TradeSide, TradingEngine};
use std::time::Duration;
use tokio::time::{interval, sleep};

const NOTIONAL_PER_TRADE: f64 = 100_000.0;
const SIMULATION_DURATION_SECS: u64 = 600; // 10 minutes
const TRADE_INTERVAL_SECS: u64 = 5; // Execute trades every 5 seconds

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let use_advanced_model = args.iter().any(|arg| arg == "--advanced");

    println!("\n╔════════════════════════════════════════════════════════════════════╗");
    println!("║              MARKET MAKER SIMULATOR - ETH/USDC                     ║");
    println!("╠════════════════════════════════════════════════════════════════════╣");
    println!("║ Notional per Trade:    ${}                                  ║", NOTIONAL_PER_TRADE.separated_string());
    println!("║ Simulation Duration:   {} minutes                                 ║", SIMULATION_DURATION_SECS / 60);
    println!("║ Trade Interval:        {} seconds                                 ║", TRADE_INTERVAL_SECS);
    println!("║ Execution Model:       {}                              ║", 
        if use_advanced_model { "ADVANCED (20%-90%)" } else { "BASIC (70% fixed) " });
    println!("╚════════════════════════════════════════════════════════════════════╝\n");

    // Initialize components
    println!("[INIT] Starting price aggregator...");
    let aggregator = PriceAggregator::new();
    aggregator.start().await?;

    println!("[INIT] Waiting 10 seconds for initial price data...");
    sleep(Duration::from_secs(10)).await;

    println!("[INIT] Initializing trading engine and PnL tracker...");
    let trading_engine = TradingEngine::new(NOTIONAL_PER_TRADE, use_advanced_model);
    let pnl_tracker = PnLTracker::new();

    println!("[START] Beginning market making session...\n");

    // Trading loop
    let mut trade_interval = interval(Duration::from_secs(TRADE_INTERVAL_SECS));
    let start_time = std::time::Instant::now();
    let mut cycle_count = 0;

    loop {
        trade_interval.tick().await;

        let elapsed = start_time.elapsed().as_secs();
        if elapsed >= SIMULATION_DURATION_SECS {
            break;
        }

        cycle_count += 1;
        let remaining = SIMULATION_DURATION_SECS - elapsed;

        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Cycle #{} │ Elapsed: {}s │ Remaining: {}s", cycle_count, elapsed, remaining);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        let prices = aggregator.get_prices().await;

        if let Some(summary) = trading_engine.get_market_summary(&prices) {
            println!("[MARKET] Median: ${:.2} │ Spread: {:.1} bps │ Best Bid: ${:.2} │ Best Ask: ${:.2}",
                summary.median_mid,
                summary.spread_bps,
                summary.best_bid,
                summary.best_ask
            );
        }

        // Display source status
        print!("[SOURCES] ");
        if prices.binance.is_some() {
            print!("Binance ✓ │ ");
        } else {
            print!("Binance ✗ │ ");
        }
        if prices.jupiter.is_some() {
            print!("Jupiter ✓ │ ");
        } else {
            print!("Jupiter ✗ │ ");
        }
        if prices.cowswap.is_some() {
            println!("CowSwap ✓");
        } else {
            println!("CowSwap ✗");
        }

        // Attempt buy trade
        if let Some(trade) = trading_engine.attempt_trade(&prices, TradeSide::Buy) {
            pnl_tracker.print_trade(&trade).await;
            pnl_tracker.record_trade(trade).await;
        } else {
            println!("[SKIP] Buy trade not executed (probability miss)");
        }

        // Attempt sell trade
        if let Some(trade) = trading_engine.attempt_trade(&prices, TradeSide::Sell) {
            pnl_tracker.print_trade(&trade).await;
            pnl_tracker.record_trade(trade).await;
        } else {
            println!("[SKIP] Sell trade not executed (probability miss)");
        }

        // Show current stats every 10 cycles
        if cycle_count % 10 == 0 {
            let stats = pnl_tracker.get_stats().await;
            println!("\n[STATS] Running Total: {} trades │ PnL: ${:.2} │ Avg per trade: ${:.2}",
                stats.total_trades,
                stats.total_pnl,
                stats.avg_pnl_per_trade()
            );
        }
    }

    // Final summary
    println!("\n");
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║                     SIMULATION COMPLETE                            ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    
    pnl_tracker.print_summary().await;

    // Show last few trades
    println!("Last 5 Trades:");
    println!("─────────────────────────────────────────────────────────────────────");
    let recent_trades = pnl_tracker.get_recent_trades(5).await;
    for trade in recent_trades.iter().rev() {
        let side = match trade.side {
            TradeSide::Buy => "BUY ",
            TradeSide::Sell => "SELL",
        };
        println!("  {} │ ${:.2} │ {:.4} ETH │ PnL: ${:.2}",
            side, trade.price, trade.amount_eth, trade.pnl);
    }
    println!("─────────────────────────────────────────────────────────────────────\n");

    Ok(())
}

// Helper trait for formatting numbers with separators
trait FormattedNumber {
    fn separated_string(&self) -> String;
}

impl FormattedNumber for f64 {
    fn separated_string(&self) -> String {
        let s = format!("{:.0}", self);
        let mut result = String::new();
        let chars: Vec<char> = s.chars().collect();
        
        for (i, c) in chars.iter().enumerate() {
            if i > 0 && (chars.len() - i) % 3 == 0 {
                result.push(',');
            }
            result.push(*c);
        }
        result
    }
}