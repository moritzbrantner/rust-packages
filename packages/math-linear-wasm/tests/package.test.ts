import { expect, test } from "bun:test";

test("math-linear-wasm package exports stable entrypoints", async () => {
  const entry = await import("../index.js");
  expect(typeof entry.init).toBe("function");
  expect(typeof entry.packageSurface).toBe("function");
  expect(typeof entry.runOperation).toBe("function");
});

test("math-linear-wasm exposes new operations when generated pkg is present", async () => {
  const entry = await import("../index.js");
  try {
    const surface = await entry.packageSurface();
    expect(surface.operations.map((operation) => operation.id)).toContain("linear.leastSquares");
    const response = await entry.runOperation({
      operation: "linear.leastSquares",
      input: {
        matrix: { rows: 3, cols: 2, values: [1, 1, 1, 2, 1, 3] },
        target: [3, 5, 7],
        tolerance: 0,
      },
    });
    expect(typeof response.value).toBe("object");
  } catch (error) {
    if (String(error).includes("/pkg/")) return;
    throw error;
  }
});
