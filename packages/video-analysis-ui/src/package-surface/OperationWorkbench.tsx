import type { FormEvent } from "react";

import type { PackageAppPreset, SurfaceOperation } from "./types";

export function OperationWorkbench({
  error,
  input,
  operation,
  operations,
  presets = [],
  running,
  selectedOperation,
  onInputChange,
  onPreset,
  onRun,
  onSelectOperation,
}: {
  error: string | null;
  input: string;
  operation: SurfaceOperation | null;
  operations: SurfaceOperation[];
  presets?: PackageAppPreset[];
  running: boolean;
  selectedOperation: string;
  onInputChange: (input: string) => void;
  onPreset: (preset: PackageAppPreset) => void;
  onRun: () => void;
  onSelectOperation: (operation: string) => void;
}) {
  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    onRun();
  }

  return (
    <form className="rounded-md border border-zinc-200 bg-white p-4" onSubmit={submit}>
      <div className="grid gap-3 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-end">
        <label className="grid gap-1 text-sm">
          <span className="text-xs font-semibold uppercase text-zinc-500">Operation</span>
          <select
            className="min-h-10 rounded-md border border-zinc-300 bg-white px-3"
            value={selectedOperation}
            onChange={(event) => onSelectOperation(event.target.value)}
          >
            {operations.map((candidate) => (
              <option key={candidate.id} value={candidate.id}>
                {candidate.name}
              </option>
            ))}
          </select>
        </label>
        <button
          className="min-h-10 rounded-md bg-zinc-950 px-4 text-sm font-semibold text-white disabled:cursor-not-allowed disabled:opacity-50"
          disabled={running || !selectedOperation}
          type="submit"
        >
          {running ? "Running" : "Run"}
        </button>
      </div>
      <p className="mt-3 text-sm leading-6 text-zinc-600">{operation?.description ?? "Run a package operation."}</p>
      {presets.length > 0 ? (
        <div className="mt-3 flex flex-wrap gap-2">
          {presets.map((preset) => (
            <button
              key={preset.id}
              className="rounded-md border border-zinc-300 bg-white px-3 py-2 text-sm font-semibold text-zinc-700 hover:bg-zinc-50"
              type="button"
              title={preset.description}
              onClick={() => onPreset(preset)}
            >
              {preset.label}
            </button>
          ))}
        </div>
      ) : null}
      <textarea
        className="mt-4 min-h-80 w-full resize-y rounded-md border border-zinc-300 bg-zinc-950 p-4 font-mono text-sm leading-6 text-zinc-50 outline-none focus:border-zinc-500 focus:ring-2 focus:ring-zinc-200"
        spellCheck={false}
        value={input}
        onChange={(event) => onInputChange(event.target.value)}
      />
      {error ? <p className="mt-4 rounded-md border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-800">{error}</p> : null}
    </form>
  );
}

