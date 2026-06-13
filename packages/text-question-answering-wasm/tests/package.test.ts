import { expect, test } from "bun:test";

test("text-question-answering-wasm package exports stable entrypoints", async () => {
  const entry = await import("../index.js");
  expect(typeof entry.init).toBe("function");
  expect(typeof entry.packageSurface).toBe("function");
  expect(typeof entry.runOperation).toBe("function");
});

test("runOperation answers from imported spans with structured output", async () => {
  const entry = await import("../index.js");
  const result = assertStructuredResponse(
    await entry.runOperation({
      operation: "qa.answer",
      input: {
        question: "What is reliable?",
        context: "Rust is reliable.",
        importedPredictions: [{ text: "Rust", score: 0.9 }],
      },
    }),
    "qa.answer",
  );
  expect(result.answers.length).toBeGreaterThan(0);
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
