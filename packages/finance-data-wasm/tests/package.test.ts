import { expect, test } from "bun:test";

test("finance-data-wasm package exports operation-specific entrypoints", async () => {
  const entry = await import("../index.js");
  expect(typeof entry.init).toBe("function");
  expect(typeof entry.packageSurface).toBe("function");
  expect(typeof entry.runOperation).toBe("function");
  expect(typeof entry.createSeriesIndex).toBe("function");
  expect(typeof entry.FinanceDataSeriesIndex).toBe("function");
  expect(typeof entry.packageSurfaceSync).toBe("function");
});

test("FinanceDataSeriesIndex exposes compact returns synchronously", async () => {
  const { FinanceDataSeriesIndex } = await import("../index.js");
  const index = new FinanceDataSeriesIndex({
    bars: [
      { timestampMs: 1, open: 99, high: 101, low: 98, close: 100 },
      { timestampMs: 2, open: 100, high: 112, low: 99, close: 110 },
      { timestampMs: 3, open: 110, high: 111, low: 104, close: 105 },
    ],
    instrument: { assetClass: "equity", id: "AAPL", symbol: "AAPL" },
  });

  expect(
    index.getCompactReturns({ startMs: 1, endMs: 3, method: "simple", targetCount: 2 }).summary,
  ).toEqual({
    pointCount: 2,
    sampleCount: 2,
    xDomain: [1, 3],
  });
});
