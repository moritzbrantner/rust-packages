import { FormEvent, useEffect, useMemo, useState } from "react";

import {
  fetchHealth,
  fetchServerSurface,
  initializeWasm,
  runOperation,
  serverBaseUrl,
  wrappedLibrary,
  type HealthPayload,
  type PackageSurface,
  type RuntimeMode,
  type SurfaceOperation,
} from "./api";

type LoadState = "loading" | "ready" | "error";
const packageDescription = "Typed radiance project loading, validation, summaries, and CPU previews for video-analysis.";

export function App() {
  const [runtimeMode, setRuntimeMode] = useState<RuntimeMode>("client-wasm");
  const [wasmState, setWasmState] = useState<LoadState>("loading");
  const [serverState, setServerState] = useState<LoadState>("loading");
  const [health, setHealth] = useState<HealthPayload | null>(null);
  const [surface, setSurface] = useState<PackageSurface | null>(null);
  const [selectedOperation, setSelectedOperation] = useState("describe");
  const [input, setInput] = useState("{}");
  const [result, setResult] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    initializeWasm()
      .then((nextSurface) => {
        setSurface(nextSurface);
        setSelectedOperation(nextSurface.operations[0]?.id ?? "describe");
        setInput(JSON.stringify(nextSurface.operations[0]?.exampleRequest ?? {}, null, 2));
        setWasmState("ready");
      })
      .catch((caught) => {
        setError(caught instanceof Error ? caught.message : String(caught));
        setWasmState("error");
      });

    Promise.all([fetchHealth(), fetchServerSurface()])
      .then(([nextHealth, serverSurface]) => {
        setHealth(nextHealth);
        setSurface((current) => current ?? serverSurface);
        setServerState("ready");
      })
      .catch(() => setServerState("error"));
  }, []);

  const operation = useMemo(
    () => surface?.operations.find((candidate) => candidate.id === selectedOperation) ?? surface?.operations[0],
    [selectedOperation, surface?.operations],
  );

  function chooseOperation(nextOperation: string) {
    setSelectedOperation(nextOperation);
    const metadata = surface?.operations.find((candidate) => candidate.id === nextOperation);
    setInput(JSON.stringify(metadata?.exampleRequest ?? {}, null, 2));
    setResult("");
    setError(null);
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setResult("");
    try {
      const payload = JSON.parse(input || "{}");
      const response = await runOperation(runtimeMode, selectedOperation, payload);
      setResult(JSON.stringify(response, null, 2));
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Operation failed");
    }
  }

  return (
    <main className="min-h-screen bg-zinc-50 text-zinc-950">
      <section className="border-b border-zinc-200 bg-white">
        <div className="mx-auto flex max-w-6xl flex-col gap-4 px-5 py-5 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <p className="text-xs font-semibold uppercase tracking-wide text-teal-700">Package surface app</p>
            <h1 className="mt-1 text-2xl font-semibold">Video Analysis Radiance Pipeline</h1>
            <p className="mt-2 max-w-3xl text-sm leading-6 text-zinc-600">{packageDescription}</p>
          </div>
          <div className="segmented-control" role="group" aria-label="Runtime mode">
            <ModeButton active={runtimeMode === "client-wasm"} onClick={() => setRuntimeMode("client-wasm")}>
              Client WASM
            </ModeButton>
            <ModeButton active={runtimeMode === "server"} onClick={() => setRuntimeMode("server")}>
              Server API
            </ModeButton>
          </div>
        </div>
      </section>

      <section className="mx-auto grid max-w-6xl gap-5 px-5 py-6 lg:grid-cols-[minmax(0,1fr)_360px]">
        <form className="panel" onSubmit={submit}>
          <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
            <label className="grid flex-1 gap-1 text-sm">
              <span className="text-xs font-semibold uppercase tracking-wide text-zinc-500">Operation</span>
              <select
                className="rounded-md border border-zinc-300 px-3 py-2"
                value={selectedOperation}
                onChange={(event) => chooseOperation(event.target.value)}
              >
                {(surface?.operations ?? []).map((candidate) => (
                  <option key={candidate.id} value={candidate.id}>
                    {candidate.name}
                  </option>
                ))}
              </select>
            </label>
            <button className="button-primary" type="submit">
              Run
            </button>
          </div>
          <p className="section-copy mt-3">{operation?.description ?? `Run ${wrappedLibrary} operation.`}</p>
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
            <h2 className="section-title">Runtime</h2>
            <dl className="detail-list">
              <StatusRow label="WASM" state={wasmState} />
              <StatusRow label="Server" state={serverState} />
              <div>
                <dt>Server URL</dt>
                <dd>{serverBaseUrl}</dd>
              </div>
              <div>
                <dt>Health</dt>
                <dd>{health?.package ?? "Not loaded"}</dd>
              </div>
            </dl>
          </section>

          <section className="panel">
            <h2 className="section-title">Surface</h2>
            <dl className="detail-list">
              <div>
                <dt>Library</dt>
                <dd>{surface?.library ?? wrappedLibrary}</dd>
              </div>
              <div>
                <dt>Operations</dt>
                <dd>{surface?.operations.length ?? 0}</dd>
              </div>
              <div>
                <dt>Selected</dt>
                <dd>{selectedOperation}</dd>
              </div>
            </dl>
          </section>

          <section className="panel">
            <h2 className="section-title">Support</h2>
            <ul className="endpoint-list">
              {(surface?.operations ?? []).map((candidate: SurfaceOperation) => (
                <li key={candidate.id}>
                  {candidate.id} · WASM {candidate.wasmSupported ? "yes" : "no"} · server 
                  {candidate.serverSupported ? "yes" : "no"}
                </li>
              ))}
            </ul>
          </section>
        </aside>
      </section>
    </main>
  );
}

function ModeButton(props: { active: boolean; children: string; onClick: () => void }) {
  return (
    <button className={props.active ? "mode-button mode-button-active" : "mode-button"} type="button" onClick={props.onClick}>
      {props.children}
    </button>
  );
}

function StatusRow(props: { label: string; state: LoadState }) {
  return (
    <div>
      <dt>{props.label}</dt>
      <dd>{props.state === "ready" ? "Ready" : props.state === "error" ? "Unavailable" : "Loading"}</dd>
    </div>
  );
}
