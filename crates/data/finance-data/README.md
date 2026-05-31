# finance-data

Provider-neutral financial market data types and indexing helpers.

This crate owns OHLCV bars, quotes, instruments, corporate actions, range
queries, candle downsampling, and market-data validation. Return and risk
statistics are delegated to `finance-statistics`.

Provider-specific clients are intentionally out of scope for the initial API.

## Package Surface

`finance-data` owns the standard runtime surface used by its CLI, server, WASM,
and app companions. The surface accepts inline `FinanceSeries` payloads and
exposes:

- `describe`
- `financeData.bounds`
- `financeData.barsInRange`
- `financeData.downsampleOhlcv`
- `financeData.returns`
- `financeData.riskSummary`

Transport packages:

- `finance-data-cli`
- `finance-data-server`
- `crates/bindings/finance-data-wasm`
- `packages/finance-data-wasm`
- `packages/finance-data-app`
