#![doc = include_str!("../README.md")]

pub mod surface;

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use video_analysis_core::{DetectError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Instrument {
    pub id: String,
    pub symbol: String,
    pub name: Option<String>,
    pub exchange: Option<String>,
    pub currency: Option<String>,
    pub asset_class: AssetClass,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssetClass {
    Equity,
    Etf,
    Fund,
    Index,
    Future,
    Option,
    Crypto,
    Forex,
    Bond,
    #[default]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OhlcvBar {
    pub timestamp_ms: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: Option<f64>,
    pub adjusted_close: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Quote {
    pub timestamp_ms: i64,
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub last: Option<f64>,
    pub size: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorporateAction {
    pub timestamp_ms: i64,
    pub kind: CorporateActionKind,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum CorporateActionKind {
    Split { ratio: f64 },
    Dividend { amount: f64 },
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinanceSeries {
    pub instrument: Instrument,
    pub bars: Vec<OhlcvBar>,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinanceBounds {
    pub start_ms: i64,
    pub end_ms: i64,
    pub min_price: f64,
    pub max_price: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskSummaryOptions {
    #[serde(default)]
    pub adjusted: bool,
    #[serde(default = "default_periods_per_year")]
    pub periods_per_year: f64,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    #[serde(default)]
    pub risk_free_return_per_period: f64,
}

impl Default for RiskSummaryOptions {
    fn default() -> Self {
        Self {
            adjusted: false,
            periods_per_year: default_periods_per_year(),
            confidence: default_confidence(),
            risk_free_return_per_period: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskSummary {
    pub mean_return: f64,
    pub std_dev: f64,
    pub annualized_return: f64,
    pub annualized_volatility: f64,
    pub sharpe_ratio: Option<f64>,
    pub sortino_ratio: Option<f64>,
    pub value_at_risk: f64,
    pub conditional_value_at_risk: f64,
    pub max_drawdown: DrawdownSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawdownSummary {
    pub depth: f64,
    pub peak_index: usize,
    pub trough_index: usize,
    pub recovery_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadBarsRequest {
    pub instrument: Instrument,
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
}

pub trait FinanceDataProvider {
    fn load_bars(&self, request: LoadBarsRequest) -> Result<FinanceSeries>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct FinanceSeriesIndex {
    series: FinanceSeries,
}

impl FinanceSeriesIndex {
    pub fn new(mut series: FinanceSeries) -> Result<Self> {
        validate_instrument(&series.instrument)?;
        series.bars.sort_by_key(|bar| bar.timestamp_ms);
        validate_bars(&series.bars)?;
        Ok(Self { series })
    }

    pub fn series(&self) -> &FinanceSeries {
        &self.series
    }

    pub fn bounds(&self) -> Option<FinanceBounds> {
        let first = self.series.bars.first()?;
        let last = self.series.bars.last()?;
        let mut min_price = f64::INFINITY;
        let mut max_price = f64::NEG_INFINITY;

        for bar in &self.series.bars {
            min_price = min_price.min(bar.low);
            max_price = max_price.max(bar.high);
        }

        Some(FinanceBounds {
            start_ms: first.timestamp_ms,
            end_ms: last.timestamp_ms,
            min_price,
            max_price,
        })
    }

    pub fn bars_in_range(&self, start_ms: i64, end_ms: i64) -> Vec<OhlcvBar> {
        if start_ms > end_ms {
            return Vec::new();
        }

        self.series
            .bars
            .iter()
            .copied()
            .filter(|bar| bar.timestamp_ms >= start_ms && bar.timestamp_ms <= end_ms)
            .collect()
    }

    pub fn downsample_ohlcv(
        &self,
        start_ms: i64,
        end_ms: i64,
        target_count: usize,
    ) -> Result<Vec<OhlcvBar>> {
        if target_count == 0 {
            return Err(invalid_argument("target count must be greater than zero"));
        }
        let bars = self.bars_in_range(start_ms, end_ms);
        if bars.len() <= target_count {
            return Ok(bars);
        }

        let bucket_count = target_count.min(bars.len());
        let mut downsampled = Vec::with_capacity(bucket_count);

        for bucket_index in 0..bucket_count {
            let start = bucket_index * bars.len() / bucket_count;
            let end = ((bucket_index + 1) * bars.len() / bucket_count).max(start + 1);
            downsampled.push(aggregate_ohlcv_bucket(&bars[start..end])?);
        }

        Ok(downsampled)
    }

    pub fn close_prices(&self) -> Vec<f64> {
        self.series.bars.iter().map(|bar| bar.close).collect()
    }

    pub fn adjusted_close_prices(&self) -> Vec<f64> {
        self.series
            .bars
            .iter()
            .map(|bar| bar.adjusted_close.unwrap_or(bar.close))
            .collect()
    }

    pub fn simple_returns(&self, adjusted: bool) -> Result<Vec<f64>> {
        let prices = self.prices(adjusted);
        finance_statistics::simple_returns(&prices)
    }

    pub fn log_returns(&self, adjusted: bool) -> Result<Vec<f64>> {
        let prices = self.prices(adjusted);
        finance_statistics::log_returns(&prices)
    }

    pub fn risk_summary(&self, options: RiskSummaryOptions) -> Result<RiskSummary> {
        let returns = self.simple_returns(options.adjusted)?;
        let historical =
            finance_statistics::historical_value_at_risk(&returns, options.confidence)?;
        let drawdown = finance_statistics::max_drawdown(&returns)?;

        Ok(RiskSummary {
            mean_return: finance_statistics::mean_return(&returns)?,
            std_dev: finance_statistics::std_dev(
                &returns,
                finance_statistics::VarianceMode::Sample,
            )?,
            annualized_return: finance_statistics::annualized_return(
                &returns,
                options.periods_per_year,
            )?,
            annualized_volatility: finance_statistics::annualized_volatility(
                &returns,
                options.periods_per_year,
            )?,
            sharpe_ratio: finance_statistics::sharpe_ratio(
                &returns,
                options.risk_free_return_per_period,
                options.periods_per_year,
            )
            .ok(),
            sortino_ratio: finance_statistics::sortino_ratio(
                &returns,
                options.risk_free_return_per_period,
                options.periods_per_year,
            )
            .ok(),
            value_at_risk: historical.value_at_risk,
            conditional_value_at_risk: historical.conditional_value_at_risk,
            max_drawdown: DrawdownSummary {
                depth: drawdown.depth,
                peak_index: drawdown.peak_index,
                trough_index: drawdown.trough_index,
                recovery_index: drawdown.recovery_index,
            },
        })
    }

    fn prices(&self, adjusted: bool) -> Vec<f64> {
        if adjusted {
            self.adjusted_close_prices()
        } else {
            self.close_prices()
        }
    }
}

pub fn parse_ohlcv_json(input: &str) -> Result<FinanceSeries> {
    serde_json::from_str(input).map_err(|error| invalid_argument(format!("invalid JSON: {error}")))
}

fn aggregate_ohlcv_bucket(bars: &[OhlcvBar]) -> Result<OhlcvBar> {
    let first = bars
        .first()
        .ok_or_else(|| invalid_argument("cannot aggregate an empty OHLCV bucket"))?;
    let last = bars
        .last()
        .ok_or_else(|| invalid_argument("cannot aggregate an empty OHLCV bucket"))?;
    let high = bars
        .iter()
        .map(|bar| bar.high)
        .fold(f64::NEG_INFINITY, f64::max);
    let low = bars.iter().map(|bar| bar.low).fold(f64::INFINITY, f64::min);
    let volume = bars
        .iter()
        .any(|bar| bar.volume.is_some())
        .then(|| bars.iter().filter_map(|bar| bar.volume).sum());
    let adjusted_close = last
        .adjusted_close
        .or_else(|| bars.iter().rev().find_map(|bar| bar.adjusted_close));

    Ok(OhlcvBar {
        timestamp_ms: first.timestamp_ms,
        open: first.open,
        high,
        low,
        close: last.close,
        volume,
        adjusted_close,
    })
}

fn validate_instrument(instrument: &Instrument) -> Result<()> {
    if instrument.symbol.trim().is_empty() {
        return Err(invalid_argument("instrument symbol must not be empty"));
    }
    Ok(())
}

fn validate_bars(bars: &[OhlcvBar]) -> Result<()> {
    let mut seen = HashSet::with_capacity(bars.len());
    let mut last_timestamp = None;

    for bar in bars {
        validate_bar(bar)?;

        if !seen.insert(bar.timestamp_ms) {
            return Err(invalid_argument("bar timestamps must be unique"));
        }
        if let Some(previous) = last_timestamp {
            if bar.timestamp_ms < previous {
                return Err(invalid_argument("bars must be sorted ascending"));
            }
        }
        last_timestamp = Some(bar.timestamp_ms);
    }

    Ok(())
}

fn validate_bar(bar: &OhlcvBar) -> Result<()> {
    validate_positive_price(bar.open, "open")?;
    validate_positive_price(bar.high, "high")?;
    validate_positive_price(bar.low, "low")?;
    validate_positive_price(bar.close, "close")?;

    if bar.high < bar.open.max(bar.close).max(bar.low) {
        return Err(invalid_argument(
            "high must be greater than or equal to open, close, and low",
        ));
    }
    if bar.low > bar.open.min(bar.close).min(bar.high) {
        return Err(invalid_argument(
            "low must be less than or equal to open, close, and high",
        ));
    }
    if let Some(volume) = bar.volume {
        if !volume.is_finite() || volume < 0.0 {
            return Err(invalid_argument("volume must be finite and non-negative"));
        }
    }
    if let Some(adjusted_close) = bar.adjusted_close {
        validate_positive_price(adjusted_close, "adjusted close")?;
    }

    Ok(())
}

fn validate_positive_price(value: f64, name: &str) -> Result<()> {
    if !value.is_finite() || value <= 0.0 {
        return Err(invalid_argument(format!(
            "{name} must be finite and positive"
        )));
    }
    Ok(())
}

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

fn default_periods_per_year() -> f64 {
    252.0
}

fn default_confidence() -> f64 {
    0.95
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instrument() -> Instrument {
        Instrument {
            id: "aapl".to_string(),
            symbol: "AAPL".to_string(),
            name: Some("Apple Inc.".to_string()),
            exchange: Some("NASDAQ".to_string()),
            currency: Some("USD".to_string()),
            asset_class: AssetClass::Equity,
        }
    }

    fn bar(timestamp_ms: i64, open: f64, high: f64, low: f64, close: f64) -> OhlcvBar {
        OhlcvBar {
            timestamp_ms,
            open,
            high,
            low,
            close,
            volume: Some(10.0),
            adjusted_close: Some(close - 1.0),
        }
    }

    fn index() -> FinanceSeriesIndex {
        FinanceSeriesIndex::new(FinanceSeries {
            instrument: instrument(),
            bars: vec![
                bar(1, 100.0, 110.0, 99.0, 108.0),
                bar(2, 108.0, 112.0, 105.0, 106.0),
                bar(3, 106.0, 109.0, 101.0, 102.0),
                bar(4, 102.0, 120.0, 100.0, 118.0),
            ],
        })
        .unwrap()
    }

    #[test]
    fn accepts_and_sorts_valid_bars() {
        let index = FinanceSeriesIndex::new(FinanceSeries {
            instrument: instrument(),
            bars: vec![bar(2, 10.0, 12.0, 9.0, 11.0), bar(1, 9.0, 10.0, 8.0, 10.0)],
        })
        .unwrap();
        assert_eq!(index.series().bars[0].timestamp_ms, 1);
    }

    #[test]
    fn rejects_invalid_bars() {
        assert!(FinanceSeriesIndex::new(FinanceSeries {
            instrument: instrument(),
            bars: vec![bar(1, 1.0, 2.0, 0.5, 1.5), bar(1, 1.0, 2.0, 0.5, 1.5)]
        })
        .is_err());

        assert!(FinanceSeriesIndex::new(FinanceSeries {
            instrument: instrument(),
            bars: vec![bar(1, f64::NAN, 2.0, 0.5, 1.5)]
        })
        .is_err());

        assert!(FinanceSeriesIndex::new(FinanceSeries {
            instrument: instrument(),
            bars: vec![bar(1, 10.0, 9.0, 8.0, 10.0)]
        })
        .is_err());

        let mut negative_volume = bar(1, 10.0, 11.0, 9.0, 10.0);
        negative_volume.volume = Some(-1.0);
        assert!(FinanceSeriesIndex::new(FinanceSeries {
            instrument: instrument(),
            bars: vec![negative_volume]
        })
        .is_err());
    }

    #[test]
    fn ranges_and_empty_queries_work() {
        let index = index();
        assert_eq!(index.bars_in_range(2, 3).len(), 2);
        assert!(index.bars_in_range(5, 6).is_empty());
        let bounds = index.bounds().unwrap();
        assert_eq!(bounds.start_ms, 1);
        assert_eq!(bounds.end_ms, 4);
        assert_eq!(bounds.min_price, 99.0);
        assert_eq!(bounds.max_price, 120.0);
    }

    #[test]
    fn downsamples_ohlcv_without_averaging_prices() {
        let bars = index().downsample_ohlcv(1, 4, 2).unwrap();
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0].timestamp_ms, 1);
        assert_eq!(bars[0].open, 100.0);
        assert_eq!(bars[0].high, 112.0);
        assert_eq!(bars[0].low, 99.0);
        assert_eq!(bars[0].close, 106.0);
        assert_eq!(bars[0].volume, Some(20.0));
        assert_eq!(bars[0].adjusted_close, Some(105.0));
    }

    #[test]
    fn computes_returns_and_risk_with_adjusted_mode() {
        let index = index();
        assert_eq!(index.simple_returns(false).unwrap().len(), 3);
        assert_eq!(index.log_returns(true).unwrap().len(), 3);
        let risk = index.risk_summary(RiskSummaryOptions::default()).unwrap();
        assert!(risk.annualized_volatility > 0.0);
        assert!(risk.value_at_risk >= 0.0);
    }

    #[test]
    fn parses_provider_neutral_json() {
        let json = serde_json::to_string(&FinanceSeries {
            instrument: instrument(),
            bars: vec![bar(1, 10.0, 11.0, 9.0, 10.5)],
        })
        .unwrap();
        let parsed = parse_ohlcv_json(&json).unwrap();
        assert_eq!(parsed.bars.len(), 1);
    }
}
