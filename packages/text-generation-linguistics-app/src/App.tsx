import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/text-generation-linguistics-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "text-generation-linguistics",
  title: "Text Generation Linguistics",
  description: "Adapters from text-linguistics analysis outputs into text-generation.",
  domain: "text",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/text-generation-linguistics",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
