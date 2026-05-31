import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/text-embeddings-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "text-embeddings",
  title: "Text Embeddings",
  description: "Lightweight semantic text embeddings and search for video-analysis.",
  domain: "text",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/text-embeddings",
    standaloneRoute: "",
  },
  defaultOperation: "embeddings.embed",
  featuredOperations: ["embeddings.embed", "embeddings.similarity", "embeddings.semanticSearch", "embeddings.relatedTerms", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run deterministic embedding, similarity, semantic search, and related-term workflows.",
      operations: ["embeddings.embed", "embeddings.similarity", "embeddings.semanticSearch", "embeddings.relatedTerms"],
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
      id: "embed",
      label: "Embed",
      operation: "embeddings.embed",
      description: "Build deterministic hashed embeddings.",
      input: { texts: ["rust text analysis"], dimensions: 64 },
    },
    {
      id: "semantic-search",
      label: "Search",
      operation: "embeddings.semanticSearch",
      description: "Search a transient hashed semantic index.",
      input: {
        documents: [
          { id: "doc-1", text: "rust text analysis" },
          { id: "doc-2", text: "video scenes" },
        ],
        query: "text",
      },
    },
  ],
  benchmarkScenarios: [
    {
      id: "hashed-embed",
      label: "Hashed Embed",
      operation: "embeddings.embed",
      input: { texts: ["rust text analysis", "semantic transcript retrieval", "video scene reports"], dimensions: 128 },
      iterations: 80,
      warmupIterations: 5,
      outputCountPath: ["embeddings"],
    },
    {
      id: "semantic-search",
      label: "Semantic Search",
      operation: "embeddings.semanticSearch",
      input: {
        documents: [
          { id: "doc-1", text: "rust text analysis" },
          { id: "doc-2", text: "semantic search over transcripts" },
          { id: "doc-3", text: "video scene boundary reports" },
        ],
        query: "text search",
      },
      iterations: 60,
      warmupIterations: 5,
      outputCountPath: ["results"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
