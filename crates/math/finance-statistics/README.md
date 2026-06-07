# finance-statistics

Finance-oriented statistics helpers for return series and portfolio analytics.
Generic scalar, pairwise, rolling, tail-risk, and drawdown algorithms are
implemented in `moritzbrantner-math-statistics`; this crate keeps the finance
vocabulary and market-specific wrappers.

## Highlights

- Simple and log return generation from price series
- Sample and population volatility, covariance, and correlation
- Annualized return, volatility, Sharpe, Sortino, beta, alpha, and tracking error
- Portfolio weight validation, weighted portfolio returns, and portfolio risk summaries
- Historical covariance matrices, portfolio variance, risk contribution, return
  contribution, and turnover
- Calmar ratio, Omega ratio, and drawdown duration helpers
- Historical VaR/CVaR and maximum drawdown helpers
- Rolling mean, standard deviation, and correlation windows

## Example

```rust,no_run
use finance_statistics::{annualized_volatility, max_drawdown, portfolio_risk_contribution, sharpe_ratio, simple_returns};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let returns = simple_returns(&[100.0, 101.0, 99.0, 103.0])?;
    let volatility = annualized_volatility(&returns, 252.0)?;
    let sharpe = sharpe_ratio(&returns, 0.0, 252.0)?;
    let drawdown = max_drawdown(&returns)?;
    let risk = portfolio_risk_contribution(
        &[vec![0.02, -0.01, 0.03], vec![0.01, 0.0, 0.02]],
        &[0.6, 0.4],
        252.0,
    )?;

    assert!(volatility >= 0.0);
    assert!(sharpe.is_finite());
    assert!(drawdown.depth >= 0.0);
    assert!(risk.volatility > 0.0);
    Ok(())
}
```

## Behavior

Portfolio risk attribution is historical covariance-based. Asset return series
must be aligned, finite, non-empty, and equal length; sample covariance requires
at least two observations per asset. Risk contribution annualizes covariance by
`periods_per_year` and rejects zero or negative portfolio variance before
dividing by volatility. These helpers are deterministic local portfolio
analytics, not optimizer-heavy portfolio construction or efficient-frontier
tools.

## Package surface

Primary workflow: `finance.returns`.

Workflow operations:

- `finance.returns`: Computes simple or log returns from strictly positive prices.
- `finance.risk`: Computes return, volatility, VaR/CVaR, ratios, and optional beta/alpha.
- `finance.drawdown`: Computes maximum drawdown details from a return series.
- `finance.rolling`: Computes rolling mean, sample volatility, and optional benchmark correlation.
- `finance.portfolio`: Computes weighted portfolio returns and compact portfolio risk metrics.
- `finance.performanceRatios`: Computes Calmar ratio, Omega ratio, and drawdown duration for a return series.
- `finance.riskContribution`: Computes annualized historical covariance-based portfolio volatility contributions.

Debug operations:

- `describe`: inspect package metadata and runtime support.

Runtime support: library, CLI, server, and WASM wrappers expose these operations.

Run the primary workflow through the CLI:

```bash
cargo run -p moritzbrantner-finance-statistics-cli -- run \
  --operation finance.returns \
  --json '{"method":"simple","prices":[100.0,110.0,99.0]}'
```

Successful responses use the shared package-surface shape with `operation`,
`title`, `message`, `summary`, and `result`. Default surface calls are
deterministic, local-first, and do not download models, write persistent files,
or execute external tools unless an operation explicitly documents native or
external-tool execution.

## Related crates

- `math-statistics`
- `numbers-core`
- `finance-data`
- `dense-data`
