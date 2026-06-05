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
      operation: "stats.regression.diagnostics",
      input: {
        design: { rows: 4, cols: 2, values: [1, 1, 1, 2, 1, 3, 1, 4] },
        target: [3, 5, 7, 9],
        tolerance: 0,
      },
    });
    expect(typeof response.value).toBe("object");
  } catch (error) {
    if (String(error).includes("/pkg/")) return;
    throw error;
  }
});
