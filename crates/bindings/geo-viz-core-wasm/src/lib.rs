//! WASM bindings for `geo-viz-core`.

use serde::Serialize;
use video_analysis_core::runtime::SurfaceRequest;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = GeoPointIndex)]
pub struct WasmGeoPointIndex {
    inner: geo_viz_core::GeoPointIndex,
}

#[wasm_bindgen(js_name = GeoFlowIndex)]
pub struct WasmGeoFlowIndex {
    inner: geo_viz_core::GeoFlowIndex,
}

#[wasm_bindgen(js_name = GeoJsonIndex)]
pub struct WasmGeoJsonIndex {
    inner: geo_viz_core::GeoJsonIndex,
}

#[wasm_bindgen(js_class = GeoPointIndex)]
impl WasmGeoPointIndex {
    #[wasm_bindgen(constructor)]
    pub fn new(points: JsValue, options: JsValue) -> Result<WasmGeoPointIndex, JsValue> {
        let points: Vec<geo_viz_core::GeoVizPoint> =
            serde_wasm_bindgen::from_value(points).map_err(into_js_error)?;
        let options = if options.is_undefined() || options.is_null() {
            geo_viz_core::GeoVizAggregationOptions::default()
        } else {
            serde_wasm_bindgen::from_value(options).map_err(into_js_error)?
        };
        let inner = geo_viz_core::GeoPointIndex::new(points, options).map_err(into_js_error)?;
        Ok(Self { inner })
    }

    #[wasm_bindgen(js_name = getBounds)]
    pub fn get_bounds(&self) -> Result<JsValue, JsValue> {
        to_json_value(&self.inner.get_bounds())
    }

    #[wasm_bindgen(js_name = getPointById)]
    pub fn get_point_by_id(&self, point_id: String) -> Result<JsValue, JsValue> {
        to_json_value(&self.inner.get_point_by_id(&point_id))
    }

    #[wasm_bindgen(js_name = getViewportAggregation)]
    pub fn get_viewport_aggregation(&self, query: JsValue) -> Result<JsValue, JsValue> {
        let query: geo_viz_core::GeoVizViewportQuery =
            serde_wasm_bindgen::from_value(query).map_err(into_js_error)?;
        to_json_value(
            &self
                .inner
                .get_viewport_aggregation(query)
                .map_err(into_js_error)?,
        )
    }

    #[wasm_bindgen(js_name = getClusterExpansionZoom)]
    pub fn get_cluster_expansion_zoom(&self, cluster_id: usize) -> usize {
        self.inner.get_cluster_expansion_zoom(cluster_id)
    }

    #[wasm_bindgen(js_name = getClusterLeaves)]
    pub fn get_cluster_leaves(
        &self,
        cluster_id: usize,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<JsValue, JsValue> {
        to_json_value(&self.inner.get_cluster_leaves(
            cluster_id,
            limit.unwrap_or(10),
            offset.unwrap_or(0),
        ))
    }

    #[wasm_bindgen(js_name = getHeatFeatures)]
    pub fn get_heat_features(&self, query: JsValue, options: JsValue) -> Result<JsValue, JsValue> {
        let query: geo_viz_core::GeoVizViewportQuery =
            serde_wasm_bindgen::from_value(query).map_err(into_js_error)?;
        let options = if options.is_undefined() || options.is_null() {
            geo_viz_core::GeoVizHeatOptions {
                radius_meters: None,
                weight_metric: None,
            }
        } else {
            serde_wasm_bindgen::from_value(options).map_err(into_js_error)?
        };
        to_json_value(
            &self
                .inner
                .get_heat_features(query, options)
                .map_err(into_js_error)?,
        )
    }

    #[wasm_bindgen(js_name = nearestPoint)]
    pub fn nearest_point(&self, query: JsValue) -> Result<JsValue, JsValue> {
        let query: geo_viz_core::GeoVizNearestPointQuery =
            serde_wasm_bindgen::from_value(query).map_err(into_js_error)?;
        to_json_value(&self.inner.nearest_point(query).map_err(into_js_error)?)
    }
}

#[wasm_bindgen(js_class = GeoFlowIndex)]
impl WasmGeoFlowIndex {
    #[wasm_bindgen(constructor)]
    pub fn new(flows: JsValue) -> Result<WasmGeoFlowIndex, JsValue> {
        let flows: Vec<geo_viz_core::GeoVizFlow> =
            serde_wasm_bindgen::from_value(flows).map_err(into_js_error)?;
        let inner = geo_viz_core::GeoFlowIndex::new(flows).map_err(into_js_error)?;
        Ok(Self { inner })
    }

    #[wasm_bindgen(js_name = getBounds)]
    pub fn get_bounds(&self) -> Result<JsValue, JsValue> {
        to_json_value(&self.inner.get_bounds())
    }

    #[wasm_bindgen(js_name = getViewportFlows)]
    pub fn get_viewport_flows(&self, query: JsValue, options: JsValue) -> Result<JsValue, JsValue> {
        let query: geo_viz_core::GeoVizViewportQuery =
            serde_wasm_bindgen::from_value(query).map_err(into_js_error)?;
        let options = if options.is_undefined() || options.is_null() {
            geo_viz_core::GeoVizFlowOptions::default()
        } else {
            serde_wasm_bindgen::from_value(options).map_err(into_js_error)?
        };
        to_json_value(
            &self
                .inner
                .get_viewport_flows(query, options)
                .map_err(into_js_error)?,
        )
    }
}

#[wasm_bindgen(js_class = GeoJsonIndex)]
impl WasmGeoJsonIndex {
    #[wasm_bindgen(constructor)]
    pub fn new(geo_json: JsValue) -> Result<WasmGeoJsonIndex, JsValue> {
        let geo_json: serde_json::Value =
            serde_wasm_bindgen::from_value(geo_json).map_err(into_js_error)?;
        let inner = geo_viz_core::GeoJsonIndex::new(geo_json).map_err(into_js_error)?;
        Ok(Self { inner })
    }

    #[wasm_bindgen(js_name = getBounds)]
    pub fn get_bounds(&self) -> Result<JsValue, JsValue> {
        to_json_value(&self.inner.get_bounds())
    }

    #[wasm_bindgen(js_name = getViewportFeatures)]
    pub fn get_viewport_features(
        &self,
        query: JsValue,
        options: JsValue,
    ) -> Result<JsValue, JsValue> {
        let query: geo_viz_core::GeoVizViewportQuery =
            serde_wasm_bindgen::from_value(query).map_err(into_js_error)?;
        let options = if options.is_undefined() || options.is_null() {
            geo_viz_core::GeoVizGeoJsonOptions::default()
        } else {
            serde_wasm_bindgen::from_value(options).map_err(into_js_error)?
        };
        to_json_value(
            &self
                .inner
                .get_viewport_features(query, options)
                .map_err(into_js_error)?,
        )
    }
}

#[wasm_bindgen(js_name = packageSurface)]
pub fn package_surface() -> Result<JsValue, JsValue> {
    to_json_value(&geo_viz_core::surface::package_surface())
}

#[wasm_bindgen(js_name = runOperation)]
pub fn run_operation(request: JsValue) -> Result<JsValue, JsValue> {
    let request: SurfaceRequest = serde_wasm_bindgen::from_value(request).map_err(into_js_error)?;
    let response = geo_viz_core::surface::run_surface_operation(request).map_err(into_js_error)?;
    to_json_value(&response)
}

fn to_json_value(value: &impl Serialize) -> Result<JsValue, JsValue> {
    let json = serde_json::to_string(value).map_err(into_js_error)?;
    js_sys::JSON::parse(&json)
}

fn into_js_error(error: impl std::fmt::Display) -> JsValue {
    js_sys::Error::new(&error.to_string()).into()
}

#[cfg(test)]
mod tests {
    #[test]
    fn wrapped_surface_has_operations() {
        let surface = geo_viz_core::surface::package_surface();
        assert_eq!(surface.library, "geo-viz-core");
        assert!(!surface.operations.is_empty());
    }
}
