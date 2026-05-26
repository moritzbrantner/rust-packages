import { init, packageSurface, runOperation as runWasmOperation } from "@mb-rust/text-retrieval-wasm";

export type RuntimeMode = "client-wasm" | "server";

export interface SurfaceOperation {
  id: string;
  name: string;
  description?: string;
  inputSchema: unknown;
  outputSchema: unknown;
  exampleRequest: unknown;
  wasmSupported: boolean;
  serverSupported: boolean;
}

export interface PackageSurface {
  library: string;
  version: string;
  operations: SurfaceOperation[];
  capabilities: unknown;
}

export interface SurfaceResponse {
  operation: string;
  value: unknown;
  diagnostics: unknown[];
  artifacts: unknown[];
}

export interface HealthPayload {
  ok: boolean;
  package: string;
  library: string;
}

const configuredServerUrl = import.meta.env.VITE_SERVER_URL as string | undefined;

export const serverBaseUrl = configuredServerUrl ?? "http://127.0.0.1:3000";
export const wrappedLibrary = "text-retrieval";

export async function initializeWasm(): Promise<PackageSurface> {
  await init();
  return packageSurface();
}

export async function fetchHealth(): Promise<HealthPayload> {
  return fetchPackageJson<HealthPayload>("/health");
}

export async function fetchServerSurface(): Promise<PackageSurface> {
  const metadata = await fetchPackageJson<{ operations: SurfaceOperation[]; library: string }>("/api/package");
  return {
    library: metadata.library,
    version: "0.1.0",
    operations: metadata.operations ?? [],
    capabilities: {},
  };
}

export async function runOperation(mode: RuntimeMode, operation: string, input: unknown): Promise<SurfaceResponse> {
  if (mode === "client-wasm") {
    return runWasmOperation({ operation, input });
  }
  const response = await fetchPackageRoute("/api/run", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ operation, input }),
  });
  if (!response.ok) {
    throw new Error(`Server returned ${response.status}`);
  }
  return response.json() as Promise<SurfaceResponse>;
}

async function fetchPackageJson<T>(path: string): Promise<T> {
  const response = await fetchPackageRoute(path);
  if (!response.ok) {
    throw new Error(`Server returned ${response.status}`);
  }
  return response.json() as Promise<T>;
}

async function fetchPackageRoute(path: string, init?: RequestInit): Promise<Response> {
  const scopedResponse = await fetch(`${serverBaseUrl}${packageRoute(path)}`, init);
  if (scopedResponse.status !== 404) {
    return scopedResponse;
  }
  return fetch(`${serverBaseUrl}${path}`, init);
}

function packageRoute(path: string): string {
  return `/api/rust/packages/${wrappedLibrary}${path}`;
}
