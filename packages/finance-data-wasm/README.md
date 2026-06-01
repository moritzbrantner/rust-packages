# @moritzbrantner/finance-data-wasm

WASM package for `finance-data`.

```bash
bun run --cwd packages/finance-data-wasm build
```

The package exposes the standard runtime surface operations and preserves the
`FinanceDataSeriesIndex` class API from the Rust WASM binding.
