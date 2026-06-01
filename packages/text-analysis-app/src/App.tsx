import { createTextResultTabs, PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/text-analysis-wasm";

const sampleText =
  "Alice presented the tokenizer roadmap in Berlin. Rust crates analyze text with deterministic local features. Semantic search and lexical statistics support transcript workflows.";

const packageAppConfig: PackageAppConfig = {
  library: "text-analysis",
  title: "Text Analysis",
  description: "Unified text analysis orchestration for video-analysis.",
  domain: "text",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/text-analysis",
    standaloneRoute: "",
  },
  defaultOperation: "analysis.document",
  featuredOperations: ["analysis.document", "analysis.corpus", "analysis.similarity", "analysis.describe", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run document, corpus, and text-similarity analysis workflows.",
      operations: ["analysis.document", "analysis.corpus", "analysis.similarity"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect package metadata and operation support.",
      operations: ["analysis.describe", "describe"],
    },
  ],
  presets: [
    {
      id: "document-deterministic",
      label: "Document",
      operation: "analysis.document",
      description: "Deterministic document analysis with lexical, linguistic, and embedding sections.",
      input: {
        id: "app-input",
        text: sampleText,
        profile: "deterministic",
        keywordLimit: 10,
        summarySentences: 3,
        ngramSizes: [2, 3],
        shingleSizes: [3, 5],
        linguistics: { mode: "heuristicBalanced" },
        embedding: { mode: "hashed", dimensions: 128, useIdf: false },
      },
    },
    {
      id: "document-model-backed",
      label: "Model-backed",
      operation: "analysis.document",
      description: "Server-oriented model-backed profile with deterministic fallback metadata.",
      input: {
        id: "model-backed-input",
        text: sampleText,
        profile: "modelBacked",
        keywordLimit: 10,
        summarySentences: 3,
        linguistics: { mode: "modelBacked" },
        embedding: { mode: "hashed", dimensions: 128, useIdf: false },
      },
    },
    {
      id: "corpus",
      label: "Corpus retrieval report",
      operation: "analysis.corpus",
      description: "Transient corpus search and similarity report.",
      input: {
        documents: [
          { id: "doc-1", text: "rust text analysis" },
          { id: "doc-2", text: "video scene analysis" },
          { id: "doc-3", text: "semantic search over transcripts" },
        ],
        query: "text analysis",
        topK: 10,
        includeNearDuplicates: true,
        includeSemanticNeighbors: true,
        embedding: { mode: "hashed", dimensions: 128, useIdf: true },
      },
    },
    {
      id: "similarity",
      label: "Similarity overlap and embedding",
      operation: "analysis.similarity",
      description: "Compare two transcript-style passages with lexical and deterministic embedding signals.",
      input: {
        left: "Rust text packages extract keywords, entities, and transcript retrieval evidence.",
        right: "Transcript search in Rust combines lexical features with deterministic semantic embeddings.",
        n: 3,
        mode: "token",
      },
    },
  ],
  benchmarkScenarios: [
    {
      id: "document-report",
      label: "Document Report",
      operation: "analysis.document",
      input: {
        id: "bench-doc",
        text: sampleText.repeat(6),
        profile: "deterministic",
        keywordLimit: 12,
        summarySentences: 3,
        embedding: { mode: "hashed", dimensions: 128, useIdf: false },
      },
      iterations: 20,
      warmupIterations: 3,
      outputCountPath: ["summary"],
    },
    {
      id: "corpus-report",
      label: "Corpus Report",
      operation: "analysis.corpus",
      input: {
        documents: [
          { id: "doc-1", text: sampleText },
          { id: "doc-2", text: "Scene reports and transcript retrieval share lexical search." },
          { id: "doc-3", text: "Embeddings support semantic discovery over captions." },
        ],
        query: "text retrieval",
        topK: 5,
        includeSemanticNeighbors: true,
        embedding: { mode: "hashed", dimensions: 128, useIdf: true },
      },
      iterations: 15,
      warmupIterations: 2,
      outputCountPath: ["results"],
    },
  ],
  resultTabs: createTextResultTabs({
    library: "text-analysis",
    primaryOperations: {
      "analysis.document": {
        title: "Document analysis",
        summaryFields: ["profile", "language", "wordCount", "keywordCount", "entityCount", "embeddingDimensions"],
        listFields: ["lexical.keywords", "linguistics.entities", "retrieval.results", "diagnostics"],
        objectFields: ["core", "lexical", "linguistics", "embedding", "retrieval"],
        explanation: () => "The orchestrator ran core statistics, lexical keyword extraction, linguistic projections, deterministic embeddings, and retrieval-oriented diagnostics for one document.",
      },
      "analysis.corpus": {
        title: "Corpus analysis",
        summaryFields: ["documentCount", "resultCount", "nearDuplicateCount", "semanticNeighborCount"],
        listFields: ["results", "nearDuplicates", "semanticNeighbors", "documents"],
        objectFields: ["embedding", "diagnostics"],
        explanation: () => "The app built a transient corpus from the sample documents, searched it, and added near-duplicate or semantic-neighbor sections when requested.",
      },
      "analysis.similarity": {
        title: "Text similarity",
        summaryFields: ["mode", "score", "lexicalScore", "embeddingScore"],
        listFields: ["sharedTerms", "diagnostics"],
        objectFields: ["lexical", "embedding", "result"],
        explanation: () => "The run compared two passages with the selected similarity mode and reports the lexical and embedding contributions that were available.",
      },
    },
  }),
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
