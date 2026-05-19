# finance-statistics

Finance-oriented statistics helpers for return series and portfolio analytics.

## Highlights

- Simple and log return generation from price series
- Sample and population volatility, covariance, and correlation
- Annualized return, volatility, Sharpe, Sortino, beta, alpha, and tracking error
- Historical VaR/CVaR and maximum drawdown helpers
- Rolling mean, standard deviation, and correlation windows

## Example

```rust,no_run
use finance_statistics::{annualized_volatility, max_drawdown, sharpe_ratio, simple_returns};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let returns = simple_returns(&[100.0, 101.0, 99.0, 103.0])?;
    let volatility = annualized_volatility(&returns, 252.0)?;
    let sharpe = sharpe_ratio(&returns, 0.0, 252.0)?;
    let drawdown = max_drawdown(&returns)?;

    assert!(volatility >= 0.0);
    assert!(sharpe.is_finite());
    assert!(drawdown.depth >= 0.0);
    Ok(())
}
```

## Related crates

- `numbers-core`
- `math-statistics`
- `dense-data`
