# @mb-rust/dense-data-wasm

WASM package for `dense-data`.

```bash
bun run --cwd packages/dense-data-wasm build
```

## Numeric series

`NumericSeriesIndex` exposes repeated-query helpers for display-sized data:

- `getChartSeries({ xDomain, targetBinCount, valueMode, includeEmptyBins })`
- `getHistogram({ bucketCount, xDomain, valueDomain, valueAccessor })`
- `getHeatmap({ xBinCount, xDomain, yBinCount, yDomain, valueAccessor })`

Domains are normalized when passed in reverse order. Histogram buckets and
heatmap cells include empty entries by default; chart series includes empty bins
only when requested.
