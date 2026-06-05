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

#[derive(Debug, Clone, PartialEq)]
/// Validated portfolio weights.
pub struct PortfolioWeights {
    weights: Vec<f64>,
}

impl PortfolioWeights {
    /// Creates finite non-negative weights that sum to 1 within a small tolerance.
    pub fn new(weights: Vec<f64>) -> Result<Self> {
        if weights.is_empty() {
            return Err(invalid_argument("portfolio weights must not be empty"));
        }
        if weights
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(invalid_argument(
                "portfolio weights must be finite and non-negative",
            ));
        }
        let sum = weights.iter().sum::<f64>();
        if (sum - 1.0).abs() > 1.0e-6 {
            return Err(invalid_argument("portfolio weights must sum to 1"));
        }
        Ok(Self { weights })
    }

    /// Borrows validated weights.
    pub fn weights(&self) -> &[f64] {
        &self.weights
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Historical covariance matrix for aligned asset return series.
pub struct PortfolioCovariance {
    /// Number of assets.
    pub asset_count: usize,
    /// Number of aligned observations per asset.
    pub observations: usize,
    /// Variance normalization mode.
    pub mode: VarianceMode,
    /// Row-major `asset_count x asset_count` covariance values.
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
/// Portfolio volatility attribution by marginal, component, and percent risk.
pub struct PortfolioRiskContribution {
    /// Annualized portfolio variance.
    pub portfolio_variance: f64,
    /// Annualized portfolio volatility.
    pub volatility: f64,
    /// Marginal volatility contributions.
    pub marginal_contributions: Vec<f64>,
    /// Weighted component volatility contributions.
    pub component_contributions: Vec<f64>,
    /// Component contributions divided by total volatility.
    pub percent_contributions: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
/// Expected return attribution by asset.
pub struct PortfolioReturnContribution {
    /// Mean asset return multiplied by asset weight.
    pub weighted_returns: Vec<f64>,
    /// Percent contributions, absent when expected return is effectively zero.
    pub percent_contributions: Vec<Option<f64>>,
    /// Sum of weighted mean returns.
    pub expected_portfolio_return: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Portfolio rebalance turnover summary.
pub struct PortfolioTurnover {
    /// Sum of absolute target/current weight differences.
    pub gross_turnover: f64,
    /// One-way turnover equal to half the gross turnover.
    pub one_way_turnover: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Portfolio risk summary.
pub struct PortfolioRiskSummary {
    /// Mean portfolio return per period.
    pub mean_return: f64,
    /// Annualized sample volatility.
    pub volatility: f64,
    /// Annualized Sharpe ratio when non-zero volatility is available.
    pub sharpe: Option<f64>,
    /// Maximum drawdown depth.
    pub max_drawdown: f64,
    /// Historical positive value at risk.
    pub value_at_risk: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Drawdown duration summary for a compounded return path.
pub struct DrawdownDuration {
    /// Maximum number of periods spent below the prior equity peak.
    pub max_duration: usize,
    /// Current drawdown duration at the end of the series.
    pub current_duration: usize,
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

/// Computes weighted portfolio returns from aligned asset return series.
pub fn portfolio_returns(asset_returns: &[Vec<f64>], weights: &[f64]) -> Result<Vec<f64>> {
    let weights = PortfolioWeights::new(weights.to_vec())?;
    if asset_returns.len() != weights.weights().len() {
        return Err(invalid_argument(
            "asset return series count must match weight count",
        ));
    }
    let periods = asset_returns
        .first()
        .ok_or_else(|| invalid_argument("asset returns must not be empty"))?
        .len();
    if periods == 0 {
        return Err(invalid_argument("asset return series must not be empty"));
    }
    for returns in asset_returns {
        validate_returns(returns)?;
        if returns.len() != periods {
            return Err(invalid_argument("asset return series lengths must match"));
        }
    }
    let mut portfolio = vec![0.0; periods];
    for (returns, weight) in asset_returns.iter().zip(weights.weights()) {
        for (index, value) in returns.iter().copied().enumerate() {
            portfolio[index] += value * weight;
        }
    }
    Ok(portfolio)
}

/// Computes historical covariance between aligned asset return series.
pub fn portfolio_covariance(
    asset_returns: &[Vec<f64>],
    mode: VarianceMode,
) -> Result<PortfolioCovariance> {
    let observations = validate_asset_returns(asset_returns, mode)?;
    let asset_count = asset_returns.len();
    let mut values = vec![0.0; asset_count * asset_count];
    for row in 0..asset_count {
        for col in 0..asset_count {
            values[row * asset_count + col] =
                covariance(&asset_returns[row], &asset_returns[col], mode)?;
        }
    }
    Ok(PortfolioCovariance {
        asset_count,
        observations,
        mode,
        values,
    })
}

/// Computes `w^T covariance w`.
pub fn portfolio_variance(covariance: &PortfolioCovariance, weights: &[f64]) -> Result<f64> {
    validate_portfolio_covariance(covariance)?;
    let weights = PortfolioWeights::new(weights.to_vec())?;
    if weights.weights().len() != covariance.asset_count {
        return Err(invalid_argument(
            "portfolio covariance asset count must match weight count",
        ));
    }
    let mut variance = 0.0;
    for row in 0..covariance.asset_count {
        for col in 0..covariance.asset_count {
            variance += weights.weights()[row]
                * covariance.values[row * covariance.asset_count + col]
                * weights.weights()[col];
        }
    }
    if !variance.is_finite() {
        return Err(invalid_argument("portfolio variance must be finite"));
    }
    Ok(variance)
}

/// Computes historical covariance-based annualized portfolio risk contributions.
pub fn portfolio_risk_contribution(
    asset_returns: &[Vec<f64>],
    weights: &[f64],
    periods_per_year: f64,
) -> Result<PortfolioRiskContribution> {
    validate_periods_per_year(periods_per_year)?;
    let weights = PortfolioWeights::new(weights.to_vec())?;
    if asset_returns.len() != weights.weights().len() {
        return Err(invalid_argument(
            "asset return series count must match weight count",
        ));
    }
    let covariance = portfolio_covariance(asset_returns, VarianceMode::Sample)?;
    let annualized_values = covariance
        .values
        .iter()
        .map(|value| value * periods_per_year)
        .collect::<Vec<_>>();
    let annualized = PortfolioCovariance {
        asset_count: covariance.asset_count,
        observations: covariance.observations,
        mode: covariance.mode,
        values: annualized_values,
    };
    let portfolio_variance = portfolio_variance(&annualized, weights.weights())?;
    if portfolio_variance <= f64::EPSILON {
        return Err(invalid_argument(
            "risk contribution requires positive portfolio variance",
        ));
    }
    let volatility = portfolio_variance.sqrt();
    let mut covariance_times_weights = vec![0.0; annualized.asset_count];
    for (row, value) in covariance_times_weights.iter_mut().enumerate() {
        for col in 0..annualized.asset_count {
            *value +=
                annualized.values[row * annualized.asset_count + col] * weights.weights()[col];
        }
    }
    let marginal_contributions = covariance_times_weights
        .iter()
        .map(|value| value / volatility)
        .collect::<Vec<_>>();
    let component_contributions = weights
        .weights()
        .iter()
        .zip(&marginal_contributions)
        .map(|(weight, marginal)| weight * marginal)
        .collect::<Vec<_>>();
    let percent_contributions = component_contributions
        .iter()
        .map(|value| value / volatility)
        .collect::<Vec<_>>();
    Ok(PortfolioRiskContribution {
        portfolio_variance,
        volatility,
        marginal_contributions,
        component_contributions,
        percent_contributions,
    })
}

/// Computes expected return attribution from aligned asset return series.
pub fn portfolio_return_contribution(
    asset_returns: &[Vec<f64>],
    weights: &[f64],
) -> Result<PortfolioReturnContribution> {
    let _ = validate_asset_returns(asset_returns, VarianceMode::Population)?;
    let weights = PortfolioWeights::new(weights.to_vec())?;
    if asset_returns.len() != weights.weights().len() {
        return Err(invalid_argument(
            "asset return series count must match weight count",
        ));
    }
    let weighted_returns = asset_returns
        .iter()
        .zip(weights.weights())
        .map(|(returns, weight)| Ok(mean_return(returns)? * weight))
        .collect::<Result<Vec<_>>>()?;
    let expected_portfolio_return = weighted_returns.iter().sum::<f64>();
    let percent_contributions = if expected_portfolio_return.abs() <= f64::EPSILON {
        vec![None; weighted_returns.len()]
    } else {
        weighted_returns
            .iter()
            .map(|value| Some(value / expected_portfolio_return))
            .collect()
    };
    Ok(PortfolioReturnContribution {
        weighted_returns,
        percent_contributions,
        expected_portfolio_return,
    })
}

/// Computes gross and one-way rebalance turnover between two validated portfolios.
pub fn portfolio_turnover(
    current_weights: &[f64],
    target_weights: &[f64],
) -> Result<PortfolioTurnover> {
    let current = PortfolioWeights::new(current_weights.to_vec())?;
    let target = PortfolioWeights::new(target_weights.to_vec())?;
    if current.weights().len() != target.weights().len() {
        return Err(invalid_argument("portfolio weight lengths must match"));
    }
    let gross_turnover = current
        .weights()
        .iter()
        .zip(target.weights())
        .map(|(current, target)| (target - current).abs())
        .sum::<f64>();
    Ok(PortfolioTurnover {
        gross_turnover,
        one_way_turnover: gross_turnover / 2.0,
    })
}

/// Computes a compact portfolio risk summary.
pub fn portfolio_risk_summary(
    asset_returns: &[Vec<f64>],
    weights: &[f64],
    periods_per_year: f64,
    risk_free_return_per_period: f64,
    confidence: f64,
) -> Result<PortfolioRiskSummary> {
    let returns = portfolio_returns(asset_returns, weights)?;
    let risk = historical_value_at_risk(&returns, confidence)?;
    Ok(PortfolioRiskSummary {
        mean_return: mean_return(&returns)?,
        volatility: annualized_volatility(&returns, periods_per_year)?,
        sharpe: sharpe_ratio(&returns, risk_free_return_per_period, periods_per_year).ok(),
        max_drawdown: max_drawdown(&returns)?.depth,
        value_at_risk: risk.value_at_risk,
    })
}

/// Returns annualized return divided by maximum drawdown.
pub fn calmar_ratio(returns: &[f64], periods_per_year: f64) -> Result<f64> {
    let annualized = annualized_return(returns, periods_per_year)?;
    let drawdown = max_drawdown(returns)?.depth;
    if drawdown <= f64::EPSILON {
        return Err(invalid_argument("Calmar ratio requires non-zero drawdown"));
    }
    Ok(annualized / drawdown)
}

/// Returns gains above a threshold divided by losses below the threshold.
pub fn omega_ratio(returns: &[f64], threshold_return_per_period: f64) -> Result<f64> {
    validate_returns(returns)?;
    if !threshold_return_per_period.is_finite() {
        return Err(invalid_argument("Omega threshold must be finite"));
    }
    let gains = returns
        .iter()
        .map(|value| (value - threshold_return_per_period).max(0.0))
        .sum::<f64>();
    let losses = returns
        .iter()
        .map(|value| (threshold_return_per_period - value).max(0.0))
        .sum::<f64>();
    if losses <= f64::EPSILON {
        return Err(invalid_argument("Omega ratio requires non-zero losses"));
    }
    Ok(gains / losses)
}

/// Summarizes drawdown durations for a compounded return path.
pub fn drawdown_duration(returns: &[f64]) -> Result<DrawdownDuration> {
    validate_returns(returns)?;
    let mut equity = 1.0;
    let mut peak = 1.0;
    let mut current_duration = 0;
    let mut max_duration = 0;
    for value in returns {
        if *value < -1.0 {
            return Err(invalid_argument(
                "compound values must be greater than or equal to -1",
            ));
        }
        equity *= 1.0 + value;
        if equity >= peak {
            peak = equity;
            current_duration = 0;
        } else {
            current_duration += 1;
            max_duration = max_duration.max(current_duration);
        }
    }
    Ok(DrawdownDuration {
        max_duration,
        current_duration,
    })
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

fn validate_asset_returns(asset_returns: &[Vec<f64>], mode: VarianceMode) -> Result<usize> {
    if asset_returns.is_empty() {
        return Err(invalid_argument("asset returns must not be empty"));
    }
    let observations = asset_returns[0].len();
    if observations == 0 {
        return Err(invalid_argument("asset return series must not be empty"));
    }
    if matches!(mode, VarianceMode::Sample) && observations < 2 {
        return Err(invalid_argument(
            "sample portfolio covariance requires at least two observations per asset",
        ));
    }
    for returns in asset_returns {
        validate_returns(returns)?;
        if returns.len() != observations {
            return Err(invalid_argument("asset return series lengths must match"));
        }
    }
    Ok(observations)
}

fn validate_portfolio_covariance(covariance: &PortfolioCovariance) -> Result<()> {
    if covariance.asset_count == 0 {
        return Err(invalid_argument(
            "portfolio covariance asset count must be greater than zero",
        ));
    }
    if covariance.values.len() != covariance.asset_count * covariance.asset_count {
        return Err(invalid_argument(
            "portfolio covariance values must match asset count",
        ));
    }
    if covariance.values.iter().any(|value| !value.is_finite()) {
        return Err(invalid_argument(
            "portfolio covariance values must be finite",
        ));
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
    fn portfolio_and_performance_helpers_work() {
        let assets = vec![vec![0.02, -0.01, 0.03, 0.01], vec![0.01, 0.00, 0.02, -0.01]];
        let returns = portfolio_returns(&assets, &[0.6, 0.4]).unwrap();
        assert_eq!(returns.len(), 4);
        assert_close(returns[0], 0.016);

        let summary = portfolio_risk_summary(&assets, &[0.6, 0.4], 252.0, 0.0, 0.8).unwrap();
        assert!(summary.volatility > 0.0);
        assert!(summary.value_at_risk >= 0.0);

        assert!(calmar_ratio(&[0.1, -0.2, 0.3], 252.0).unwrap().is_finite());
        assert!(omega_ratio(&[0.1, -0.2, 0.3], 0.0).unwrap().is_finite());
        assert!(
            drawdown_duration(&[0.1, -0.2, 0.05, 0.3])
                .unwrap()
                .max_duration
                > 0
        );
        assert!(PortfolioWeights::new(vec![0.8, 0.3]).is_err());
    }

    #[test]
    fn portfolio_covariance_matrix_is_symmetric() {
        let assets = vec![vec![0.02, -0.01, 0.03], vec![0.01, 0.0, 0.02]];
        let covariance = portfolio_covariance(&assets, VarianceMode::Sample).unwrap();

        assert_eq!(covariance.asset_count, 2);
        assert_eq!(covariance.observations, 3);
        assert_close(covariance.values[1], covariance.values[2]);
    }

    #[test]
    fn portfolio_variance_matches_manual_two_asset_calculation() {
        let covariance = PortfolioCovariance {
            asset_count: 2,
            observations: 3,
            mode: VarianceMode::Sample,
            values: vec![0.04, 0.01, 0.01, 0.09],
        };
        let variance = portfolio_variance(&covariance, &[0.6, 0.4]).unwrap();

        assert_close(
            variance,
            0.6 * 0.6 * 0.04 + 2.0 * 0.6 * 0.4 * 0.01 + 0.4 * 0.4 * 0.09,
        );
    }

    #[test]
    fn risk_contributions_sum_to_annualized_volatility() {
        let assets = vec![vec![0.02, -0.01, 0.03, 0.01], vec![0.01, 0.0, 0.02, -0.01]];
        let risk = portfolio_risk_contribution(&assets, &[0.6, 0.4], 252.0).unwrap();

        assert!(risk.portfolio_variance > 0.0);
        assert!(risk.volatility > 0.0);
        assert_close(
            risk.component_contributions.iter().sum::<f64>(),
            risk.volatility,
        );
        assert_close(risk.percent_contributions.iter().sum::<f64>(), 1.0);
    }

    #[test]
    fn return_contributions_sum_to_expected_portfolio_return() {
        let assets = vec![vec![0.02, -0.01, 0.03], vec![0.01, 0.0, 0.02]];
        let contribution = portfolio_return_contribution(&assets, &[0.6, 0.4]).unwrap();

        assert_close(
            contribution.weighted_returns.iter().sum::<f64>(),
            contribution.expected_portfolio_return,
        );
        assert!(contribution
            .percent_contributions
            .iter()
            .all(Option::is_some));
    }

    #[test]
    fn turnover_handles_identical_and_changed_portfolios() {
        let unchanged = portfolio_turnover(&[0.6, 0.4], &[0.6, 0.4]).unwrap();
        let changed = portfolio_turnover(&[0.6, 0.4], &[0.5, 0.5]).unwrap();

        assert_close(unchanged.gross_turnover, 0.0);
        assert_close(unchanged.one_way_turnover, 0.0);
        assert_close(changed.gross_turnover, 0.2);
        assert_close(changed.one_way_turnover, 0.1);
    }

    #[test]
    fn risk_attribution_rejects_invalid_inputs() {
        assert!(portfolio_risk_contribution(
            &[vec![0.01, 0.01], vec![0.02, 0.02]],
            &[0.5, 0.5],
            252.0
        )
        .is_err());
        assert!(
            portfolio_risk_contribution(&[vec![0.01, 0.02], vec![0.02]], &[0.5, 0.5], 252.0)
                .is_err()
        );
        assert!(portfolio_risk_contribution(
            &[vec![0.01, 0.02], vec![0.0, 0.03]],
            &[0.7, 0.7],
            252.0
        )
        .is_err());
        assert!(portfolio_risk_contribution(
            &[vec![0.01, 0.02], vec![0.0, 0.03]],
            &[0.5, 0.5],
            0.0
        )
        .is_err());
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
