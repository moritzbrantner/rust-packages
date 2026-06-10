import { expect, test } from "bun:test";

test("math-statistics-wasm package exports stable entrypoints", async () => {
  const entry = await import("../index.js");
  expect(typeof entry.init).toBe("function");
  expect(typeof entry.packageSurface).toBe("function");
  expect(typeof entry.runOperation).toBe("function");
});

test("math-statistics-wasm exposes new operations when generated pkg is present", async () => {
  const entry = await import("../index.js");
  try {
    const surface = await entry.packageSurface();
    expect(surface.operations.map((operation) => operation.id)).toContain("stats.regression.diagnostics");
    const response = await entry.runOperation({
      operation: "stats.regression.ols",
      input: {
        design: { rows: 4, cols: 2, values: [1, 2, 2, 4, 3, 6, 4, 8] },
        target: [1, 2, 3, 4],
      },
    });
    expect(response.value.precision).toBe("f64");
  } catch (error) {
    if (String(error).includes("/pkg/")) return;
    throw error;
  }
});
