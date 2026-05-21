//! WASM bindings for maps-kernels-core.

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = resampleLineFlat)]
/// Resamples an open line represented as flat coordinates.
pub fn resample_line_flat_binding(
    coordinates: &[f64],
    coordinate_count: usize,
) -> Result<Vec<f64>, JsValue> {
    maps_kernels_core::resample_line_flat(coordinates, coordinate_count).map_err(into_js_error)
}

#[wasm_bindgen(js_name = resampleRingFlat)]
/// Resamples an open ring represented as flat coordinates.
pub fn resample_ring_flat_binding(
    open_ring: &[f64],
    coordinate_count: usize,
) -> Result<Vec<f64>, JsValue> {
    maps_kernels_core::resample_ring_flat(open_ring, coordinate_count).map_err(into_js_error)
}

fn into_js_error(error: video_analysis_core::DetectError) -> JsValue {
    JsValue::from_str(&error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_binding_resamples_flat_coordinates() {
        let samples = resample_line_flat_binding(&[0.0, 0.0, 10.0, 0.0], 3).unwrap();

        assert_eq!(samples, vec![0.0, 0.0, 5.0, 0.0, 10.0, 0.0]);
    }
}
