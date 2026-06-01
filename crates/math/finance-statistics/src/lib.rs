#![doc = include_str!("../README.md")]

pub mod surface;
use video_analysis_core::{DetectError, Result};

pub use math_statistics::{Drawdown, VarianceMode};

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
    math_statistics::changes(prices, math_statistics::ChangeMethod::Relative)
}

/// Returns natural-log period returns from strictly positive prices.
pub fn log_returns(prices: &[f64]) -> Result<Vec<f64>> {
    math_statistics::changes(prices, math_statistics::ChangeMethod::LogRatio)
}

/// Returns the arithmetic mean of a return series.
pub fn mean_return(returns: &[f64]) -> Result<f64> {
    math_statistics::mean(returns)
}

/// Returns variance for a return series.
pub fn variance(returns: &[f64], mode: VarianceMode) -> Result<f64> {
    math_statistics::variance(returns, mode)
}

/// Returns standard deviation for a return series.
pub fn std_dev(returns: &[f64], mode: VarianceMode) -> Result<f64> {
    math_statistics::std_dev(returns, mode)
}

/// Returns compounded cumulative return.
pub fn cumulative_return(returns: &[f64]) -> Result<f64> {
    math_statistics::cumulative_compounded_return(returns)
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
    math_statistics::covariance(left, right, mode)
}

/// Returns Pearson correlation between two equally sized return series.
pub fn correlation(left: &[f64], right: &[f64]) -> Result<f64> {
    math_statistics::correlation(left, right)
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
    math_statistics::max_drawdown(returns)
}

/// Returns historical VaR and CVaR as positive loss values.
pub fn historical_value_at_risk(returns: &[f64], confidence: f64) -> Result<HistoricalRisk> {
    let tail = math_statistics::tail_risk(returns, confidence)?;
    Ok(HistoricalRisk {
        confidence: tail.confidence,
        value_at_risk: tail.value_at_risk,
        conditional_value_at_risk: tail.conditional_value_at_risk,
        return_threshold: tail.threshold,
        observations: tail.observations,
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
    math_statistics::rolling_mean(values, window)
}

/// Returns rolling sample standard deviations for fixed-size windows.
pub fn rolling_std_dev(values: &[f64], window: usize) -> Result<Vec<f64>> {
    math_statistics::rolling_std_dev(values, window, VarianceMode::Sample)
}

/// Returns rolling correlations for fixed-size paired windows.
pub fn rolling_correlation(left: &[f64], right: &[f64], window: usize) -> Result<Vec<f64>> {
    math_statistics::rolling_correlation(left, right, window)
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
    fn finance_wrappers_match_generic_statistics() {
        let prices = [100.0, 102.0, 99.0, 105.0];
        let returns = simple_returns(&prices).unwrap();
        assert_eq!(
            returns,
            math_statistics::changes(&prices, math_statistics::ChangeMethod::Relative).unwrap()
        );
        assert_close(
            mean_return(&returns).unwrap(),
            math_statistics::mean(&returns).unwrap(),
        );
        assert_close(
            std_dev(&returns, VarianceMode::Sample).unwrap(),
            math_statistics::std_dev(&returns, VarianceMode::Sample).unwrap(),
        );
        assert_close(
            correlation(&returns, &returns).unwrap(),
            math_statistics::correlation(&returns, &returns).unwrap(),
        );

        let risk = historical_value_at_risk(&returns, 0.8).unwrap();
        let generic_risk = math_statistics::tail_risk(&returns, 0.8).unwrap();
        assert_close(risk.value_at_risk, generic_risk.value_at_risk);
        assert_close(
            risk.conditional_value_at_risk,
            generic_risk.conditional_value_at_risk,
        );

        assert_eq!(
            max_drawdown(&returns).unwrap(),
            math_statistics::max_drawdown(&returns).unwrap()
        );
        assert_eq!(
            rolling_mean(&returns, 2).unwrap(),
            math_statistics::rolling_mean(&returns, 2).unwrap()
        );
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
