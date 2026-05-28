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
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
