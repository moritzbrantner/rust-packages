//! WASM bindings for `finance-data`.

use finance_data::{FinanceSeries, FinanceSeriesIndex, RiskSummaryOptions};
use runtime_core::SurfaceRequest;
use serde::Deserialize;
use wasm_bindgen::prelude::*;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BarsQuery {
    start_ms: i64,
    end_ms: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DownsampleQuery {
    start_ms: i64,
    end_ms: i64,
    target_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReturnsQuery {
    #[serde(default)]
    adjusted: bool,
    #[serde(default = "default_returns_method")]
    method: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompactReturnsQuery {
    start_ms: i64,
    end_ms: i64,
    #[serde(default)]
    adjusted: bool,
    #[serde(default = "default_returns_method")]
    method: String,
    target_count: Option<usize>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CompactReturns {
    point_count: Vec<u32>,
    x: Vec<f64>,
    y: Vec<f64>,
    summary: CompactReturnsSummary,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CompactReturnsSummary {
    point_count: usize,
    sample_count: usize,
    x_domain: [f64; 2],
}

#[wasm_bindgen(js_name = packageSurface)]
pub fn package_surface() -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(&finance_data::surface::package_surface()).map_err(into_js_error)
}

#[wasm_bindgen(js_name = runOperation)]
pub fn run_operation(request: JsValue) -> Result<JsValue, JsValue> {
    let request: SurfaceRequest = serde_wasm_bindgen::from_value(request).map_err(into_js_error)?;
    let response = finance_data::surface::run_surface_operation(request).map_err(into_js_error)?;
    serde_wasm_bindgen::to_value(&response).map_err(into_js_error)
}

#[wasm_bindgen(js_name = FinanceDataSeriesIndex)]
pub struct FinanceDataWasmSeriesIndex {
    index: FinanceSeriesIndex,
}

#[wasm_bindgen(js_class = FinanceDataSeriesIndex)]
impl FinanceDataWasmSeriesIndex {
    #[wasm_bindgen(constructor)]
    pub fn new(input: JsValue) -> Result<FinanceDataWasmSeriesIndex, JsValue> {
        let series: FinanceSeries = serde_wasm_bindgen::from_value(input).map_err(into_js_error)?;
        let index = FinanceSeriesIndex::new(series).map_err(into_js_error)?;
        Ok(Self { index })
    }

    #[wasm_bindgen(js_name = getBounds)]
    pub fn get_bounds(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.index.bounds()).map_err(into_js_error)
    }

    #[wasm_bindgen(js_name = getBars)]
    pub fn get_bars(&self, query: JsValue) -> Result<JsValue, JsValue> {
        let query: BarsQuery = serde_wasm_bindgen::from_value(query).map_err(into_js_error)?;
        serde_wasm_bindgen::to_value(&self.index.bars_in_range(query.start_ms, query.end_ms))
            .map_err(into_js_error)
    }

    #[wasm_bindgen(js_name = getDownsampledBars)]
    pub fn get_downsampled_bars(&self, query: JsValue) -> Result<JsValue, JsValue> {
        let query: DownsampleQuery =
            serde_wasm_bindgen::from_value(query).map_err(into_js_error)?;
        let bars = self
            .index
            .downsample_ohlcv(query.start_ms, query.end_ms, query.target_count)
            .map_err(into_js_error)?;
        serde_wasm_bindgen::to_value(&bars).map_err(into_js_error)
    }

    #[wasm_bindgen(js_name = getReturns)]
    pub fn get_returns(&self, query: JsValue) -> Result<JsValue, JsValue> {
        let query: ReturnsQuery = serde_wasm_bindgen::from_value(query).map_err(into_js_error)?;
        let returns = match query.method.as_str() {
            "simple" => self
                .index
                .simple_returns(query.adjusted)
                .map_err(into_js_error)?,
            "log" => self
                .index
                .log_returns(query.adjusted)
                .map_err(into_js_error)?,
            method => {
                return Err(into_js_error(format!(
                    "unsupported returns method `{method}`"
                )))
            }
        };
        serde_wasm_bindgen::to_value(&returns).map_err(into_js_error)
    }

    #[wasm_bindgen(js_name = getCompactReturns)]
    pub fn get_compact_returns(&self, query: JsValue) -> Result<JsValue, JsValue> {
        let query: CompactReturnsQuery =
            serde_wasm_bindgen::from_value(query).map_err(into_js_error)?;
        let series = self.index.series();
        let start = lower_bound_timestamp(&series.bars, query.start_ms);
        let end = upper_bound_timestamp(&series.bars, query.end_ms);
        let return_count = end.saturating_sub(start).saturating_sub(1);
        let target_count = query
            .target_count
            .unwrap_or_else(|| return_count.max(1))
            .max(1);
        let bucket_count = return_count.min(target_count);
        let mut point_count = vec![0_u32; bucket_count];
        let mut x = vec![0.0_f64; bucket_count];
        let mut y = vec![f64::NAN; bucket_count];
        let mut sums = vec![0.0_f64; bucket_count];
        let mut bucket_index = 0;
        let mut bucket_end = return_bucket_end(bucket_index, return_count, bucket_count);

        for return_index in 0..return_count {
            let previous = return_price(&series.bars[start + return_index], query.adjusted);
            let current = return_price(&series.bars[start + return_index + 1], query.adjusted);
            let value = match query.method.as_str() {
                "simple" => current / previous - 1.0,
                "log" => (current / previous).ln(),
                method => {
                    return Err(into_js_error(format!(
                        "unsupported returns method `{method}`"
                    )));
                }
            };

            while return_index >= bucket_end && bucket_index < bucket_count.saturating_sub(1) {
                bucket_index += 1;
                bucket_end = return_bucket_end(bucket_index, return_count, bucket_count);
            }

            x[bucket_index] = series.bars[start + return_index + 1].timestamp_ms as f64;
            point_count[bucket_index] += 1;
            sums[bucket_index] += value;
        }

        for index in 0..bucket_count {
            if point_count[index] > 0 {
                y[index] = sums[index] / f64::from(point_count[index]);
            }
        }

        serde_wasm_bindgen::to_value(&CompactReturns {
            point_count,
            x,
            y,
            summary: CompactReturnsSummary {
                point_count: return_count,
                sample_count: bucket_count,
                x_domain: [query.start_ms as f64, query.end_ms as f64],
            },
        })
        .map_err(into_js_error)
    }

    #[wasm_bindgen(js_name = getRiskSummary)]
    pub fn get_risk_summary(&self, query: JsValue) -> Result<JsValue, JsValue> {
        let query: RiskSummaryOptions =
            serde_wasm_bindgen::from_value(query).map_err(into_js_error)?;
        serde_wasm_bindgen::to_value(&self.index.risk_summary(query).map_err(into_js_error)?)
            .map_err(into_js_error)
    }
}

fn default_returns_method() -> String {
    "simple".to_string()
}

fn lower_bound_timestamp(bars: &[finance_data::OhlcvBar], timestamp_ms: i64) -> usize {
    let mut low = 0;
    let mut high = bars.len();

    while low < high {
        let mid = low + (high - low) / 2;
        if bars[mid].timestamp_ms < timestamp_ms {
            low = mid + 1;
        } else {
            high = mid;
        }
    }

    low
}

fn upper_bound_timestamp(bars: &[finance_data::OhlcvBar], timestamp_ms: i64) -> usize {
    let mut low = 0;
    let mut high = bars.len();

    while low < high {
        let mid = low + (high - low) / 2;
        if bars[mid].timestamp_ms <= timestamp_ms {
            low = mid + 1;
        } else {
            high = mid;
        }
    }

    low
}

fn return_bucket_end(bucket_index: usize, return_count: usize, bucket_count: usize) -> usize {
    if bucket_count == 0 {
        return 0;
    }

    let start = bucket_index * return_count / bucket_count;
    (((bucket_index + 1) * return_count) / bucket_count).max(start + 1)
}

fn return_price(bar: &finance_data::OhlcvBar, adjusted: bool) -> f64 {
    if adjusted {
        bar.adjusted_close.unwrap_or(bar.close)
    } else {
        bar.close
    }
}

fn into_js_error(error: impl std::fmt::Display) -> JsValue {
    js_sys::Error::new(&error.to_string()).into()
}

#[cfg(test)]
mod tests {
    use finance_data::{AssetClass, Instrument, OhlcvBar};

    #[test]
    fn wrapped_index_accepts_series() {
        let series = finance_data::FinanceSeries {
            instrument: Instrument {
                id: "aapl".to_string(),
                symbol: "AAPL".to_string(),
                name: None,
                exchange: None,
                currency: Some("USD".to_string()),
                asset_class: AssetClass::Equity,
            },
            bars: vec![
                OhlcvBar {
                    timestamp_ms: 1,
                    open: 100.0,
                    high: 110.0,
                    low: 99.0,
                    close: 108.0,
                    volume: Some(10.0),
                    adjusted_close: Some(107.0),
                },
                OhlcvBar {
                    timestamp_ms: 2,
                    open: 108.0,
                    high: 112.0,
                    low: 105.0,
                    close: 106.0,
                    volume: Some(11.0),
                    adjusted_close: Some(105.0),
                },
            ],
        };
        assert!(finance_data::FinanceSeriesIndex::new(series).is_ok());
    }

    #[test]
    fn wrapped_surface_has_operations() {
        let surface = finance_data::surface::package_surface();
        assert_eq!(surface.library, "moritzbrantner-finance-data");
        assert!(surface
            .operations
            .iter()
            .any(|operation| operation.id.as_str() == "financeData.bounds"));
    }
}
