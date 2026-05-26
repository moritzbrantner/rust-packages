import { expect, test } from "bun:test";

test("dense-data-wasm package exports stable entrypoints", async () => {
  const entry = await import("../index.js");
  expect(typeof entry.init).toBe("function");
  expect(typeof entry.NumericSeriesIndex).toBe("function");
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

test("dense-data-wasm indexes numeric series for repeated queries", async () => {
  const entry = await import("../index.js");
  const index = new entry.NumericSeriesIndex([
    { sourceIndex: 0, x: 10, y: 6, metrics: { count: 1 } },
    { sourceIndex: 1, x: Number.NaN, y: 100, metrics: { count: 100 } },
    { sourceIndex: 2, x: 0, y: 2, metrics: { count: 1 } },
    { sourceIndex: 3, x: 1, y: 4, metrics: { count: 1 } },
  ]);

  expect(index.getSeriesBounds()).toEqual({
    maxX: 10,
    maxY: 6,
    minX: 0,
    minY: 2,
  });

  const response = index.getBinnedSeries({
    xDomain: [10, 0],
    targetBinCount: 2,
    includeEmptyBins: true,
  });

  expect(response.bins).toHaveLength(2);
  expect(response.bins[0].pointCount).toBe(2);
  expect(response.bins[0].firstPointIndex).toBe(2);
  expect(response.bins[0].lastPointIndex).toBe(3);
  expect(response.bins[0].metrics.count).toBe(2);
});
