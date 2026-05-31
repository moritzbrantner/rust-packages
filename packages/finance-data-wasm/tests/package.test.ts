import { expect, test } from "bun:test";

test("finance-data-wasm package exports operation-specific entrypoints", async () => {
  const entry = await import("../index.js");
  expect(typeof entry.init).toBe("function");
  expect(typeof entry.packageSurface).toBe("function");
  expect(typeof entry.runOperation).toBe("function");
  expect(typeof entry.createSeriesIndex).toBe("function");
});
