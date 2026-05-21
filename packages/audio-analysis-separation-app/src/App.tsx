import { FormEvent, useEffect, useMemo, useState } from "react";

import {
  fetchHealth,
  fetchPackageMetadata,
  runOperation,
  serverBaseUrl,
  wrappedLibrary,
  type HealthPayload,
  type PackageMetadata,
} from "./api";

type LoadState = "idle" | "loading" | "ready" | "error";
const packageDescription = "Demucs-based audio stem separation command wrapper for video-analysis.";

export function App() {
  const [loadState, setLoadState] = useState<LoadState>("idle");
  const [health, setHealth] = useState<HealthPayload | null>(null);
  const [metadata, setMetadata] = useState<PackageMetadata | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [input, setInput] = useState('{"operation":"introspect"}');
  const [result, setResult] = useState<string>("");

  useEffect(() => {
    void refresh();
  }, []);

  const statusLabel = useMemo(() => {
    if (loadState === "ready" && health?.ok) {
      return "Online";
    }
    if (loadState === "error") {
      return "Offline";
    }
    return "Checking";
  }, [health?.ok, loadState]);

  async function refresh() {
    setLoadState("loading");
    setError(null);
    try {
      const [nextHealth, nextMetadata] = await Promise.all([fetchHealth(), fetchPackageMetadata()]);
      setHealth(nextHealth);
      setMetadata(nextMetadata);
      setLoadState("ready");
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Unable to reach the server");
      setLoadState("error");
    }
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    try {
      const payload = await runOperation(input);
      setResult(JSON.stringify(payload, null, 2));
    } catch (caught) {
      setResult("");
      setError(caught instanceof Error ? caught.message : "Operation failed");
    }
  }

  return (
    <main className="min-h-screen bg-zinc-50 text-zinc-950">
      <section className="border-b border-zinc-200 bg-white">
        <div className="mx-auto flex max-w-6xl flex-col gap-4 px-5 py-5 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <p className="text-xs font-semibold uppercase tracking-wide text-teal-700">
              Package app
            </p>
            <h1 className="mt-1 text-2xl font-semibold">Audio Analysis Separation</h1>
            <p className="mt-2 max-w-3xl text-sm leading-6 text-zinc-600">{packageDescription}</p>
          </div>
          <div className="flex items-center gap-3">
            <span
              className={`status-pill ${loadState === "ready" ? "status-online" : loadState === "error" ? "status-offline" : "status-pending"}`}
            >
              {statusLabel}
            </span>
            <button className="button-secondary" type="button" onClick={refresh}>
              Refresh
            </button>
          </div>
        </div>
      </section>

      <section className="mx-auto grid max-w-6xl gap-5 px-5 py-6 lg:grid-cols-[minmax(0,1fr)_360px]">
        <form className="panel" onSubmit={submit}>
          <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
            <div>
              <h2 className="section-title">API operation</h2>
              <p className="section-copy">POST payload for audio-analysis-separation-server.</p>
            </div>
            <button className="button-primary" type="submit">
              Run
            </button>
          </div>
          <textarea
            className="code-input mt-4"
            spellCheck={false}
            value={input}
            onChange={(event) => setInput(event.target.value)}
          />
          {result ? <pre className="result-block">{result}</pre> : null}
          {error ? <p className="error-text">{error}</p> : null}
        </form>

        <aside className="space-y-5">
          <section className="panel">
            <h2 className="section-title">Server</h2>
            <dl className="detail-list">
              <div>
                <dt>URL</dt>
                <dd>{serverBaseUrl}</dd>
              </div>
              <div>
                <dt>Health</dt>
                <dd>{health?.package ?? "Not loaded"}</dd>
              </div>
            </dl>
          </section>

          <section className="panel">
            <h2 className="section-title">Package</h2>
            <dl className="detail-list">
              <div>
                <dt>Library</dt>
                <dd>{metadata?.library ?? wrappedLibrary}</dd>
              </div>
              <div>
                <dt>Import</dt>
                <dd>{metadata?.libraryImport ?? "Loading"}</dd>
              </div>
              <div>
                <dt>CLI</dt>
                <dd>{metadata?.cliPackage ?? `${wrappedLibrary}-cli`}</dd>
              </div>
              <div>
                <dt>App</dt>
                <dd>{metadata?.appPackage ?? `${wrappedLibrary}-app`}</dd>
              </div>
            </dl>
          </section>

          <section className="panel">
            <h2 className="section-title">Endpoints</h2>
            <ul className="endpoint-list">
              {(
                metadata?.endpoints ?? [
                  "GET /health",
                  "GET /api/package",
                  "GET /api/schema",
                  "POST /api/run",
                ]
              ).map((endpoint) => (
                <li key={endpoint}>{endpoint}</li>
              ))}
            </ul>
          </section>
        </aside>
      </section>
    </main>
  );
}
