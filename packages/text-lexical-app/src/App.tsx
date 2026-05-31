import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/text-lexical-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "text-lexical",
  title: "Text Lexical",
  description: "Lexical text features, corpus statistics, TF-IDF, and BM25 for video-analysis.",
  domain: "text",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/text-lexical",
    standaloneRoute: "",
  },
  defaultOperation: "lexical.analyze",
  featuredOperations: ["lexical.analyze", "lexical.keywords", "lexical.corpusSearch", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run lexical analysis, keyword extraction, and transient corpus search.",
      operations: ["lexical.analyze", "lexical.keywords", "lexical.corpusSearch"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect package metadata and operation support.",
      operations: ["describe"],
    },
  ],
  presets: [
    {
      id: "lexical-analysis",
      label: "Analyze",
      operation: "lexical.analyze",
      description: "Compute deterministic lexical features for a short text.",
      input: { text: "Rust crates make text analysis reliable.", maxTerms: 5 },
    },
    {
      id: "corpus-search",
      label: "Corpus",
      operation: "lexical.corpusSearch",
      description: "Search a transient BM25 corpus.",
      input: {
        documents: [
          { id: "doc-1", text: "rust text analysis" },
          { id: "doc-2", text: "video scene analysis" },
        ],
        query: "text",
        mode: "bm25",
      },
    },
  ],
  benchmarkScenarios: [
    {
      id: "keywords",
      label: "Keywords",
      operation: "lexical.keywords",
      input: { text: "Rust text analysis supports transcript retrieval and lexical search. ".repeat(24), maxTerms: 12 },
      iterations: 100,
      warmupIterations: 5,
      outputCountPath: ["keywords"],
    },
    {
      id: "corpus-search",
      label: "Corpus Search",
      operation: "lexical.corpusSearch",
      input: {
        documents: [
          { id: "doc-1", text: "rust text analysis" },
          { id: "doc-2", text: "video scene analysis" },
          { id: "doc-3", text: "transcript retrieval and search" },
        ],
        query: "text search",
        mode: "bm25",
      },
      iterations: 80,
      warmupIterations: 5,
      outputCountPath: ["results"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
