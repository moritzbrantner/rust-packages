# math-statistics-app

React, TypeScript, TailwindCSS, Bun, and oxfmt frontend for `math-statistics`.
The workbench keeps `stats.series.describe` as the default and exposes
precision-aware f64 matrix examples for normalization, covariance, PCA, and OLS.

Run the server:

```bash
cargo run -p math-statistics-server -- --addr 127.0.0.1:3000
```

Run the app:

```bash
bun run --cwd packages/math-statistics-app dev
```
