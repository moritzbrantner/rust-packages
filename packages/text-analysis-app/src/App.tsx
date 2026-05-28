import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/text-analysis-wasm";

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
      label: "Corpus",
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
  ],
  resultTabs: [
    { id: "overview", label: "Overview", select: (response) => summarizeTextAnalysis(response.value) },
    { id: "stats", label: "Stats", select: (response) => selectObject(response.value, ["core", "enrichedStats"]) },
    { id: "lexical", label: "Lexical", select: (response) => getObject(response.value).lexical ?? {} },
    { id: "embedding", label: "Embedding", select: (response) => getObject(response.value).embedding ?? {} },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}

function summarizeTextAnalysis(value: unknown) {
  const result = getObject(value);
  return {
    id: result.id,
    language: result.language,
    diagnostics: result.diagnostics,
    words: getObject(getObject(getObject(result.core).stats).basic).words,
    keywords: Array.isArray(getObject(result.lexical).keywords) ? getObject(result.lexical).keywords.slice(0, 5) : [],
  };
}

function selectObject(value: unknown, keys: string[]) {
  const object = getObject(value);
  return Object.fromEntries(keys.map((key) => [key, object[key]]));
}

function getObject(value: unknown): Record<string, any> {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as Record<string, any>) : {};
}
