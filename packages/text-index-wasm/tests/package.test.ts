import { expect, test } from "bun:test";

test("text-index-wasm package exports stable entrypoints", async () => {
  const entry = await import("../index.js");
  expect(typeof entry.init).toBe("function");
  expect(typeof entry.packageSurface).toBe("function");
  expect(typeof entry.runOperation).toBe("function");
});

test("runOperation searches an in-memory index with structured output", async () => {
  const entry = await import("../index.js");
  const result = assertStructuredResponse(
    await entry.runOperation({
      operation: "index.search",
      input: {
        documents: [{ id: "doc-1", body: "rust text index search" }],
        query: { text: "text index", topK: 2 },
      },
    }),
    "index.search",
  );
  expect(result.results.length).toBeGreaterThan(0);
});

function assertStructuredResponse(response: any, operation: string) {
  expect(response.operation).toBe(operation);
  expect(response.value.operation).toBe(operation);
  expect(typeof response.value.title).toBe("string");
  expect(typeof response.value.message).toBe("string");
  expect(response.value.summary.status).toBe("ok");
  expect(response.value.result).toBeDefined();
  return response.value.result;
}
