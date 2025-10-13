#![allow(unused)]
use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio::{sync::RwLock, time::interval};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Debug, Clone, Copy)]
pub struct Quote {
    pub bid: f64,
    pub ask: f64,
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct AggregatedPrices {
    pub binance: Option<Quote>,
    pub jupiter: Option<Quote>,
    pub cowswap: Option<Quote>,
}

impl AggregatedPrices {
    pub fn median_quote(&self) -> Option<Quote> {
        let mut bids = Vec::new();
        let mut asks = Vec::new();
        let mut timestamps = Vec::new();

        if let Some(q) = self.binance {
            bids.push(q.bid);
            asks.push(q.ask);
            timestamps.push(q.timestamp);
        }
        if let Some(q) = self.jupiter {
            bids.push(q.bid);
            asks.push(q.ask);
            timestamps.push(q.timestamp);
        }
        if let Some(q) = self.cowswap {
            bids.push(q.bid);
            asks.push(q.ask);
            timestamps.push(q.timestamp);
        }

        if bids.is_empty() {
            return None;
        }

        bids.sort_by(|a, b| a.partial_cmp(b).unwrap());
        asks.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let latest_timestamp = *timestamps.iter().max().unwrap();

        let median_bid = bids[bids.len() / 2];
        let median_ask = asks[asks.len() / 2];

        Some(Quote {
            bid: median_bid,
            ask: median_ask,
            timestamp: latest_timestamp,
        })
    }

    pub fn best_quote(&self) -> Option<Quote> {
        let mut best_bid = None;
        let mut best_ask = None;
        let mut latest_timestamp = 0;

        for quote in [self.binance, self.jupiter, self.cowswap]
            .iter()
            .filter_map(|&q| q)
        {
            best_bid = Some(best_bid.map_or(quote.bid, |b: f64| b.max(quote.bid)));
            best_ask = Some(best_ask.map_or(quote.ask, |a: f64| a.min(quote.ask)));
            latest_timestamp = latest_timestamp.max(quote.timestamp);
        }

        match (best_bid, best_ask) {
            (Some(bid), Some(ask)) => Some(Quote {
                bid: bid,
                ask: ask,
                timestamp: latest_timestamp,
            }),
            _ => None,
        }
    }

    pub fn median_mid(&self) -> Option<f64> {
        let mut mids = Vec::new();

        if let Some(q) = self.binance {
            mids.push((q.bid + q.ask) / 2.0);
        }
        if let Some(q) = self.jupiter {
            mids.push((q.bid + q.ask) / 2.0);
        }
        if let Some(q) = self.cowswap { 
            mids.push((q.bid + q.ask) / 2.0);
        }

        if mids.is_empty() {
            return None;
        }

        mids.sort_by(|a, b| a.partial_cmp(b).unwrap());
        Some(mids[mids.len() / 2])
    }
}

#[derive(Debug, Deserialize)]
pub struct BinanceBookTicker {
    #[serde(rename = "b")]
    bid_price: String,
    #[serde(rename = "a")]
    ask_price: String,
}

#[derive(Debug, Deserialize)]
pub struct JupiterResponse {
    data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct CowSwapQuote {
    quote: CowSwapQuoteData,
}

#[derive(Debug, Deserialize)]
pub struct CowSwapQuoteData {
    #[serde(rename = "buyAmount")]
    buy_amount: String,
    #[serde(rename = "sellAmount")]
    sell_amount: String,
}

pub struct PriceAggregator {
    prices: Arc<RwLock<AggregatedPrices>>,
}

impl PriceAggregator {
    pub fn new() -> Self {
        Self {
            prices: Arc::new(RwLock::new(AggregatedPrices {
                binance: None,
                jupiter: None,
                cowswap: None,
            })),
        }
    }

    pub async fn start(&self) -> Result<()> {
        let prices_binance = Arc::clone(&self.prices);
        let prices_jupiter = Arc::clone(&self.prices);
        let prices_cowswap = Arc::clone(&self.prices);

        tokio::spawn(async move {
            if let Err(e) = Self::binance_stream(prices_binance).await {
                eprintln!("[ERROR] Binance stream error: {}", e);
            }
        });

        tokio::spawn(async move {
            if let Err(e) = Self::jupiter_poll(prices_jupiter).await {
                eprintln!("[ERROR] Jupiter poll error: {}", e);
            }
        });

        tokio::spawn(async move {
            if let Err(e) = Self::cowswap_poll(prices_cowswap).await {
                eprintln!("[ERROR] Cowswap poll error: {}", e);
            }
        });

        Ok(())
    }

    async fn binance_stream(prices: Arc<RwLock<AggregatedPrices>>) -> Result<()> {
        let url = "wss://stream.binance.com:9443/ws/ethusdc@bookTicker";

        loop {
            match connect_async(url).await {
                Ok((ws_stream, _)) => {
                    println!("Connected to Binance WebSocket");
                    let (mut _write, mut read) = ws_stream.split();

                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if let Ok(ticker) = serde_json::from_str::<BinanceBookTicker>(&text)
                                {
                                    if let (Ok(bid), Ok(ask)) = (
                                        ticker.bid_price.parse::<f64>(),
                                        ticker.ask_price.parse::<f64>(),
                                    ) {
                                        let quote = Quote {
                                            bid,
                                            ask,
                                            timestamp: chrono::Utc::now().timestamp_millis(),
                                        };
                                        prices.write().await.binance = Some(quote);
                                    }
                                }
                            }
                            Ok(Message::Binary(_)) => {}
                            Ok(Message::Ping(_)) => {}
                            Ok(Message::Pong(_)) => {}
                            Ok(Message::Frame(_)) => {}
                            Ok(Message::Close(_)) => {
                                println!("[INFO] Binance websocket closed, reconnecting...");
                                break;
                            }
                            Err(e) => {
                                eprintln!("[ERROR] Binance WebSocket error: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[ERROR] Failed to connect to Binance: {}", e);
                }
            }

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    async fn jupiter_poll(prices: Arc<RwLock<AggregatedPrices>>) -> Result<()> {
        let client = reqwest::Client::new();
        let mut interval = interval(Duration::from_secs(2));
        let url =
            "https://lite-api.jup.ag/price/v3?ids=7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs";
        let eth_token_id = "7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs";

        loop {
            interval.tick().await;

            match client.get(url).send().await {
                Ok(response) => {
                    if let Ok(data) = response.json::<serde_json::Value>().await {
                        if let Some(price) = data[eth_token_id]["usdPrice"].as_f64() {
                            let spread = price * 0.0005;
                            let quote = Quote {
                                bid: price - spread,
                                ask: price + spread,
                                timestamp: chrono::Utc::now().timestamp_millis(),
                            };
                            prices.write().await.jupiter = Some(quote);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[ERROR] Jupiter fetch error: {}", e);
                }
            }
        }
    }

    async fn cowswap_poll(prices: Arc<RwLock<AggregatedPrices>>) -> Result<()> {
        let client = reqwest::Client::new();
        let mut interval = interval(Duration::from_secs(3));

        let eth_address = "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE";
        let usdc_address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";

        loop {
            interval.tick().await;

            let buy_url = "https://api.cow.fi/mainnet/api/v1/quote";
            let buy_params = serde_json::json!({
                "sellToken": usdc_address,
                "buyToken": eth_address,
                "sellAmountBeforeFee": "1000000000", // 1000 USDC (6 decimals)
                "kind": "sell",
                "from": "0x0000000000000000000000000000000000000000"
            });

            match client.post(buy_url).json(&buy_params).send().await {
                Ok(response) => {
                    if let Ok(data) = response.json::<serde_json::Value>().await {
                        if let (Some(sell_amount), Some(buy_amount)) = (
                            data["quote"]["sellAmount"].as_str(),
                            data["quote"]["buyAmount"].as_str(),
                        ) {
                            if let (Ok(sell), Ok(buy)) =
                                (sell_amount.parse::<f64>(), buy_amount.parse::<f64>())
                            {
                                // Calculate price with proper decimals
                                let price = (sell / 1e6) / (buy / 1e18);
                                let spread = price * 0.001; // 0.1% spread estimate

                                let quote = Quote {
                                    bid: price - spread,
                                    ask: price + spread,
                                    timestamp: chrono::Utc::now().timestamp_millis(),
                                };
                                prices.write().await.cowswap = Some(quote);
                            }
                        }
                    }
                }
                Err(e) => eprintln!("[ERROR] CowSwap fetch error: {}", e),
            }
        }
    }

    pub async fn get_prices(&self) -> AggregatedPrices {
        self.prices.read().await.clone()
    }
}
