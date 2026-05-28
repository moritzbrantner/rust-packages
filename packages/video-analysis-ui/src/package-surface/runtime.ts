import type {
  HealthPayload,
  ModelCatalogEntry,
  PackageAppConfig,
  PackageSurface,
  RuntimeMode,
  SurfaceResponse,
} from "./types";

export function configuredServerBaseUrl(config: PackageAppConfig): string {
  const key = config.server?.baseUrlEnv ?? "VITE_SERVER_URL";
  const env = (import.meta as ImportMeta & { env?: Record<string, string | undefined> }).env;
  return env?.[key] ?? env?.VITE_SERVER_URL ?? "http://127.0.0.1:3000";
}

export async function initializeWasmSurface(config: PackageAppConfig): Promise<PackageSurface> {
  if (!config.wasm) {
    throw new Error("No WASM runtime is configured for this package.");
  }
  await config.wasm.init();
  return config.wasm.packageSurface();
}

export async function fetchHealth(config: PackageAppConfig, mode: RuntimeMode): Promise<HealthPayload> {
  return fetchPackageJson<HealthPayload>(config, mode, "/health");
}

export async function fetchServerSurface(config: PackageAppConfig, mode: RuntimeMode): Promise<PackageSurface> {
  const metadata = await fetchPackageJson<{ library: string; version?: string; operations?: unknown[]; capabilities?: unknown }>(
    config,
    mode,
    "/api/package",
  );
  return {
    library: metadata.library,
    version: metadata.version ?? "0.1.0",
    operations: normalizeOperations(metadata.operations ?? []),
    capabilities: metadata.capabilities ?? {},
  };
}

export async function fetchModelCatalog(config: PackageAppConfig, mode: RuntimeMode): Promise<ModelCatalogEntry[]> {
  try {
    const models = await fetchPackageJson<unknown[]>(config, mode, "/api/models");
    return normalizeModelCatalog(models);
  } catch {
    return [];
  }
}

export async function runOperation(
  config: PackageAppConfig,
  mode: RuntimeMode,
  operation: string,
  input: unknown,
): Promise<SurfaceResponse> {
  if (mode === "client-wasm") {
    if (!config.wasm) {
      throw new Error("No WASM runtime is configured for this package.");
    }
    return config.wasm.runOperation({ operation, input });
  }
  const response = await fetchPackageRoute(config, mode, "/api/run", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ operation, input }),
  });
  if (!response.ok) {
    const body = await response.text();
    throw new Error(body || `Server returned ${response.status}`);
  }
  return response.json() as Promise<SurfaceResponse>;
}

async function fetchPackageJson<T>(config: PackageAppConfig, mode: RuntimeMode, path: string): Promise<T> {
  const response = await fetchPackageRoute(config, mode, path);
  if (!response.ok) {
    throw new Error(`Server returned ${response.status}`);
  }
  return response.json() as Promise<T>;
}

async function fetchPackageRoute(
  config: PackageAppConfig,
  mode: RuntimeMode,
  path: string,
  init?: RequestInit,
): Promise<Response> {
  const serverBaseUrl = configuredServerBaseUrl(config);
  if (mode === "standalone-server") {
    const standaloneRoute = config.server?.standaloneRoute ?? "";
    return fetch(`${serverBaseUrl}${standaloneRoute}${path}`, init);
  }
  const scopedRoute = config.server?.scopedRoute ?? `/api/rust/packages/${config.library}`;
  const scopedResponse = await fetch(`${serverBaseUrl}${scopedRoute}${path}`, init);
  if (scopedResponse.status !== 404) {
    return scopedResponse;
  }
  return fetch(`${serverBaseUrl}${path}`, init);
}

function normalizeOperations(input: unknown[]): PackageSurface["operations"] {
  return input
    .filter((value): value is Record<string, unknown> => Boolean(value) && typeof value === "object")
    .map((value) => ({
      id: String(value.id ?? ""),
      name: String(value.name ?? value.id ?? "Operation"),
      description: typeof value.description === "string" ? value.description : undefined,
      inputSchema: value.inputSchema ?? value.input_schema ?? {},
      outputSchema: value.outputSchema ?? value.output_schema ?? {},
      exampleRequest: value.exampleRequest ?? value.example_request ?? {},
      wasmSupported: Boolean(value.wasmSupported ?? value.wasm_supported ?? false),
      serverSupported: Boolean(value.serverSupported ?? value.server_supported ?? false),
    }))
    .filter((operation) => operation.id.length > 0);
}

export function normalizeModelCatalog(input: unknown[]): ModelCatalogEntry[] {
  return input
    .filter((value): value is Record<string, unknown> => Boolean(value) && typeof value === "object")
    .map((value) => ({
      id: String(value.id ?? value.modelId ?? value.model_id ?? ""),
      label: String(value.label ?? value.id ?? value.modelId ?? value.model_id ?? "Model"),
      task: String(value.task ?? "general"),
      runtime: normalizeRuntime(value.runtime),
      supported: Boolean(value.supported ?? false),
      fallback: stringField(value.fallback),
      requiredFeature: stringField(value.requiredFeature ?? value.required_feature),
      source: stringField(value.source ?? value.modelId ?? value.model_id),
      note: stringField(value.note),
    }))
    .filter((model) => model.id.length > 0);
}

function normalizeRuntime(value: unknown): ModelCatalogEntry["runtime"] {
  const runtime = String(value ?? "heuristic").replace("-", "_");
  if (
    runtime === "deterministic" ||
    runtime === "heuristic" ||
    runtime === "candle" ||
    runtime === "onnx" ||
    runtime === "whisper_cpp" ||
    runtime === "opencv" ||
    runtime === "comfyui" ||
    runtime === "external"
  ) {
    return runtime;
  }
  return "heuristic";
}

function stringField(value: unknown): string | undefined {
  return typeof value === "string" && value.length > 0 ? value : undefined;
}

