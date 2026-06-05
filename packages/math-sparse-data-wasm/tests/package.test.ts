import { expect, test } from "bun:test";

test("math-sparse-data-wasm package exports stable entrypoints", async () => {
  const entry = await import("../index.js");
  expect(typeof entry.init).toBe("function");
  expect(typeof entry.packageSurface).toBe("function");
  expect(typeof entry.runOperation).toBe("function");
});

test("math-sparse-data-wasm exposes new operations when generated pkg is present", async () => {
  const entry = await import("../index.js");
  try {
    const surface = await entry.packageSurface();
    expect(surface.operations.map((operation) => operation.id)).toContain("sparse.vectorOps");
    const response = await entry.runOperation({
      operation: "sparse.vectorOps",
      input: { vector: { dimensions: 3, indices: [0, 2], values: [1, -2] }, topK: 1 },
    });
    expect(typeof response.value).toBe("object");
  } catch (error) {
    if (String(error).includes("/pkg/")) return;
    throw error;
  }
});
