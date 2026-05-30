# finance-data

Provider-neutral financial market data types and indexing helpers.

This crate owns OHLCV bars, quotes, instruments, corporate actions, range
queries, candle downsampling, and market-data validation. Return and risk
statistics are delegated to `finance-statistics`.

Provider-specific clients are intentionally out of scope for the initial API.
