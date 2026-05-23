#![doc = include_str!("../README.md")]

pub mod surface;
use numbers_core::quantile;
use video_analysis_core::{DetectError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variance normalization to use for second-moment statistics.
pub enum VarianceMode {
    /// Divide by `n`.
    Population,
    /// Divide by `n - 1`.
    Sample,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Historical value-at-risk summary using return quantiles.
pub struct HistoricalRisk {
    /// Confidence level in the inclusive range `(0, 1)`.
    pub confidence: f64,
    /// Positive loss amount at the requested confidence.
    pub value_at_risk: f64,
    /// Positive average tail loss at or below the VaR return threshold.
    pub conditional_value_at_risk: f64,
    /// Return threshold used to derive the loss values.
    pub return_threshold: f64,
    /// Number of return observations used.
    pub observations: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Maximum drawdown details for a compounded return path.
pub struct Drawdown {
    /// Positive drawdown depth as a fraction of prior peak equity.
    pub depth: f64,
    /// Index of the equity peak before the drawdown.
    pub peak_index: usize,
    /// Index of the equity trough.
    pub trough_index: usize,
    /// First index where equity recovered to the prior peak, if any.
    pub recovery_index: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// CAPM-style beta and alpha summary.
pub struct BetaAlpha {
    /// Asset beta versus benchmark excess returns.
    pub beta: f64,
    /// Per-period alpha relative to the benchmark and risk-free rate.
    pub alpha: f64,
    /// Correlation between asset and benchmark returns.
    pub correlation: f64,
    /// Mean asset return per period.
    pub asset_mean: f64,
    /// Mean benchmark return per period.
    pub benchmark_mean: f64,
}

/// Returns simple period returns from strictly positive prices.
pub fn simple_returns(prices: &[f64]) -> Result<Vec<f64>> {
    validate_prices(prices)?;
    Ok(prices
        .windows(2)
        .map(|window| window[1] / window[0] - 1.0)
        .collect())
}

/// Returns natural-log period returns from strictly positive prices.
pub fn log_returns(prices: &[f64]) -> Result<Vec<f64>> {
    validate_prices(prices)?;
    Ok(prices
        .windows(2)
        .map(|window| (window[1] / window[0]).ln())
        .collect())
}

/// Returns the arithmetic mean of a return series.
pub fn mean_return(returns: &[f64]) -> Result<f64> {
    validate_returns(returns)?;
    Ok(returns.iter().sum::<f64>() / returns.len() as f64)
}

/// Returns variance for a return series.
pub fn variance(returns: &[f64], mode: VarianceMode) -> Result<f64> {
    validate_returns(returns)?;
    let denom = variance_denominator(returns.len(), mode)?;
    let mean = mean_return(returns)?;
    Ok(returns
        .iter()
        .map(|value| {
            let centered = value - mean;
            centered * centered
        })
        .sum::<f64>()
        / denom)
}

/// Returns standard deviation for a return series.
pub fn std_dev(returns: &[f64], mode: VarianceMode) -> Result<f64> {
    Ok(variance(returns, mode)?.sqrt())
}

/// Returns compounded cumulative return.
pub fn cumulative_return(returns: &[f64]) -> Result<f64> {
    validate_compoundable_returns(returns)?;
    Ok(returns
        .iter()
        .fold(1.0, |equity, value| equity * (1.0 + value))
        - 1.0)
}

/// Returns annualized compounded return.
pub fn annualized_return(returns: &[f64], periods_per_year: f64) -> Result<f64> {
    validate_periods_per_year(periods_per_year)?;
    let cumulative = 1.0 + cumulative_return(returns)?;
    if cumulative <= 0.0 {
        return Ok(-1.0);
    }
    Ok(cumulative.powf(periods_per_year / returns.len() as f64) - 1.0)
}

/// Returns annualized sample volatility.
pub fn annualized_volatility(returns: &[f64], periods_per_year: f64) -> Result<f64> {
    validate_periods_per_year(periods_per_year)?;
    Ok(std_dev(returns, VarianceMode::Sample)? * periods_per_year.sqrt())
}

/// Returns annualized downside deviation around a target return.
pub fn downside_deviation(
    returns: &[f64],
    target_return_per_period: f64,
    periods_per_year: f64,
) -> Result<f64> {
    validate_returns(returns)?;
    validate_periods_per_year(periods_per_year)?;
    if !target_return_per_period.is_finite() {
        return Err(invalid_argument("target return must be finite"));
    }
    let downside = returns
        .iter()
        .map(|value| (value - target_return_per_period).min(0.0))
        .map(|value| value * value)
        .sum::<f64>()
        / returns.len() as f64;
    Ok(downside.sqrt() * periods_per_year.sqrt())
}

/// Returns annualized Sharpe ratio using per-period risk-free return.
pub fn sharpe_ratio(
    returns: &[f64],
    risk_free_return_per_period: f64,
    periods_per_year: f64,
) -> Result<f64> {
    validate_returns(returns)?;
    validate_periods_per_year(periods_per_year)?;
    if !risk_free_return_per_period.is_finite() {
        return Err(invalid_argument("risk-free return must be finite"));
    }
    let excess = returns
        .iter()
        .map(|value| value - risk_free_return_per_period)
        .collect::<Vec<_>>();
    let volatility = std_dev(&excess, VarianceMode::Sample)?;
    if volatility <= f64::EPSILON {
        return Err(invalid_argument(
            "Sharpe ratio requires non-zero volatility",
        ));
    }
    Ok(mean_return(&excess)? / volatility * periods_per_year.sqrt())
}

/// Returns annualized Sortino ratio using per-period target return.
pub fn sortino_ratio(
    returns: &[f64],
    target_return_per_period: f64,
    periods_per_year: f64,
) -> Result<f64> {
    validate_returns(returns)?;
    validate_periods_per_year(periods_per_year)?;
    let downside = downside_deviation(returns, target_return_per_period, 1.0)?;
    if downside <= f64::EPSILON {
        return Err(invalid_argument(
            "Sortino ratio requires non-zero downside deviation",
        ));
    }
    Ok((mean_return(returns)? - target_return_per_period) / downside * periods_per_year.sqrt())
}

/// Returns sample covariance between two equally sized return series.
pub fn covariance(left: &[f64], right: &[f64], mode: VarianceMode) -> Result<f64> {
    validate_pair(left, right)?;
    let denom = variance_denominator(left.len(), mode)?;
    let left_mean = mean_return(left)?;
    let right_mean = mean_return(right)?;
    Ok(left
        .iter()
        .zip(right)
        .map(|(left, right)| (left - left_mean) * (right - right_mean))
        .sum::<f64>()
        / denom)
}

/// Returns Pearson correlation between two equally sized return series.
pub fn correlation(left: &[f64], right: &[f64]) -> Result<f64> {
    let cov = covariance(left, right, VarianceMode::Sample)?;
    let left_std = std_dev(left, VarianceMode::Sample)?;
    let right_std = std_dev(right, VarianceMode::Sample)?;
    let denom = left_std * right_std;
    if denom <= f64::EPSILON {
        return Err(invalid_argument("correlation requires non-zero variance"));
    }
    Ok((cov / denom).clamp(-1.0, 1.0))
}

/// Returns CAPM-style beta and alpha using per-period risk-free return.
pub fn beta_alpha(
    asset_returns: &[f64],
    benchmark_returns: &[f64],
    risk_free_return_per_period: f64,
) -> Result<BetaAlpha> {
    validate_pair(asset_returns, benchmark_returns)?;
    if !risk_free_return_per_period.is_finite() {
        return Err(invalid_argument("risk-free return must be finite"));
    }
    let asset_excess = asset_returns
        .iter()
        .map(|value| value - risk_free_return_per_period)
        .collect::<Vec<_>>();
    let benchmark_excess = benchmark_returns
        .iter()
        .map(|value| value - risk_free_return_per_period)
        .collect::<Vec<_>>();
    let benchmark_variance = variance(&benchmark_excess, VarianceMode::Sample)?;
    if benchmark_variance <= f64::EPSILON {
        return Err(invalid_argument(
            "beta requires non-zero benchmark variance",
        ));
    }
    let beta =
        covariance(&asset_excess, &benchmark_excess, VarianceMode::Sample)? / benchmark_variance;
    let asset_mean = mean_return(asset_returns)?;
    let benchmark_mean = mean_return(benchmark_returns)?;
    Ok(BetaAlpha {
        beta,
        alpha: asset_mean
            - risk_free_return_per_period
            - beta * (benchmark_mean - risk_free_return_per_period),
        correlation: correlation(asset_returns, benchmark_returns)?,
        asset_mean,
        benchmark_mean,
    })
}

/// Returns maximum drawdown for a compounded return path.
pub fn max_drawdown(returns: &[f64]) -> Result<Drawdown> {
    validate_compoundable_returns(returns)?;
    let mut equity = 1.0;
    let mut peak = 1.0;
    let mut peak_index = 0;
    let mut max_drawdown = Drawdown {
        depth: 0.0,
        peak_index: 0,
        trough_index: 0,
        recovery_index: Some(0),
    };
    let mut active_peak_index = 0;

    for (index, value) in returns.iter().enumerate() {
        let equity_index = index + 1;
        equity *= 1.0 + value;
        if equity >= peak {
            peak = equity;
            peak_index = equity_index;
            active_peak_index = equity_index;
            if max_drawdown.recovery_index.is_none() && equity >= peak {
                max_drawdown.recovery_index = Some(equity_index);
            }
        }
        let depth = if peak > 0.0 {
            (peak - equity) / peak
        } else {
            0.0
        };
        if depth > max_drawdown.depth {
            max_drawdown = Drawdown {
                depth,
                peak_index: active_peak_index,
                trough_index: equity_index,
                recovery_index: None,
            };
        } else if max_drawdown.recovery_index.is_none()
            && peak_index > max_drawdown.trough_index
            && equity >= peak
        {
            max_drawdown.recovery_index = Some(equity_index);
        }
    }

    Ok(max_drawdown)
}

/// Returns historical VaR and CVaR as positive loss values.
pub fn historical_value_at_risk(returns: &[f64], confidence: f64) -> Result<HistoricalRisk> {
    validate_returns(returns)?;
    if !confidence.is_finite() || confidence <= 0.0 || confidence >= 1.0 {
        return Err(invalid_argument(
            "confidence must be finite and between 0 and 1",
        ));
    }
    let tail_probability = 1.0 - confidence;
    let threshold = quantile(returns, tail_probability)?;
    let tail = returns
        .iter()
        .copied()
        .filter(|value| *value <= threshold)
        .collect::<Vec<_>>();
    let tail_mean = tail.iter().sum::<f64>() / tail.len() as f64;
    Ok(HistoricalRisk {
        confidence,
        value_at_risk: (-threshold).max(0.0),
        conditional_value_at_risk: (-tail_mean).max(0.0),
        return_threshold: threshold,
        observations: returns.len(),
    })
}

/// Returns annualized tracking error between portfolio and benchmark returns.
pub fn tracking_error(
    portfolio_returns: &[f64],
    benchmark_returns: &[f64],
    periods_per_year: f64,
) -> Result<f64> {
    validate_periods_per_year(periods_per_year)?;
    let active = active_returns(portfolio_returns, benchmark_returns)?;
    Ok(std_dev(&active, VarianceMode::Sample)? * periods_per_year.sqrt())
}

/// Returns annualized information ratio between portfolio and benchmark returns.
pub fn information_ratio(
    portfolio_returns: &[f64],
    benchmark_returns: &[f64],
    periods_per_year: f64,
) -> Result<f64> {
    validate_periods_per_year(periods_per_year)?;
    let active = active_returns(portfolio_returns, benchmark_returns)?;
    let tracking_error_per_period = std_dev(&active, VarianceMode::Sample)?;
    if tracking_error_per_period <= f64::EPSILON {
        return Err(invalid_argument(
            "information ratio requires non-zero tracking error",
        ));
    }
    Ok(mean_return(&active)? / tracking_error_per_period * periods_per_year.sqrt())
}

/// Returns portfolio minus benchmark returns.
pub fn active_returns(portfolio_returns: &[f64], benchmark_returns: &[f64]) -> Result<Vec<f64>> {
    validate_pair(portfolio_returns, benchmark_returns)?;
    Ok(portfolio_returns
        .iter()
        .zip(benchmark_returns)
        .map(|(portfolio, benchmark)| portfolio - benchmark)
        .collect())
}

/// Returns rolling arithmetic means for fixed-size windows.
pub fn rolling_mean(values: &[f64], window: usize) -> Result<Vec<f64>> {
    rolling_apply(values, window, mean_return)
}

/// Returns rolling sample standard deviations for fixed-size windows.
pub fn rolling_std_dev(values: &[f64], window: usize) -> Result<Vec<f64>> {
    rolling_apply(values, window, |window| {
        std_dev(window, VarianceMode::Sample)
    })
}

/// Returns rolling correlations for fixed-size paired windows.
pub fn rolling_correlation(left: &[f64], right: &[f64], window: usize) -> Result<Vec<f64>> {
    validate_pair(left, right)?;
    validate_window(window, left.len())?;
    let mut values = Vec::with_capacity(left.len() + 1 - window);
    for start in 0..=(left.len() - window) {
        values.push(correlation(
            &left[start..start + window],
            &right[start..start + window],
        )?);
    }
    Ok(values)
}

fn rolling_apply(
    values: &[f64],
    window: usize,
    mut operation: impl FnMut(&[f64]) -> Result<f64>,
) -> Result<Vec<f64>> {
    validate_returns(values)?;
    validate_window(window, values.len())?;
    let mut results = Vec::with_capacity(values.len() + 1 - window);
    for start in 0..=(values.len() - window) {
        results.push(operation(&values[start..start + window])?);
    }
    Ok(results)
}

fn validate_window(window: usize, len: usize) -> Result<()> {
    if window == 0 {
        return Err(invalid_argument("rolling window must be greater than zero"));
    }
    if window > len {
        return Err(invalid_argument(
            "rolling window must not exceed series length",
        ));
    }
    Ok(())
}

fn validate_prices(prices: &[f64]) -> Result<()> {
    if prices.len() < 2 {
        return Err(invalid_argument(
            "price series must contain at least two values",
        ));
    }
    if prices
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(invalid_argument("prices must be finite and positive"));
    }
    Ok(())
}

fn validate_returns(returns: &[f64]) -> Result<()> {
    if returns.is_empty() {
        return Err(invalid_argument("return series must not be empty"));
    }
    if returns.iter().any(|value| !value.is_finite()) {
        return Err(invalid_argument("returns must be finite"));
    }
    Ok(())
}

fn validate_compoundable_returns(returns: &[f64]) -> Result<()> {
    validate_returns(returns)?;
    if returns.iter().any(|value| *value < -1.0) {
        return Err(invalid_argument(
            "compound returns must be greater than or equal to -1",
        ));
    }
    Ok(())
}

fn validate_pair(left: &[f64], right: &[f64]) -> Result<()> {
    validate_returns(left)?;
    validate_returns(right)?;
    if left.len() != right.len() {
        return Err(invalid_argument("return series lengths must match"));
    }
    Ok(())
}

fn validate_periods_per_year(periods_per_year: f64) -> Result<()> {
    if !periods_per_year.is_finite() || periods_per_year <= 0.0 {
        return Err(invalid_argument(
            "periods per year must be finite and positive",
        ));
    }
    Ok(())
}

fn variance_denominator(len: usize, mode: VarianceMode) -> Result<f64> {
    match mode {
        VarianceMode::Population => Ok(len as f64),
        VarianceMode::Sample => {
            if len < 2 {
                return Err(invalid_argument(
                    "sample variance requires at least two observations",
                ));
            }
            Ok((len - 1) as f64)
        }
    }
}

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(left: f64, right: f64) {
        assert!(
            (left - right).abs() < 1.0e-10,
            "expected {left} to be close to {right}"
        );
    }

    #[test]
    fn computes_return_series_and_compounding() {
        let prices = [100.0, 110.0, 99.0];
        let simple = simple_returns(&prices).unwrap();
        assert_close(simple[0], 0.1);
        assert_close(simple[1], -0.1);
        assert_close(cumulative_return(&simple).unwrap(), -0.01);

        let logs = log_returns(&prices).unwrap();
        assert_close(logs.iter().sum::<f64>(), (99.0_f64 / 100.0).ln());
    }

    #[test]
    fn computes_risk_and_performance_ratios() {
        let returns = [0.02, -0.01, 0.03, -0.02, 0.01];
        assert!(annualized_volatility(&returns, 252.0).unwrap() > 0.0);
        assert!(sharpe_ratio(&returns, 0.0, 252.0).unwrap().is_finite());
        assert!(sortino_ratio(&returns, 0.0, 252.0).unwrap().is_finite());

        let risk = historical_value_at_risk(&returns, 0.8).unwrap();
        assert_eq!(risk.observations, 5);
        assert!(risk.value_at_risk >= 0.0);
        assert!(risk.conditional_value_at_risk >= risk.value_at_risk);
    }

    #[test]
    fn computes_pairwise_finance_metrics() {
        let asset = [0.025, 0.01, -0.005, 0.035];
        let benchmark = [0.01, 0.0, -0.02, 0.02];
        let beta = beta_alpha(&asset, &benchmark, 0.0).unwrap();
        assert!(beta.beta > 0.0);
        assert!(beta.correlation > 0.0);

        assert!(tracking_error(&asset, &benchmark, 252.0).unwrap() > 0.0);
        assert!(information_ratio(&asset, &benchmark, 252.0)
            .unwrap()
            .is_finite());
    }

    #[test]
    fn computes_drawdowns_and_rolling_windows() {
        let returns = [0.1, -0.2, 0.05, 0.3, -0.1];
        let drawdown = max_drawdown(&returns).unwrap();
        assert_close(drawdown.depth, 0.2);
        assert_eq!(drawdown.peak_index, 1);
        assert_eq!(drawdown.trough_index, 2);
        assert_eq!(drawdown.recovery_index, Some(4));

        assert_eq!(rolling_mean(&returns, 3).unwrap().len(), 3);
        assert_eq!(rolling_std_dev(&returns, 3).unwrap().len(), 3);
        for value in rolling_correlation(&returns, &returns, 3).unwrap() {
            assert_close(value, 1.0);
        }
    }

    #[test]
    fn rejects_ambiguous_inputs() {
        assert!(simple_returns(&[1.0]).is_err());
        assert!(simple_returns(&[1.0, 0.0]).is_err());
        assert!(std_dev(&[0.1], VarianceMode::Sample).is_err());
        assert!(historical_value_at_risk(&[0.1], 1.0).is_err());
        assert!(rolling_mean(&[0.1, 0.2], 3).is_err());
        assert!(information_ratio(&[0.1, 0.2], &[0.0, 0.1], 0.0).is_err());
    }
}
