# rust-hiring-assignement
Rust Hiring Assignement
Build a small market-maker simulator that continuously quotes and “executes” a $100,000 notional buy/sell program in ETH/USDC over ~10 minutes, using live prices from:
Binance WebSocket (book ticker): wss://stream.binance.com:9443/ws (e.g., ethusdc@bookTicker)
Jupiter HTTP: https://lite-api.jup.ag/price/v3?ids=7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs
Eth aggregator CowSwap: https://docs.cow.fi/category/apis

You will:

Stream/poll best bid/ask from each source.

Build a basic price model which ensures the offered buy price is never worse than the median of the above sources. Report a running PnL on shell assuming you got the trade x% of the time.

- Basic: Fix x at 70
- Advanced; Compute x based on how good your price is, say for example
  - If you offer median price your chances of booking the trade is 20%
  - If you offer at or above the best price your chances of booking the trade is 90%
  - Interpolate linearly between the two prices
