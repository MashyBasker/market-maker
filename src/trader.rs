use crate::aggregator::AggregatedPrices;
use rand::Rng;

#[derive(Debug, Clone)]
pub struct Trade {
    pub side: TradeSide,
    pub price: f64,
    pub amount_eth: f64,
    pub notional_usd: f64,
    pub pnl: f64,
    pub timestamp: i64,
    pub execution_prob: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TradeSide {
    Buy,
    Sell,
}

pub struct TradingEngine {
    notional_per_trade: f64,
    use_advanced_model: bool,
}

impl TradingEngine {
    pub fn new(notional_per_trade: f64, use_advanced_model: bool) -> Self {
        Self {
            notional_per_trade,
            use_advanced_model,
        }
    }

    /// Calculate execution probability based on our price vs market
    fn calculate_execution_probability(
        &self,
        our_price: f64,
        median_price: f64,
        best_price: f64,
        side: TradeSide,
    ) -> f64 {
        if !self.use_advanced_model {
            return 0.70; // Basic model: fixed 70%
        }

        // Advanced model: interpolate between 20% and 90%
        match side {
            TradeSide::Buy => {
                // For buying: higher price = more likely to get filled
                // Best price = highest bid in market
                if our_price >= best_price {
                    0.90
                } else if our_price <= median_price {
                    0.20
                } else {
                    // Linear interpolation
                    let range = best_price - median_price;
                    if range > 0.0 {
                        let position = (our_price - median_price) / range;
                        0.20 + position * 0.70
                    } else {
                        0.20
                    }
                }
            }
            TradeSide::Sell => {
                // For selling: lower price = more likely to get filled
                // Best price = lowest ask in market
                if our_price <= best_price {
                    0.90
                } else if our_price >= median_price {
                    0.20
                } else {
                    // Linear interpolation
                    let range = median_price - best_price;
                    if range > 0.0 {
                        let position = (median_price - our_price) / range;
                        0.20 + position * 0.70
                    } else {
                        0.20
                    }
                }
            }
        }
    }

    /// Calculate PnL for a trade
    /// For buys: we buy at our_price, mark-to-market at best_bid
    /// For sells: we sell at our_price, mark-to-market at best_ask
    fn calculate_pnl(&self, side: TradeSide, our_price: f64, market_price: f64, amount_eth: f64) -> f64 {
        match side {
            TradeSide::Buy => {
                // We bought ETH at our_price
                // Current value if we sell at market best bid
                (market_price - our_price) * amount_eth
            }
            TradeSide::Sell => {
                // We sold ETH at our_price
                // Current cost if we buy back at market best ask
                (our_price - market_price) * amount_eth
            }
        }
    }

    /// Attempt to execute a trade based on current market conditions
    pub fn attempt_trade(
        &self,
        prices: &AggregatedPrices,
        side: TradeSide,
    ) -> Option<Trade> {
        let median_quote = prices.median_quote()?;
        let best_quote = prices.best_quote()?;

        let (our_price, market_price) = match side {
            TradeSide::Buy => {
                // We quote our buy price at median bid (ensures never worse than median)
                // Mark-to-market at best bid in market
                (median_quote.bid, best_quote.bid)
            }
            TradeSide::Sell => {
                // We quote our sell price at median ask
                // Mark-to-market at best ask in market
                (median_quote.ask, best_quote.ask)
            }
        };

        let amount_eth = self.notional_per_trade / our_price;

        // Calculate execution probability
        let median_price = match side {
            TradeSide::Buy => median_quote.bid,
            TradeSide::Sell => median_quote.ask,
        };
        let best_price = match side {
            TradeSide::Buy => best_quote.bid,
            TradeSide::Sell => best_quote.ask,
        };

        let execution_prob = self.calculate_execution_probability(
            our_price,
            median_price,
            best_price,
            side,
        );

        // Simulate execution
        let mut rng = rand::rng();
        let executed = rng.random::<f64>() < execution_prob;

        if executed {
            let pnl = self.calculate_pnl(side, our_price, market_price, amount_eth);
            
            Some(Trade {
                side,
                price: our_price,
                amount_eth,
                notional_usd: self.notional_per_trade,
                pnl,
                timestamp: chrono::Utc::now().timestamp_millis(),
                execution_prob,
            })
        } else {
            None
        }
    }

    /// Get market summary for display
    pub fn get_market_summary(&self, prices: &AggregatedPrices) -> Option<MarketSummary> {
        let median_quote = prices.median_quote()?;
        let best_quote = prices.best_quote()?;
        let median_mid = prices.median_mid()?;

        Some(MarketSummary {
            median_bid: median_quote.bid,
            median_ask: median_quote.ask,
            median_mid,
            best_bid: best_quote.bid,
            best_ask: best_quote.ask,
            spread_bps: ((median_quote.ask - median_quote.bid) / median_mid * 10000.0),
        })
    }
}

#[derive(Debug)]
pub struct MarketSummary {
    pub median_bid: f64,
    pub median_ask: f64,
    pub median_mid: f64,
    pub best_bid: f64,
    pub best_ask: f64,
    pub spread_bps: f64,
}