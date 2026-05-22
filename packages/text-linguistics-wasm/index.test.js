import { beforeAll, expect, test } from "bun:test";

import init, { analyzeTextLinguistics } from "./index.js";

beforeAll(async () => {
  await init();
});

test("analyzes linguistics through wasm heuristic mode", () => {
  const analysis = analyzeTextLinguistics("Alice works at OpenAI in Berlin.", {
    profile: "balanced",
    entityRecognition: "heuristic",
  });

  expect(analysis.model.entityRecognition).toBe("heuristic");
  expect(analysis.summary.tokenCount).toBeGreaterThan(0);
  expect(analysis.summary.entityCount).toBeGreaterThan(0);
});

test("accepts BERT-NER token predictions through wasm", () => {
  const analysis = analyzeTextLinguistics("Alice works at OpenAI.", {
    profile: "balanced",
    bertNerPredictions: [
      {
        kind: "token",
        label: "B-PER",
        text: "Alice",
        score: 0.99,
        attributes: { byte_start: "0", byte_end: "5", token_index: "1" },
      },
      {
        kind: "token",
        label: "B-ORG",
        text: "OpenAI",
        score: 0.98,
        attributes: { byte_start: "15", byte_end: "21", token_index: "4" },
      },
    ],
  });

  expect(analysis.model).toMatchObject({
    entityRecognition: "client-wasm-predictions",
    entityModel: "bert-base-ner",
  });
  expect(analysis.entities.map((entity) => entity.text)).toContain("Alice");
  expect(analysis.entities.map((entity) => entity.text)).toContain("OpenAI");
});
