import type { WorkspaceArchitectureResponse } from "./workspaceArchitecture";

export async function fetchWorkspaceArchitecture(signal?: AbortSignal): Promise<WorkspaceArchitectureResponse> {
  const base = import.meta.env.BASE_URL || "/";
  const urls = import.meta.env.DEV
    ? ["/api/workspace-architecture", `${base}workspace-architecture.json`]
    : [`${base}workspace-architecture.json`, "/api/workspace-architecture"];
  let lastError: unknown = null;

  for (const url of urls) {
    try {
      const response = await fetch(url, { signal });
      if (!response.ok) {
        throw new Error(`${url} returned ${response.status}`);
      }
      return (await response.json()) as WorkspaceArchitectureResponse;
    } catch (error) {
      if (signal?.aborted) {
        throw error;
      }
      lastError = error;
    }
  }

  throw lastError instanceof Error ? lastError : new Error("Could not load workspace architecture");
}
