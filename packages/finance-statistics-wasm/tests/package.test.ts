import { expect, test } from "bun:test";

test("finance-statistics-wasm package exports stable entrypoints", async () => {
  const entry = await import("../index.js");
  expect(typeof entry.init).toBe("function");
  expect(typeof entry.packageSurface).toBe("function");
  expect(typeof entry.runOperation).toBe("function");
});

test("finance-statistics-wasm exposes new operations when generated pkg is present", async () => {
  const entry = await import("../index.js");
  try {
    const surface = await entry.packageSurface();
    expect(surface.operations.map((operation) => operation.id)).toContain("finance.portfolio");
    const response = await entry.runOperation({
      operation: "finance.portfolio",
      input: {
        assetReturns: [
          [0.02, -0.01, 0.03],
          [0.01, 0, 0.02],
        ],
        weights: [0.6, 0.4],
        confidence: 0.8,
      },
    });
    expect(typeof response.value).toBe("object");
  } catch (error) {
    if (String(error).includes("/pkg/")) return;
    throw error;
  }
});
