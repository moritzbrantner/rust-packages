import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/text-retrieval-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "text-retrieval",
  title: "Text Retrieval",
  description: "Library-first semantic and hybrid retrieval for video-analysis.",
  domain: "text",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/text-retrieval",
    standaloneRoute: "",
  },
  defaultOperation: "retrieval.search",
  featuredOperations: ["retrieval.search", "retrieval.chunk", "retrieval.rerank", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run transient retrieval chunking, search, and reranking workflows.",
      operations: ["retrieval.search", "retrieval.chunk", "retrieval.rerank"],
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
      id: "hybrid-search",
      label: "Search",
      operation: "retrieval.search",
      description: "Build and search a transient in-memory hybrid retrieval index.",
      input: {
        documents: [
          { id: "doc-1", body: "Rust text retrieval" },
          { id: "doc-2", body: "Video scene reports" },
        ],
        query: "text",
        mode: "hybrid",
      },
    },
    {
      id: "rerank",
      label: "Rerank",
      operation: "retrieval.rerank",
      description: "Rerank query/document pairs with lexical overlap.",
      input: { query: "rust", documents: ["rust text", "video scenes"] },
    },
  ],
  benchmarkScenarios: [
    {
      id: "chunk",
      label: "Chunk",
      operation: "retrieval.chunk",
      input: { text: "Rust text retrieval chunks transcript content for search. ".repeat(20), maxChunkTokens: 24, overlapTokens: 4 },
      iterations: 100,
      warmupIterations: 5,
      outputCountPath: ["chunks"],
    },
    {
      id: "full-text-search",
      label: "Full-text Search",
      operation: "retrieval.search",
      input: {
        documents: [
          { id: "doc-1", body: "rust text retrieval" },
          { id: "doc-2", body: "video scene reports" },
          { id: "doc-3", body: "transcript search and chunks" },
        ],
        query: "text search",
        mode: "fullText",
      },
      iterations: 80,
      warmupIterations: 5,
      outputCountPath: ["results"],
    },
    {
      id: "hybrid-search",
      label: "Hybrid Search",
      operation: "retrieval.search",
      input: {
        documents: [
          { id: "doc-1", body: "rust text retrieval" },
          { id: "doc-2", body: "video scene reports" },
          { id: "doc-3", body: "transcript search and chunks" },
        ],
        query: "text search",
        mode: "hybrid",
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
