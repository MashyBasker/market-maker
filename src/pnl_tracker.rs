use crate::trader::{Trade, TradeSide};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct PnLStats {
    pub total_pnl: f64,
    pub total_trades: u32,
    pub buy_trades: u32,
    pub sell_trades: u32,
    pub buy_pnl: f64,
    pub sell_pnl: f64,
    pub total_notional: f64,
    pub avg_execution_prob: f64,
}

impl PnLStats {
    pub fn new() -> Self {
        Self {
            total_pnl: 0.0,
            total_trades: 0,
            buy_trades: 0,
            sell_trades: 0,
            buy_pnl: 0.0,
            sell_pnl: 0.0,
            total_notional: 0.0,
            avg_execution_prob: 0.0,
        }
    }

    pub fn avg_pnl_per_trade(&self) -> f64 {
        if self.total_trades > 0 {
            self.total_pnl / self.total_trades as f64
        } else {
            0.0
        }
    }

    pub fn pnl_per_notional_bps(&self) -> f64 {
        if self.total_notional > 0.0 {
            (self.total_pnl / self.total_notional) * 10000.0
        } else {
            0.0
        }
    }
}

pub struct PnLTracker {
    stats: Arc<RwLock<PnLStats>>,
    trades: Arc<RwLock<Vec<Trade>>>,
}

impl PnLTracker {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(PnLStats::new())),
            trades: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn record_trade(&self, trade: Trade) {
        let mut stats = self.stats.write().await;
        let mut trades = self.trades.write().await;

        stats.total_pnl += trade.pnl;
        stats.total_trades += 1;
        stats.total_notional += trade.notional_usd;
        
        match trade.side {
            TradeSide::Buy => {
                stats.buy_trades += 1;
                stats.buy_pnl += trade.pnl;
            }
            TradeSide::Sell => {
                stats.sell_trades += 1;
                stats.sell_pnl += trade.pnl;
            }
        }

        // Update rolling average execution probability
        stats.avg_execution_prob = 
            (stats.avg_execution_prob * (stats.total_trades - 1) as f64 + trade.execution_prob) 
            / stats.total_trades as f64;

        trades.push(trade);
    }

    pub async fn get_stats(&self) -> PnLStats {
        self.stats.read().await.clone()
    }

    pub async fn get_recent_trades(&self, n: usize) -> Vec<Trade> {
        let trades = self.trades.read().await;
        let start = trades.len().saturating_sub(n);
        trades[start..].to_vec()
    }

    pub async fn print_summary(&self) {
        let stats = self.get_stats().await;
        
        println!("\n╔════════════════════════════════════════════════════════════════════╗");
        println!("║                    TRADING SESSION SUMMARY                         ║");
        println!("╠════════════════════════════════════════════════════════════════════╣");
        println!("║ Total Trades:          {:>8}                                    ║", stats.total_trades);
        println!("║   - Buy Trades:        {:>8}                                    ║", stats.buy_trades);
        println!("║   - Sell Trades:       {:>8}                                    ║", stats.sell_trades);
        println!("║                                                                    ║");
        println!("║ Total PnL:             ${:>12.2}                             ║", stats.total_pnl);
        println!("║   - Buy PnL:           ${:>12.2}                             ║", stats.buy_pnl);
        println!("║   - Sell PnL:          ${:>12.2}                             ║", stats.sell_pnl);
        println!("║                                                                    ║");
        println!("║ Avg PnL per Trade:     ${:>12.2}                             ║", stats.avg_pnl_per_trade());
        println!("║ Total Notional:        ${:>12.2}                             ║", stats.total_notional);
        println!("║ PnL / Notional:        {:>8.2} bps                            ║", stats.pnl_per_notional_bps());
        println!("║ Avg Execution Prob:    {:>7.1}%                                 ║", stats.avg_execution_prob * 100.0);
        println!("╚════════════════════════════════════════════════════════════════════╝\n");
    }

    pub async fn print_trade(&self, trade: &Trade) {
        let stats = self.get_stats().await;
        let side_str = match trade.side {
            TradeSide::Buy => "BUY ",
            TradeSide::Sell => "SELL",
        };

        println!(
            "[TRADE] {} │ Price: ${:>8.2} │ Amount: {:>8.4} ETH │ Prob: {:>5.1}% │ PnL: ${:>8.2} │ Total PnL: ${:>10.2}",
            side_str,
            trade.price,
            trade.amount_eth,
            trade.execution_prob * 100.0,
            trade.pnl,
            stats.total_pnl
        );
    }
}