import { expect, test } from "bun:test";

test("geo-viz-wasm package exports stable entrypoints", async () => {
  const entry = await import("../index.js");
  expect(typeof entry.init).toBe("function");
  expect(typeof entry.packageSurface).toBe("function");
  expect(typeof entry.runOperation).toBe("function");
  expect(typeof entry.GeoPointIndex).toBe("function");
});

test("GeoPointIndex aggregates viewport features", async () => {
  const { GeoPointIndex } = await import("../index.js");
  const index = new GeoPointIndex([
    { id: "a", longitude: 13, latitude: 52, metrics: { value: 2 } },
    { id: "b", longitude: 13.0001, latitude: 52.0001, metrics: { value: 3 } },
  ]);
  const aggregation = index.getViewportAggregation({
    bounds: [12.9, 51.9, 13.1, 52.1],
    zoom: 1,
  });

  expect(aggregation.summary.visiblePointCount).toBe(2);
  expect(aggregation.summary.metrics.value).toBe(5);
});
