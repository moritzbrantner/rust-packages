export interface PackageMetadata {
  package: string;
  surface: string;
  library: string;
  libraryImport: string;
  cliPackage: string;
  appPackage: string;
  endpoints: string[];
}

export interface HealthPayload {
  ok: boolean;
  package: string;
  library: string;
}

const configuredServerUrl = import.meta.env.VITE_SERVER_URL as string | undefined;

export const serverBaseUrl = configuredServerUrl ?? "http://127.0.0.1:3000";
export const wrappedLibrary = "image-analysis-io";

export async function fetchHealth(): Promise<HealthPayload> {
  return fetchJson<HealthPayload>("/health");
}

export async function fetchPackageMetadata(): Promise<PackageMetadata> {
  return fetchJson<PackageMetadata>("/api/package");
}

export async function runOperation(input: string): Promise<unknown> {
  const response = await fetch(`${serverBaseUrl}/api/run`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: input,
  });
  if (!response.ok) {
    throw new Error(`Server returned ${response.status}`);
  }
  return response.json() as Promise<unknown>;
}

async function fetchJson<T>(path: string): Promise<T> {
  const response = await fetch(`${serverBaseUrl}${path}`);
  if (!response.ok) {
    throw new Error(`Server returned ${response.status}`);
  }
  return response.json() as Promise<T>;
}
