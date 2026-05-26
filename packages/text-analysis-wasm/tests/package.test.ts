import { analyzeDocument, compareTexts, packageSurface } from "../index.js";

test("packageSurface lists document analysis", async () => {
  const surface = await packageSurface();
  expect(surface.operations.map((operation) => operation.id)).toContain("analysis.document");
});

test("analyzeDocument returns stats, keywords, and fingerprints", async () => {
  const report = await analyzeDocument({
    id: "doc-1",
    text: "Rust crates analyze text. Rust text analysis is deterministic.",
  });
  expect(report.core.stats.basic.words).toBeGreaterThan(0);
  expect(report.lexical.keywords.length).toBeGreaterThan(0);
  expect(report.similarity.tokenShingleSimhash).toBeDefined();
});

test("compareTexts returns token shingle jaccard", async () => {
  const report = await compareTexts({
    left: "scene transitions follow motion",
    right: "scene transitions follow dialogue",
    n: 2,
    mode: "token",
  });
  expect(report.similarity.jaccard).toBeGreaterThan(0);
});
