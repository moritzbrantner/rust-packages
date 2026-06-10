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
    const operationIds = surface.operations.map((operation) => operation.id);
    expect(operationIds).toContain("linear.leastSquares");
    expect(operationIds).toContain("linear.svd");
    expect(operationIds).toContain("linear.pseudoinverse");
    expect(operationIds).toContain("linear.rank");
    const response = await entry.runOperation({
      operation: "linear.rank",
      input: {
        matrix: { rows: 3, cols: 2, values: [1, 2, 2, 4, 3, 6] },
        tolerance: 1e-8,
      },
    });
    expect(response.value.rank).toBe(1);
  } catch (error) {
    if (String(error).includes("/pkg/")) return;
    throw error;
  }
});
