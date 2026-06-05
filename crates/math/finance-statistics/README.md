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

## Related crates

- `math-statistics`
- `numbers-core`
- `finance-data`
- `dense-data`
