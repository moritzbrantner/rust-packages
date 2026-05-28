import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { useEffect, useMemo, useState } from "react";
import { FileInputs } from "./FileInputs";
import { ModelSelector } from "./ModelSelector";
import { OperationWorkbench } from "./OperationWorkbench";
import { ResultViewer } from "./ResultViewer";
import { configuredServerBaseUrl, fetchHealth, fetchModelCatalog, fetchServerSurface, initializeWasmSurface, runOperation, } from "./runtime";
export * from "./types";
export * from "./runtime";
export { FileInputs } from "./FileInputs";
export { ModelSelector } from "./ModelSelector";
export { OperationWorkbench } from "./OperationWorkbench";
export { ResultViewer } from "./ResultViewer";
export function PackageSurfaceWorkbench({ config }) {
    const runtimeFromUrl = readRuntimeMode(config);
    const [runtimeMode, setRuntimeMode] = useState(runtimeFromUrl);
    const [wasmState, setWasmState] = useState(config.wasm ? "loading" : "disabled");
    const [serverState, setServerState] = useState("loading");
    const [health, setHealth] = useState(null);
    const [surface, setSurface] = useState(null);
    const [models, setModels] = useState([]);
    const [selectedModel, setSelectedModel] = useState("");
    const [selectedOperation, setSelectedOperation] = useState(() => readQuery("operation") ?? config.defaultOperation ?? "describe");
    const [input, setInput] = useState("{}");
    const [response, setResponse] = useState(null);
    const [error, setError] = useState(null);
    const [running, setRunning] = useState(false);
    useEffect(() => {
        let cancelled = false;
        if (config.wasm) {
            initializeWasmSurface(config)
                .then((nextSurface) => {
                if (cancelled)
                    return;
                setSurface((current) => current ?? nextSurface);
                initializeSelection(nextSurface, selectedOperation, setSelectedOperation, setInput, config.defaultOperation);
                setWasmState("ready");
            })
                .catch(() => {
                if (!cancelled)
                    setWasmState("error");
            });
        }
        Promise.all([
            fetchHealth(config, "overview-server"),
            fetchServerSurface(config, "overview-server"),
            fetchModelCatalog(config, "overview-server"),
        ])
            .then(([nextHealth, nextSurface, nextModels]) => {
            if (cancelled)
                return;
            setHealth(nextHealth);
            setSurface((current) => current ?? nextSurface);
            initializeSelection(nextSurface, selectedOperation, setSelectedOperation, setInput, config.defaultOperation);
            setModels(nextModels);
            setSelectedModel(nextModels[0]?.id ?? "");
            setServerState(nextHealth.ok === false ? "error" : "ready");
        })
            .catch(() => {
            if (!cancelled)
                setServerState("error");
        });
        return () => {
            cancelled = true;
        };
    }, [config]);
    const operations = useMemo(() => orderedOperations(surface?.operations ?? [], config.featuredOperations), [surface, config.featuredOperations]);
    const operation = useMemo(() => operations.find((candidate) => candidate.id === selectedOperation) ?? operations[0] ?? null, [operations, selectedOperation]);
    useEffect(() => {
        if (operation && !selectedOperation) {
            chooseOperation(operation.id);
        }
    }, [operation, selectedOperation]);
    function chooseRuntime(nextMode) {
        setRuntimeMode(nextMode);
        writeQuery({ runtime: nextMode });
    }
    function chooseOperation(nextOperation) {
        setSelectedOperation(nextOperation);
        writeQuery({ operation: nextOperation });
        const next = operations.find((candidate) => candidate.id === nextOperation);
        setInput(storedInput(config.library, nextOperation) ?? JSON.stringify(next?.exampleRequest ?? {}, null, 2));
        setResponse(null);
        setError(null);
    }
    function applyPreset(preset) {
        setSelectedOperation(preset.operation);
        setInput(JSON.stringify(preset.input, null, 2));
        writeQuery({ operation: preset.operation });
        persistInput(config.library, preset.operation, JSON.stringify(preset.input, null, 2));
        setResponse(null);
        setError(null);
    }
    function patchInput(path, value) {
        try {
            const parsed = JSON.parse(input || "{}");
            const patched = patchValue(parsed, path, value);
            const nextInput = JSON.stringify(patched, null, 2);
            setInput(nextInput);
            persistInput(config.library, selectedOperation, nextInput);
        }
        catch (caught) {
            setError(caught instanceof Error ? caught.message : String(caught));
        }
    }
    async function run() {
        setRunning(true);
        setResponse(null);
        setError(null);
        try {
            const payload = JSON.parse(input || "{}");
            persistInput(config.library, selectedOperation, input);
            const result = await runOperation(config, runtimeMode, selectedOperation, payload);
            setResponse(result);
        }
        catch (caught) {
            setError(caught instanceof Error ? caught.message : "Operation failed");
        }
        finally {
            setRunning(false);
        }
    }
    return (_jsxs("main", { className: "min-h-screen bg-zinc-50 text-zinc-950", children: [_jsx("section", { className: "border-b border-zinc-200 bg-white", children: _jsxs("div", { className: "mx-auto flex max-w-screen-2xl flex-col gap-4 px-5 py-5 lg:flex-row lg:items-center lg:justify-between", children: [_jsxs("div", { className: "min-w-0", children: [_jsx("p", { className: "text-xs font-semibold uppercase tracking-wide text-teal-700", children: "Package workbench" }), _jsx("h1", { className: "mt-1 break-words text-2xl font-semibold", children: config.title }), _jsx("p", { className: "mt-2 max-w-4xl text-sm leading-6 text-zinc-600", children: config.description })] }), _jsx(RuntimeButtons, { config: config, runtimeMode: runtimeMode, serverState: serverState, wasmState: wasmState, onRuntimeMode: chooseRuntime })] }) }), _jsxs("section", { className: "mx-auto grid max-w-screen-2xl gap-5 px-5 py-5 xl:grid-cols-[minmax(380px,0.8fr)_minmax(0,1.2fr)_360px]", children: [_jsxs("div", { className: "space-y-5", children: [_jsx(OperationWorkbench, { error: error, input: input, operation: operation, operations: operations, presets: config.presets, running: running, selectedOperation: selectedOperation, onInputChange: (nextInput) => {
                                    setInput(nextInput);
                                    persistInput(config.library, selectedOperation, nextInput);
                                }, onPreset: applyPreset, onRun: () => void run(), onSelectOperation: chooseOperation }), config.children] }), _jsx(ResultViewer, { response: response, resultTabs: config.resultTabs }), _jsxs("aside", { className: "space-y-5", children: [_jsx(RuntimePanel, { config: config, health: health, serverState: serverState, surface: surface, wasmState: wasmState }), _jsx(ModelSelector, { models: models, selectedModel: selectedModel, onSelectModel: setSelectedModel }), _jsx(FileInputs, { definitions: config.fileInputs ?? defaultFileInputs(config.domain), onPatch: patchInput }), _jsx(SupportPanel, { operations: operations })] })] })] }));
}
function RuntimeButtons({ config, runtimeMode, serverState, wasmState, onRuntimeMode, }) {
    return (_jsxs("div", { className: "inline-grid overflow-hidden rounded-md border border-zinc-300 bg-white sm:grid-cols-3", role: "group", "aria-label": "Runtime mode", children: [_jsx(ModeButton, { active: runtimeMode === "client-wasm", disabled: !config.wasm || wasmState === "error", onClick: () => onRuntimeMode("client-wasm"), children: "Client WASM" }), _jsx(ModeButton, { active: runtimeMode === "overview-server", disabled: serverState === "error", onClick: () => onRuntimeMode("overview-server"), children: "Overview Server" }), _jsx(ModeButton, { active: runtimeMode === "standalone-server", onClick: () => onRuntimeMode("standalone-server"), children: "Standalone Server" })] }));
}
function ModeButton(props) {
    return (_jsx("button", { className: props.active
            ? "px-3 py-2 text-sm font-medium bg-zinc-950 text-white"
            : "px-3 py-2 text-sm font-medium text-zinc-700 transition hover:bg-zinc-100 disabled:cursor-not-allowed disabled:opacity-50", disabled: props.disabled, type: "button", onClick: props.onClick, children: props.children }));
}
function RuntimePanel({ config, health, serverState, surface, wasmState, }) {
    return (_jsxs("section", { className: "rounded-md border border-zinc-200 bg-white p-4", children: [_jsx("h2", { className: "text-sm font-semibold text-zinc-950", children: "Runtime" }), _jsxs("dl", { className: "mt-3 grid gap-3 text-sm", children: [_jsx(StatusRow, { label: "WASM", state: wasmState }), _jsx(StatusRow, { label: "Server", state: serverState }), _jsx(DetailRow, { label: "Server URL", value: configuredServerBaseUrl(config) }), _jsx(DetailRow, { label: "Health", value: health?.package ?? "Not loaded" }), _jsx(DetailRow, { label: "Library", value: surface?.library ?? config.library }), _jsx(DetailRow, { label: "Operations", value: String(surface?.operations.length ?? 0) }), health?.requiredFeature ? _jsx(DetailRow, { label: "Feature", value: health.requiredFeature }) : null] })] }));
}
function SupportPanel({ operations }) {
    return (_jsxs("section", { className: "rounded-md border border-zinc-200 bg-white p-4", children: [_jsx("h2", { className: "text-sm font-semibold text-zinc-950", children: "Support" }), _jsx("ul", { className: "mt-3 space-y-2 font-mono text-xs text-zinc-800", children: operations.map((candidate) => (_jsxs("li", { className: "rounded-md bg-zinc-50 p-2", children: [candidate.id, " \u00B7 WASM ", candidate.wasmSupported ? "yes" : "no", " \u00B7 server ", candidate.serverSupported ? "yes" : "no"] }, candidate.id))) })] }));
}
function StatusRow({ label, state }) {
    return _jsx(DetailRow, { label: label, value: state === "ready" ? "Ready" : state === "error" ? "Unavailable" : state === "disabled" ? "Disabled" : "Loading" });
}
function DetailRow({ label, value }) {
    return (_jsxs("div", { children: [_jsx("dt", { className: "text-xs font-semibold uppercase text-zinc-500", children: label }), _jsx("dd", { className: "mt-1 break-words font-mono text-zinc-900", children: value })] }));
}
function orderedOperations(operations, featured) {
    if (!featured?.length) {
        return operations;
    }
    const rank = new Map(featured.map((operation, index) => [operation, index]));
    return [...operations].sort((left, right) => (rank.get(left.id) ?? 999) - (rank.get(right.id) ?? 999));
}
function initializeSelection(surface, current, setSelectedOperation, setInput, defaultOperation) {
    const operation = surface.operations.find((candidate) => candidate.id === current) ??
        surface.operations.find((candidate) => candidate.id === defaultOperation) ??
        surface.operations[0];
    if (!operation) {
        return;
    }
    setSelectedOperation(operation.id);
    setInput(storedInput(surface.library, operation.id) ?? JSON.stringify(operation.exampleRequest ?? {}, null, 2));
}
function readRuntimeMode(config) {
    const runtime = readQuery("runtime");
    if (runtime === "client-wasm" || runtime === "overview-server" || runtime === "standalone-server") {
        if (runtime === "client-wasm" && !config.wasm) {
            return "overview-server";
        }
        return runtime;
    }
    return config.wasm ? "client-wasm" : "overview-server";
}
function readQuery(key) {
    if (typeof window === "undefined") {
        return null;
    }
    return new URLSearchParams(window.location.search).get(key);
}
function writeQuery(values) {
    if (typeof window === "undefined") {
        return;
    }
    const url = new URL(window.location.href);
    for (const [key, value] of Object.entries(values)) {
        url.searchParams.set(key, value);
    }
    window.history.replaceState({}, "", url);
}
function storageKey(library, operation) {
    return `package-workbench:${library}:${operation}`;
}
function storedInput(library, operation) {
    try {
        return localStorage.getItem(storageKey(library, operation));
    }
    catch {
        return null;
    }
}
function persistInput(library, operation, input) {
    try {
        localStorage.setItem(storageKey(library, operation), input);
    }
    catch {
        return;
    }
}
function patchValue(input, path, value) {
    const root = input && typeof input === "object" && !Array.isArray(input) ? { ...input } : {};
    let cursor = root;
    for (const segment of path.slice(0, -1)) {
        const next = cursor[segment];
        const object = next && typeof next === "object" && !Array.isArray(next) ? { ...next } : {};
        cursor[segment] = object;
        cursor = object;
    }
    const last = path[path.length - 1];
    if (last) {
        cursor[last] = value;
    }
    return root;
}
function defaultFileInputs(domain) {
    if (domain === "image") {
        return [{ id: "image", label: "Image input", accept: "image/*", targetPath: ["imageDataUrl"] }];
    }
    if (domain === "audio") {
        return [{ id: "audio", label: "Audio input", accept: "audio/*", targetPath: ["audioDataUrl"] }];
    }
    if (domain === "video") {
        return [{ id: "video", label: "Video input", accept: "video/*", targetPath: ["videoDataUrl"] }];
    }
    if (domain === "comfyui") {
        return [{ id: "workflow", label: "Workflow JSON", accept: "application/json,.json", targetPath: ["workflow"], encoding: "text" }];
    }
    return [];
}
//# sourceMappingURL=index.js.map