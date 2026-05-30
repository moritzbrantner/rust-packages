import { expect, test } from "bun:test";

test("geo-io-geojson-wasm package exports stable entrypoints", async () => {
  const entry = await import("../index.js");
  expect(typeof entry.init).toBe("function");
  expect(typeof entry.packageSurface).toBe("function");
  expect(typeof entry.runOperation).toBe("function");

  const surface = await entry.packageSurface();
  expect(surface.library).toBe("moritzbrantner-geo-io-geojson");

  const describe = await entry.runOperation({
    operation: "describe",
    input: {},
  });
  expect(describe.operation).toBe("describe");

  const converted = await entry.runOperation({
    operation: "geoJson.toGeoJson",
    input: { geometry: { type: "Point", coordinates: [8, 49] } },
  });
  expect(converted.value.geoJson.type).toBe("Point");
});
