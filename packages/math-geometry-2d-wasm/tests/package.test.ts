import { expect, test } from "bun:test";

test("math-geometry-2d-wasm package exports stable entrypoints", async () => {
  const entry = await import("../index.js");
  expect(typeof entry.init).toBe("function");
  expect(typeof entry.packageSurface).toBe("function");
  expect(typeof entry.runOperation).toBe("function");
});

test("math-geometry-2d-wasm exposes new operations when generated pkg is present", async () => {
  const entry = await import("../index.js");
  try {
    const surface = await entry.packageSurface();
    expect(surface.operations.map((operation) => operation.id)).toContain("geometry.overlap");
    const response = await entry.runOperation({
      operation: "geometry.overlap",
      input: {
        left: { x: 0, y: 0, width: 2, height: 2 },
        right: { x: 1, y: 1, width: 2, height: 2 },
      },
    });
    expect(typeof response.value).toBe("object");
  } catch (error) {
    if (String(error).includes("/pkg/")) return;
    throw error;
  }
});
