//! WASM bindings for `finance-data`.

use finance_data::{FinanceSeries, FinanceSeriesIndex, RiskSummaryOptions};
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
            "simple" => self.index.simple_returns(query.adjusted),
            "log" => self.index.log_returns(query.adjusted),
            method => Err(video_analysis_error(format!(
                "unsupported returns method `{method}`"
            ))),
        }
        .map_err(into_js_error)?;
        serde_wasm_bindgen::to_value(&returns).map_err(into_js_error)
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

fn video_analysis_error(message: String) -> video_analysis_core::DetectError {
    video_analysis_core::DetectError::InvalidArgument(message)
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
}
