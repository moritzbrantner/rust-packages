import { expect, test } from "bun:test";

test("geo-io-osm-wasm package exports stable entrypoints", async () => {
  const entry = await import("../index.js");
  expect(typeof entry.init).toBe("function");
  expect(typeof entry.packageSurface).toBe("function");
  expect(typeof entry.runOperation).toBe("function");

  const surface = await entry.packageSurface();
  expect(surface.library).toBe("moritzbrantner-geo-io-osm");

  const describe = await entry.runOperation({
    operation: "describe",
    input: {},
  });
  expect(describe.operation).toBe("describe");

  const validated = await entry.runOperation({
    operation: "osm.validateSpec",
    input: { spec: { filter: { types: ["node"] } } },
  });
  expect(validated.value.valid).toBe(true);
});
