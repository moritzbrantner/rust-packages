import { useEffect, useMemo, useState } from "react";

import { FileInputs } from "./FileInputs";
import { ModelSelector } from "./ModelSelector";
import { OperationWorkbench } from "./OperationWorkbench";
import { ResultViewer } from "./ResultViewer";
import { builtInVideoFileInput } from "./samples";
import {
  configuredServerBaseUrl,
  fetchHealth,
  fetchModelCatalog,
  fetchServerSurface,
  initializeWasmSurface,
  runOperation,
} from "./runtime";
import type {
  HealthPayload,
  ModelCatalogEntry,
  PackageAppConfig,
  PackageAppPreset,
  PackageSurfaceWorkbenchContext,
  PackageSurface,
  RuntimeMode,
  SurfaceOperation,
  SurfaceResponse,
} from "./types";

export * from "./types";
export * from "./runtime";
export * from "./samples";
export { FileInputs } from "./FileInputs";
export { ModelSelector } from "./ModelSelector";
export { OperationWorkbench } from "./OperationWorkbench";
export { ResultViewer } from "./ResultViewer";

type LoadState = "loading" | "ready" | "error" | "disabled";

export function PackageSurfaceWorkbench({ config }: { config: PackageAppConfig }) {
  const runtimeFromUrl = readRuntimeMode(config);
  const [runtimeMode, setRuntimeMode] = useState<RuntimeMode>(runtimeFromUrl);
  const [wasmState, setWasmState] = useState<LoadState>(config.wasm ? "loading" : "disabled");
  const [serverState, setServerState] = useState<LoadState>("loading");
  const [health, setHealth] = useState<HealthPayload | null>(null);
  const [surface, setSurface] = useState<PackageSurface | null>(null);
  const [models, setModels] = useState<ModelCatalogEntry[]>([]);
  const [selectedModel, setSelectedModel] = useState("");
  const [selectedOperation, setSelectedOperation] = useState(() => readQuery("operation") ?? config.defaultOperation ?? "describe");
  const [input, setInput] = useState("{}");
  const [response, setResponse] = useState<SurfaceResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [running, setRunning] = useState(false);

  useEffect(() => {
    let cancelled = false;
    if (config.wasm) {
      initializeWasmSurface(config)
        .then((nextSurface) => {
          if (cancelled) return;
          setSurface((current) => current ?? nextSurface);
          initializeSelection(nextSurface, selectedOperation, setSelectedOperation, setInput, config.defaultOperation);
          setWasmState("ready");
        })
        .catch(() => {
          if (!cancelled) setWasmState("error");
        });
    }

    Promise.all([
      fetchHealth(config, "overview-server"),
      fetchServerSurface(config, "overview-server"),
      fetchModelCatalog(config, "overview-server"),
    ])
      .then(([nextHealth, nextSurface, nextModels]) => {
        if (cancelled) return;
        setHealth(nextHealth);
        setSurface((current) => current ?? nextSurface);
        initializeSelection(nextSurface, selectedOperation, setSelectedOperation, setInput, config.defaultOperation);
        setModels(nextModels);
        setSelectedModel(nextModels[0]?.id ?? "");
        setServerState(nextHealth.ok === false ? "error" : "ready");
      })
      .catch(() => {
        if (!cancelled) setServerState("error");
      });

    return () => {
      cancelled = true;
    };
  }, [config]);

  const operations = useMemo(() => orderedOperations(surface?.operations ?? [], config.featuredOperations), [surface, config.featuredOperations]);
  const operation = useMemo(
    () => operations.find((candidate) => candidate.id === selectedOperation) ?? operations[0] ?? null,
    [operations, selectedOperation],
  );
  const parsedInput = useMemo(() => parseInputOrNull(input), [input]);
  const wasmAvailable = Boolean(config.wasm) && wasmState === "ready";
  const overviewServerAvailable = serverState === "ready";
  const selectedOperationRuntimeSupported = operationSupportsRuntime(operation, runtimeMode);
  const selectedRuntimeAvailable =
    runtimeMode === "client-wasm"
      ? wasmAvailable
      : runtimeMode === "overview-server"
        ? overviewServerAvailable
        : true;
  const runDisabledReason = runtimeDisabledReason(
    runtimeMode,
    wasmAvailable,
    overviewServerAvailable,
    operations.length,
    operation,
  );
  const canRun = selectedRuntimeAvailable && selectedOperationRuntimeSupported && operations.length > 0;

  useEffect(() => {
    if (operation && !selectedOperation) {
      chooseOperation(operation.id);
    }
  }, [operation, selectedOperation]);

  useEffect(() => {
    if (runtimeMode === "client-wasm" && wasmState === "error" && overviewServerAvailable) {
      chooseRuntime("overview-server");
      return;
    }
    if (runtimeMode === "client-wasm" && operation && !operation.wasmSupported && overviewServerAvailable) {
      chooseRuntime("overview-server");
      return;
    }
    if (runtimeMode === "overview-server" && serverState === "error" && wasmAvailable) {
      chooseRuntime("client-wasm");
    }
  }, [operation, overviewServerAvailable, runtimeMode, serverState, wasmAvailable, wasmState]);

  function chooseRuntime(nextMode: RuntimeMode) {
    setRuntimeMode(nextMode);
    writeQuery({ runtime: nextMode });
  }

  function chooseOperation(nextOperation: string) {
    setSelectedOperation(nextOperation);
    writeQuery({ operation: nextOperation });
    const next = operations.find((candidate) => candidate.id === nextOperation);
    setInput(storedInput(config.library, nextOperation) ?? JSON.stringify(next?.exampleRequest ?? {}, null, 2));
    setResponse(null);
    setError(null);
  }

  function applyPreset(preset: PackageAppPreset) {
    setSelectedOperation(preset.operation);
    setInput(JSON.stringify(preset.input, null, 2));
    writeQuery({ operation: preset.operation });
    persistInput(config.library, preset.operation, JSON.stringify(preset.input, null, 2));
    setResponse(null);
    setError(null);
  }

  function patchInput(path: string[], value: unknown) {
    setInput((currentInput) => {
      try {
        const parsed = JSON.parse(currentInput || "{}") as unknown;
        const patched = patchValue(parsed, path, value);
        const nextInput = JSON.stringify(patched, null, 2);
        persistInput(config.library, selectedOperation, nextInput);
        return nextInput;
      } catch (caught) {
        setError(caught instanceof Error ? caught.message : String(caught));
        return currentInput;
      }
    });
  }

  function setInputValue(value: unknown) {
    const nextInput = JSON.stringify(value, null, 2);
    setInput(nextInput);
    persistInput(config.library, selectedOperation, nextInput);
  }

  async function run() {
    if (!canRun) {
      setError(runDisabledReason ?? "No runnable runtime is available for this package.");
      return;
    }
    setRunning(true);
    setResponse(null);
    setError(null);
    try {
      const payload = JSON.parse(input || "{}");
      persistInput(config.library, selectedOperation, input);
      const result = await runOperation(config, runtimeMode, selectedOperation, payload);
      setResponse(result);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Operation failed");
    } finally {
      setRunning(false);
    }
  }

  const childContext: PackageSurfaceWorkbenchContext = {
    input: parsedInput ?? {},
    inputJson: input,
    response,
    selectedOperation,
    runtimeMode,
    patchInput,
    setInput: setInputValue,
    setInputJson: (nextInput) => {
      setInput(nextInput);
      persistInput(config.library, selectedOperation, nextInput);
    },
  };
  const children = typeof config.children === "function" ? config.children(childContext) : config.children;

  return (
    <main className="min-h-screen bg-zinc-50 text-zinc-950">
      <section className="border-b border-zinc-200 bg-white">
        <div className="mx-auto flex max-w-screen-2xl flex-col gap-4 px-5 py-5 lg:flex-row lg:items-center lg:justify-between">
          <div className="min-w-0">
            <p className="text-xs font-semibold uppercase tracking-wide text-teal-700">Package workbench</p>
            <h1 className="mt-1 break-words text-2xl font-semibold">{config.title}</h1>
            <p className="mt-2 max-w-4xl text-sm leading-6 text-zinc-600">{config.description}</p>
          </div>
          <RuntimeButtons
            config={config}
            operation={operation}
            runtimeMode={runtimeMode}
            serverState={serverState}
            wasmState={wasmState}
            onRuntimeMode={chooseRuntime}
          />
        </div>
      </section>

      <section className="mx-auto grid max-w-screen-2xl gap-5 px-5 py-5 xl:grid-cols-[minmax(380px,0.8fr)_minmax(0,1.2fr)_360px]">
        <div className="space-y-5">
          <OperationWorkbench
            canRun={canRun}
            error={error}
            input={input}
            operation={operation}
            operations={operations}
            presets={config.presets}
            running={running}
            runDisabledReason={runDisabledReason}
            selectedOperation={selectedOperation}
            onInputChange={(nextInput) => {
              setInput(nextInput);
              persistInput(config.library, selectedOperation, nextInput);
            }}
            onPreset={applyPreset}
            onRun={() => void run()}
            onSelectOperation={chooseOperation}
          />
          {children}
        </div>
        <ResultViewer response={response} resultTabs={config.resultTabs} />
        <aside className="space-y-5">
          <RuntimePanel
            config={config}
            health={health}
            serverState={serverState}
            surface={surface}
            wasmState={wasmState}
          />
          <ModelSelector models={models} selectedModel={selectedModel} onSelectModel={setSelectedModel} />
          <FileInputs definitions={config.fileInputs ?? defaultFileInputs(config.domain)} onPatch={patchInput} />
          <SupportPanel operations={operations} />
        </aside>
      </section>
    </main>
  );
}

function RuntimeButtons({
  config,
  operation,
  runtimeMode,
  serverState,
  wasmState,
  onRuntimeMode,
}: {
  config: PackageAppConfig;
  operation: SurfaceOperation | null;
  runtimeMode: RuntimeMode;
  serverState: LoadState;
  wasmState: LoadState;
  onRuntimeMode: (mode: RuntimeMode) => void;
}) {
  return (
    <div className="inline-grid overflow-hidden rounded-md border border-zinc-300 bg-white sm:grid-cols-3" role="group" aria-label="Runtime mode">
      <ModeButton
        active={runtimeMode === "client-wasm"}
        disabled={!config.wasm || wasmState === "error" || operation?.wasmSupported === false}
        onClick={() => onRuntimeMode("client-wasm")}
      >
        Client WASM
      </ModeButton>
      <ModeButton
        active={runtimeMode === "overview-server"}
        disabled={serverState === "error" || operation?.serverSupported === false}
        onClick={() => onRuntimeMode("overview-server")}
      >
        Overview Server
      </ModeButton>
      <ModeButton active={runtimeMode === "standalone-server"} disabled={operation?.serverSupported === false} onClick={() => onRuntimeMode("standalone-server")}>
        Standalone Server
      </ModeButton>
    </div>
  );
}

function ModeButton(props: { active: boolean; children: string; disabled?: boolean; onClick: () => void }) {
  return (
    <button
      className={
        props.active
          ? "px-3 py-2 text-sm font-medium bg-zinc-950 text-white"
          : "px-3 py-2 text-sm font-medium text-zinc-700 transition hover:bg-zinc-100 disabled:cursor-not-allowed disabled:opacity-50"
      }
      disabled={props.disabled}
      type="button"
      onClick={props.onClick}
    >
      {props.children}
    </button>
  );
}

function RuntimePanel({
  config,
  health,
  serverState,
  surface,
  wasmState,
}: {
  config: PackageAppConfig;
  health: HealthPayload | null;
  serverState: LoadState;
  surface: PackageSurface | null;
  wasmState: LoadState;
}) {
  return (
    <section className="rounded-md border border-zinc-200 bg-white p-4">
      <h2 className="text-sm font-semibold text-zinc-950">Runtime</h2>
      <dl className="mt-3 grid gap-3 text-sm">
        <StatusRow label="WASM" state={wasmState} />
        <StatusRow label="Server" state={serverState} />
        <DetailRow label="Server URL" value={configuredServerBaseUrl(config)} />
        <DetailRow label="Health" value={health?.package ?? "Not loaded"} />
        <DetailRow label="Library" value={surface?.library ?? config.library} />
        <DetailRow label="Operations" value={String(surface?.operations.length ?? 0)} />
        {health?.requiredFeature ? <DetailRow label="Feature" value={health.requiredFeature} /> : null}
      </dl>
    </section>
  );
}

function SupportPanel({ operations }: { operations: SurfaceOperation[] }) {
  return (
    <section className="rounded-md border border-zinc-200 bg-white p-4">
      <h2 className="text-sm font-semibold text-zinc-950">Support</h2>
      <ul className="mt-3 space-y-2 font-mono text-xs text-zinc-800">
        {operations.map((candidate) => (
          <li key={candidate.id} className="rounded-md bg-zinc-50 p-2">
            {candidate.id} · WASM {candidate.wasmSupported ? "yes" : "no"} · server {candidate.serverSupported ? "yes" : "no"}
          </li>
        ))}
      </ul>
    </section>
  );
}

function StatusRow({ label, state }: { label: string; state: LoadState }) {
  return <DetailRow label={label} value={state === "ready" ? "Ready" : state === "error" ? "Unavailable" : state === "disabled" ? "Disabled" : "Loading"} />;
}

function DetailRow({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="text-xs font-semibold uppercase text-zinc-500">{label}</dt>
      <dd className="mt-1 break-words font-mono text-zinc-900">{value}</dd>
    </div>
  );
}

function runtimeDisabledReason(
  runtimeMode: RuntimeMode,
  wasmAvailable: boolean,
  overviewServerAvailable: boolean,
  operationCount: number,
  operation: SurfaceOperation | null,
): string | undefined {
  if (!wasmAvailable && !overviewServerAvailable && runtimeMode !== "standalone-server") {
    return "No runnable runtime is available for this package.";
  }
  if (operationCount === 0) {
    return "No operations are available for this package.";
  }
  if (runtimeMode === "client-wasm" && !wasmAvailable) {
    return "Client WASM is unavailable. Use Overview Server or build the generated WASM package.";
  }
  if (runtimeMode === "client-wasm" && operation?.wasmSupported === false) {
    return "This operation is server-only. Use Overview Server or Standalone Server.";
  }
  if (runtimeMode === "overview-server" && !overviewServerAvailable) {
    return "Overview Server is unavailable. Start the dev server with bun run dev.";
  }
  if ((runtimeMode === "overview-server" || runtimeMode === "standalone-server") && operation?.serverSupported === false) {
    return "This operation is not supported by the selected server runtime.";
  }
  return undefined;
}

function operationSupportsRuntime(operation: SurfaceOperation | null, runtimeMode: RuntimeMode): boolean {
  if (!operation) {
    return true;
  }
  if (runtimeMode === "client-wasm") {
    return operation.wasmSupported;
  }
  return operation.serverSupported;
}

function orderedOperations(operations: SurfaceOperation[], featured?: string[]): SurfaceOperation[] {
  if (!featured?.length) {
    return operations;
  }
  const rank = new Map(featured.map((operation, index) => [operation, index]));
  return [...operations].sort((left, right) => (rank.get(left.id) ?? 999) - (rank.get(right.id) ?? 999));
}

function initializeSelection(
  surface: PackageSurface,
  current: string,
  setSelectedOperation: (operation: string) => void,
  setInput: (input: string) => void,
  defaultOperation?: string,
) {
  const operation =
    surface.operations.find((candidate) => candidate.id === current) ??
    surface.operations.find((candidate) => candidate.id === defaultOperation) ??
    surface.operations[0];
  if (!operation) {
    return;
  }
  setSelectedOperation(operation.id);
  setInput(storedInput(surface.library, operation.id) ?? JSON.stringify(operation.exampleRequest ?? {}, null, 2));
}

function readRuntimeMode(config: PackageAppConfig): RuntimeMode {
  const runtime = readQuery("runtime");
  if (runtime === "client-wasm" || runtime === "overview-server" || runtime === "standalone-server") {
    if (runtime === "client-wasm" && !config.wasm) {
      return "overview-server";
    }
    return runtime;
  }
  if (config.defaultRuntime) {
    if (config.defaultRuntime === "client-wasm" && !config.wasm) {
      return "overview-server";
    }
    return config.defaultRuntime;
  }
  return config.wasm ? "client-wasm" : "overview-server";
}

function parseInputOrNull(input: string): unknown | null {
  try {
    return JSON.parse(input || "{}");
  } catch {
    return null;
  }
}

function readQuery(key: string): string | null {
  if (typeof window === "undefined") {
    return null;
  }
  return new URLSearchParams(window.location.search).get(key);
}

function writeQuery(values: Record<string, string>) {
  if (typeof window === "undefined") {
    return;
  }
  const url = new URL(window.location.href);
  for (const [key, value] of Object.entries(values)) {
    url.searchParams.set(key, value);
  }
  window.history.replaceState({}, "", url);
}

function storageKey(library: string, operation: string): string {
  return `package-workbench:${library}:${operation}`;
}

function storedInput(library: string, operation: string): string | null {
  try {
    return localStorage.getItem(storageKey(library, operation));
  } catch {
    return null;
  }
}

function persistInput(library: string, operation: string, input: string) {
  try {
    localStorage.setItem(storageKey(library, operation), input);
  } catch {
    return;
  }
}

function patchValue(input: unknown, path: string[], value: unknown): unknown {
  const root = input && typeof input === "object" && !Array.isArray(input) ? { ...(input as Record<string, unknown>) } : {};
  let cursor: Record<string, unknown> = root;
  for (const segment of path.slice(0, -1)) {
    const next = cursor[segment];
    const object = next && typeof next === "object" && !Array.isArray(next) ? { ...(next as Record<string, unknown>) } : {};
    cursor[segment] = object;
    cursor = object;
  }
  const last = path[path.length - 1];
  if (last) {
    cursor[last] = value;
  }
  return root;
}

function defaultFileInputs(domain: PackageAppConfig["domain"]) {
  if (domain === "image") {
    return [{ id: "image", label: "Image input", accept: "image/*", targetPath: ["imageDataUrl"] }];
  }
  if (domain === "audio") {
    return [{ id: "audio", label: "Audio input", accept: "audio/*", targetPath: ["audioDataUrl"] }];
  }
  if (domain === "video") {
    return [builtInVideoFileInput()];
  }
  if (domain === "comfyui") {
    return [{ id: "workflow", label: "Workflow JSON", accept: "application/json,.json", targetPath: ["workflow"], encoding: "text" as const }];
  }
  return [];
}
