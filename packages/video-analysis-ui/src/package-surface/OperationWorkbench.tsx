import { useEffect, useState, type FormEvent } from "react";
import {
  Button,
  Field,
  FieldContent,
  FieldDescription,
  FieldGroup,
  FieldLabel,
  Input,
  Textarea,
} from "../shared/primitives";

import { landscapeContractForOperation } from "./runtime";
import type { PackageAppPreset, SurfaceOperation } from "./types";

export interface OperationWorkbenchGroup {
  id: string;
  label: string;
  description?: string;
  operations: SurfaceOperation[];
}

export function OperationWorkbench({
  canRun,
  error,
  input,
  operation,
  operationGroups,
  operations,
  presets = [],
  running,
  runDisabledReason,
  selectedOperation,
  onInputChange,
  onPreset,
  onRun,
  onSelectOperation,
}: {
  canRun: boolean;
  error: string | null;
  input: string;
  operation: SurfaceOperation | null;
  operationGroups?: OperationWorkbenchGroup[];
  operations: SurfaceOperation[];
  presets?: PackageAppPreset[];
  running: boolean;
  runDisabledReason?: string;
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

  const activeGroup = operationGroups?.find((group) => group.operations.some((candidate) => candidate.id === selectedOperation));
  const visibleOperations = activeGroup?.operations ?? operations;

  return (
    <form className="rounded-md border border-zinc-200 bg-white p-4" onSubmit={submit}>
      {operationGroups && operationGroups.length > 1 ? (
        <div className="mb-4">
          <div className="inline-flex flex-wrap gap-1 rounded-md bg-zinc-100 p-1" role="tablist" aria-label="Operation category">
            {operationGroups.map((group) => {
              const active = group.id === activeGroup?.id;
              return (
                <button
                  key={group.id}
                  aria-selected={active}
                  className={
                    active
                      ? "rounded-md bg-zinc-950 px-3 py-2 text-sm font-semibold text-white"
                      : "rounded-md px-3 py-2 text-sm font-semibold text-zinc-700 hover:bg-white"
                  }
                  role="tab"
                  type="button"
                  onClick={() => {
                    const nextOperation = group.operations[0];
                    if (nextOperation && !group.operations.some((candidate) => candidate.id === selectedOperation)) {
                      onSelectOperation(nextOperation.id);
                    }
                  }}
                >
                  {group.label}
                </button>
              );
            })}
          </div>
          {activeGroup?.description ? <p className="mt-2 text-sm leading-6 text-zinc-600">{activeGroup.description}</p> : null}
        </div>
      ) : null}
      <div className="grid gap-3 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-end">
        <label className="grid gap-1 text-sm">
          <span className="text-xs font-semibold uppercase text-zinc-500">Operation</span>
          <select
            className="min-h-10 w-full rounded-md border border-zinc-300 bg-white px-3 text-sm text-zinc-950 outline-none focus:border-zinc-500 focus:ring-2 focus:ring-zinc-200"
            value={selectedOperation}
            onChange={(event) => onSelectOperation(event.target.value)}
          >
            {visibleOperations.map((candidate) => (
              <option key={candidate.id} value={candidate.id}>
                {candidate.name}
              </option>
            ))}
          </select>
        </label>
        <Button
          className="min-h-10 rounded-md bg-zinc-950 px-4 text-sm font-semibold text-white disabled:cursor-not-allowed disabled:opacity-50"
          disabled={!canRun || running || !selectedOperation}
          type="submit"
        >
          {running ? "Running" : "Run"}
        </Button>
      </div>
      <p className="mt-3 text-sm leading-6 text-zinc-600">{operation?.description ?? "Run a package operation."}</p>
      <LandscapeContractBadges operation={operation} />
      {presets.length > 0 ? (
        <div className="mt-3 flex flex-wrap gap-2">
          {presets.map((preset) => (
            <Button
              key={preset.id}
              className="rounded-md border border-zinc-300 bg-white px-3 py-2 text-sm font-semibold text-zinc-700 hover:bg-zinc-50"
              type="button"
              title={preset.description}
              onClick={() => onPreset(preset)}
            >
              {preset.label}
            </Button>
          ))}
        </div>
      ) : null}
      <OperationInputForm input={input} operation={operation} onInputChange={onInputChange} />
      {runDisabledReason ? (
        <p className="mt-4 rounded-md border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-900">
          {runDisabledReason}
        </p>
      ) : null}
      {error ? <p className="mt-4 rounded-md border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-800">{error}</p> : null}
    </form>
  );
}

function LandscapeContractBadges({ operation }: { operation: SurfaceOperation | null }) {
  const landscape = landscapeContractForOperation(operation);
  if (!landscape) {
    return null;
  }
  const inputs = landscape.function.inputs;
  const outputs = landscape.function.outputs;
  if (inputs.length === 0 && outputs.length === 0) {
    return null;
  }
  return (
    <div className="mt-3 rounded-md border border-teal-200 bg-teal-50 p-3">
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-xs font-semibold uppercase text-teal-800">Curated I/O</span>
        <span className="rounded-sm bg-white px-2 py-1 text-xs font-medium text-teal-900">{landscape.function.id}</span>
        <span className="rounded-sm bg-teal-100 px-2 py-1 text-xs font-medium text-teal-900">
          {landscape.function.stability}
        </span>
      </div>
      <div className="mt-2 flex flex-wrap gap-2">
        {inputs.map((port) => (
          <span
            key={`input-${port.name}-${port.typeRef.id}`}
            className="rounded-sm border border-teal-200 bg-white px-2 py-1 text-xs text-teal-900"
          >
            {port.name}: {port.typeRef.id}
          </span>
        ))}
        {outputs.map((port) => (
          <span
            key={`output-${port.name}-${port.typeRef.id}`}
            className="rounded-sm border border-teal-200 bg-white px-2 py-1 text-xs text-teal-900"
          >
            {port.name}: {port.typeRef.id}
          </span>
        ))}
      </div>
    </div>
  );
}

function OperationInputForm({
  input,
  operation,
  onInputChange,
}: {
  input: string;
  operation: SurfaceOperation | null;
  onInputChange: (input: string) => void;
}) {
  const parsed = parseObjectInput(input);

  if (!parsed.ok) {
    return (
      <FieldGroup className="mt-4 gap-3">
        <Field>
          <FieldContent>
            <FieldLabel htmlFor="operation-input-request-json">Request JSON</FieldLabel>
            <FieldDescription>
              The saved request could not be parsed. Fix the JSON to return to the form view.
            </FieldDescription>
          </FieldContent>
          <Textarea
            id="operation-input-request-json"
            className="min-h-80 w-full resize-y rounded-md border border-zinc-300 bg-zinc-950 p-4 font-mono text-sm leading-6 text-zinc-50 outline-none focus:border-zinc-500 focus:ring-2 focus:ring-zinc-200"
            spellCheck={false}
            value={input}
            onChange={(event) => onInputChange(event.target.value)}
          />
        </Field>
      </FieldGroup>
    );
  }

  const entries = Object.entries(parsed.value);
  const updateField = (key: string, value: unknown) => {
    onInputChange(JSON.stringify({ ...parsed.value, [key]: value }, null, 2));
  };

  if (entries.length === 0) {
    return (
      <div className="mt-4 rounded-md border border-dashed border-zinc-300 bg-zinc-50 px-4 py-5 text-sm text-zinc-600">
        This operation does not require request input.
      </div>
    );
  }

  return (
    <FieldGroup className="mt-4 gap-4 rounded-md border border-zinc-200 bg-zinc-50 p-4">
      <div>
        <h3 className="text-sm font-semibold text-zinc-950">Request input</h3>
        <p className="mt-1 text-sm leading-6 text-zinc-600">
          {operation ? `Edit inputs for ${operation.name}.` : "Edit operation inputs."}
        </p>
      </div>
      {entries.map(([key, value]) => (
        <InputField key={key} name={key} value={value} onChange={(nextValue) => updateField(key, nextValue)} />
      ))}
    </FieldGroup>
  );
}

function InputField({
  name,
  value,
  onChange,
}: {
  name: string;
  value: unknown;
  onChange: (value: unknown) => void;
}) {
  const label = humanizeKey(name);
  const fieldId = inputFieldId(name);

  if (typeof value === "boolean") {
    return (
      <div className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-3 rounded-md border border-zinc-200 bg-white p-3">
        <div className="min-w-0">
          <label className="text-sm font-medium leading-snug text-zinc-950" htmlFor={fieldId}>
            {label}
          </label>
          <p className="mt-1 break-words text-sm leading-normal text-zinc-500">{name}</p>
        </div>
        <button
          id={fieldId}
          aria-checked={value}
          aria-label={label}
          className={
            value
              ? "relative inline-flex h-6 w-11 shrink-0 items-center rounded-md border border-zinc-950 bg-zinc-950 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400"
              : "relative inline-flex h-6 w-11 shrink-0 items-center rounded-md border border-zinc-300 bg-zinc-100 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400"
          }
          role="switch"
          type="button"
          onClick={() => onChange(!value)}
        >
          <span
            className={
              value
                ? "block h-5 w-5 translate-x-5 rounded-sm bg-white transition-transform"
                : "block h-5 w-5 translate-x-0 rounded-sm bg-white shadow-sm transition-transform"
            }
          />
        </button>
      </div>
    );
  }

  if (typeof value === "number") {
    return (
      <Field>
        <FieldContent>
          <FieldLabel htmlFor={fieldId}>{label}</FieldLabel>
          <FieldDescription>{name}</FieldDescription>
        </FieldContent>
        <Input
          id={fieldId}
          type="number"
          value={Number.isFinite(value) ? String(value) : ""}
          onChange={(event) => onChange(event.target.value === "" ? 0 : Number(event.target.value))}
        />
      </Field>
    );
  }

  if (typeof value === "string") {
    const multiline = isLongTextField(name, value);
    return (
      <Field>
        <FieldContent>
          <FieldLabel htmlFor={fieldId}>{label}</FieldLabel>
          <FieldDescription>{name}</FieldDescription>
        </FieldContent>
        {multiline ? (
          <Textarea
            id={fieldId}
            className="min-h-32 resize-y"
            spellCheck={false}
            value={value}
            onChange={(event) => onChange(event.target.value)}
          />
        ) : (
          <Input id={fieldId} value={value} onChange={(event) => onChange(event.target.value)} />
        )}
      </Field>
    );
  }

  if (Array.isArray(value) && value.every((item) => ["string", "number", "boolean"].includes(typeof item))) {
    return (
      <Field>
        <FieldContent>
          <FieldLabel htmlFor={fieldId}>{label}</FieldLabel>
          <FieldDescription>One {name} value per line</FieldDescription>
        </FieldContent>
        <Textarea
          id={fieldId}
          className="min-h-28 resize-y"
          spellCheck={false}
          value={value.map(String).join("\n")}
          onChange={(event) => {
            const lines = event.target.value.split("\n").filter((line) => line.length > 0);
            onChange(value.every((item) => typeof item === "number") ? lines.map(Number) : lines);
          }}
        />
      </Field>
    );
  }

  return <StructuredValueField name={name} value={value} onChange={onChange} />;
}

function StructuredValueField({
  name,
  value,
  onChange,
}: {
  name: string;
  value: unknown;
  onChange: (value: unknown) => void;
}) {
  const externalJson = JSON.stringify(value, null, 2);
  const fieldId = inputFieldId(name);
  const [draft, setDraft] = useState(() => externalJson);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setDraft(externalJson);
    setError(null);
  }, [externalJson]);

  return (
    <Field>
      <FieldContent>
        <FieldLabel htmlFor={fieldId}>{humanizeKey(name)}</FieldLabel>
        <FieldDescription>{name}</FieldDescription>
      </FieldContent>
      <Textarea
        id={fieldId}
        className="min-h-36 resize-y font-mono text-sm"
        spellCheck={false}
        value={draft}
        onChange={(event) => {
          const nextDraft = event.target.value;
          setDraft(nextDraft);
          try {
            onChange(JSON.parse(nextDraft) as unknown);
            setError(null);
          } catch (caught) {
            setError(caught instanceof Error ? caught.message : "Invalid value");
          }
        }}
      />
      {error ? <p className="text-sm text-rose-700">{error}</p> : null}
    </Field>
  );
}

function parseObjectInput(input: string): { ok: true; value: Record<string, unknown> } | { ok: false } {
  try {
    const parsed = JSON.parse(input || "{}") as unknown;
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      return { ok: true, value: parsed as Record<string, unknown> };
    }
    return { ok: true, value: {} };
  } catch {
    return { ok: false };
  }
}

function isLongTextField(name: string, value: string): boolean {
  return name.toLowerCase().includes("text") || value.length > 72 || value.includes("\n");
}

function humanizeKey(key: string): string {
  return key
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/[-_.]/g, " ")
    .replace(/\b\w/g, (character) => character.toUpperCase());
}

function inputFieldId(name: string): string {
  return `operation-input-${name.replace(/[^a-zA-Z0-9_-]/g, "-")}`;
}
