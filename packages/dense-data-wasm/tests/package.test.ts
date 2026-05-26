import { expect, test } from "bun:test";

test("dense-data-wasm package exports stable entrypoints", async () => {
  const entry = await import("../index.js");
  expect(typeof entry.init).toBe("function");
  expect(typeof entry.packageSurface).toBe("function");
  expect(typeof entry.runOperation).toBe("function");
});

test("dense-data-wasm runs dense operations", async () => {
  const entry = await import("../index.js");
  const surface = entry.packageSurface();
  expect(surface.operations.map((operation) => operation.id)).toContain("summarizeDensePoints");

  const response = entry.runOperation({
    operation: "binNumericSeries",
    input: {
      points: [
        { index: 0, x: 0, y: 1, metrics: { count: 1 } },
        { index: 1, x: 1, y: 3, metrics: { count: 1 } },
      ],
      xDomain: [0, 2],
      targetBinCount: 2,
      includeEmptyBins: true,
    },
  });

  expect(response.value.bins).toHaveLength(2);
  expect(response.value.bins[0].pointCount).toBe(1);
});
