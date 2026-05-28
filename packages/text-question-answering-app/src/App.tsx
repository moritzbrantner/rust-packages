import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/text-question-answering-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "text-question-answering",
  title: "Text Question Answering",
  description: "Question answering APIs, imported span postprocessing, and deterministic fallback handling.",
  domain: "text",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/text-question-answering",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
