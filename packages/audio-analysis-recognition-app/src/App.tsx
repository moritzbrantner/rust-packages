import { FormEvent, useEffect, useMemo, useState } from "react";

import {
  fetchHealth,
  fetchPackageMetadata,
  listAudioModels,
  runAudioTask,
  serverBaseUrl,
  wrappedLibrary,
  type AudioFeatureSummary,
  type AudioModelMetadata,
  type AudioTask,
  type HealthPayload,
  type PackageMetadata,
} from "./api";

type LoadState = "idle" | "loading" | "ready" | "error";

const packageDescription =
  "Audio model catalog, fallback schemas, embeddings, ASR, diarization, separation, and generation contracts.";

const audioTasks: Array<{ id: AudioTask; label: string; taskKey: string }> = [
  { id: "classify", label: "Classify", taskKey: "audio_classification" },
  { id: "events", label: "Events", taskKey: "audio_event_detection" },
  { id: "embed", label: "Embed", taskKey: "audio_embedding" },
  { id: "transcribe", label: "Transcribe", taskKey: "speech_recognition" },
  { id: "diarize", label: "Diarize", taskKey: "speaker_diarization" },
  { id: "separate", label: "Separate", taskKey: "source_separation" },
  { id: "generate", label: "Generate", taskKey: "audio_generation" },
];

const fallbackModelIds: Record<AudioTask, string> = {
  classify: "ast-audioset",
  events: "audioset-event-detector",
  embed: "clap-htsat-unfused",
  transcribe: "whisper-tiny-en",
  diarize: "pyannote-speaker-diarization-3.1",
  separate: "demucs-music-separation",
  generate: "musicgen-small",
};

const sampleFeatures: AudioFeatureSummary = {
  durationSeconds: 12,
  sampleRate: 48000,
  rms: 0.14,
  peak: 0.42,
  zeroCrossingRate: 0.08,
  dominantFrequencyHz: 220,
  spectralCentroidHz: 1800,
};

export function App() {
  const [loadState, setLoadState] = useState<LoadState>("idle");
  const [health, setHealth] = useState<HealthPayload | null>(null);
  const [metadata, setMetadata] = useState<PackageMetadata | null>(null);
  const [models, setModels] = useState<AudioModelMetadata[]>([]);
  const [audioTask, setAudioTask] = useState<AudioTask>("classify");
  const [selectedModelIds, setSelectedModelIds] = useState<Partial<Record<AudioTask, string>>>({});
  const [features, setFeatures] = useState<AudioFeatureSummary>(sampleFeatures);
  const [labels, setLabels] = useState("speech,music,silence,ambient");
  const [frameRms, setFrameRms] = useState("0.04,0.08,0.34,0.41,0.09");
  const [durationSeconds, setDurationSeconds] = useState(12);
  const [stems, setStems] = useState("vocals,drums,bass,other");
  const [prompt, setPrompt] = useState("clean percussive loop with warm bass");
  const [result, setResult] = useState<unknown | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void refresh();
  }, []);

  const selectedTask = audioTasks.find((task) => task.id === audioTask) ?? audioTasks[0];
  const modelOptions = useMemo(
    () => models.filter((model) => model.task === selectedTask.taskKey),
    [models, selectedTask.taskKey],
  );
  const selectedModelId =
    selectedModelIds[audioTask] ?? modelOptions[0]?.id ?? fallbackModelIds[audioTask];
  const selectedModel = modelOptions.find((model) => model.id === selectedModelId);
  const json = result ? JSON.stringify(result, null, 2) : "";
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
      const [nextHealth, nextMetadata, nextModels] = await Promise.all([
        fetchHealth(),
        fetchPackageMetadata(),
        listAudioModels(),
      ]);
      setHealth(nextHealth);
      setMetadata(nextMetadata);
      setModels(nextModels);
      setLoadState("ready");
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Unable to reach the server");
      setLoadState("error");
    }
  }

  async function submit(event?: FormEvent<HTMLFormElement>) {
    event?.preventDefault();
    setError(null);
    try {
      const payload = await runAudioTask(audioTask, taskBody(audioTask, selectedModelId));
      setResult(payload);
      setLoadState("ready");
    } catch (caught) {
      setResult(null);
      setLoadState("error");
      setError(caught instanceof Error ? caught.message : "Operation failed");
    }
  }

  function taskBody(task: AudioTask, modelId: string) {
    const model = { modelId, fallbackPolicy: "heuristic_fallback" };
    switch (task) {
      case "classify":
        return { features, labels: splitCsv(labels), model };
      case "events":
        return {
          frames: splitCsv(frameRms).map((value, index) => ({
            startSeconds: index,
            endSeconds: index + 1,
            rms: Number(value),
            peak: Number(value),
          })),
          threshold: 0.2,
          model,
        };
      case "embed":
        return {
          features: [features],
          dimensions: 128,
          normalize: true,
          model: { modelId, fallbackPolicy: "fast_fallback" },
        };
      case "transcribe":
        return {
          model,
          importedSegments: [
            { index: 0, startSeconds: 0, endSeconds: 2.4, text: "Audio model schemas" },
            { index: 1, startSeconds: 2.4, endSeconds: 4.8, text: "stay portable across runtimes." },
          ],
        };
      case "diarize":
        return { durationSeconds, model };
      case "separate":
        return { stems: splitCsv(stems), model };
      case "generate":
        return { prompt, durationSeconds, model };
    }
  }

  return (
    <main className="min-h-screen bg-zinc-50 text-zinc-950">
      <section className="border-b border-zinc-200 bg-white">
        <div className="mx-auto flex max-w-7xl flex-col gap-4 px-5 py-5 lg:flex-row lg:items-center lg:justify-between">
          <div>
            <p className="text-xs font-semibold uppercase tracking-wide text-teal-700">
              Package app
            </p>
            <h1 className="mt-1 text-2xl font-semibold">Audio Analysis Recognition</h1>
            <p className="mt-2 max-w-3xl text-sm leading-6 text-zinc-600">{packageDescription}</p>
          </div>
          <div className="flex flex-wrap items-center gap-3">
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

      <section className="mx-auto grid max-w-screen-2xl gap-5 px-5 py-6 xl:grid-cols-[minmax(360px,0.85fr)_minmax(0,1.15fr)]">
        <form className="panel" onSubmit={submit}>
          <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
            <div>
              <h2 className="section-title">Audio task</h2>
              <p className="section-copy">Run model schemas with imported or deterministic fallback data.</p>
            </div>
            <button className="button-primary" type="submit">
              Run
            </button>
          </div>

          <div className="mt-5 flex flex-wrap gap-2">
            {audioTasks.map((task) => (
              <button
                key={task.id}
                className={`rounded-md border px-3 py-2 text-sm font-medium transition ${
                  audioTask === task.id
                    ? "border-zinc-950 bg-zinc-950 text-white"
                    : "border-zinc-200 bg-white text-zinc-700 hover:bg-zinc-100"
                }`}
                type="button"
                onClick={() => setAudioTask(task.id)}
              >
                {task.label}
              </button>
            ))}
          </div>

          <div className="mt-5 grid gap-4">
            <label className="grid gap-1 text-sm font-medium text-zinc-700">
              Model
              <select
                className="min-h-10 rounded-md border border-zinc-300 bg-white px-3 text-sm text-zinc-950 outline-none focus:border-teal-500 focus:ring-2 focus:ring-teal-200"
                value={selectedModelId}
                onChange={(event) =>
                  setSelectedModelIds((current) => ({ ...current, [audioTask]: event.target.value }))
                }
              >
                {modelOptions.length === 0 ? (
                  <option value={selectedModelId}>{selectedModelId}</option>
                ) : (
                  modelOptions.map((model) => (
                    <option key={model.id} value={model.id}>
                      {model.id}
                    </option>
                  ))
                )}
              </select>
            </label>

            {audioTask === "classify" ? (
              <label className="grid gap-1 text-sm font-medium text-zinc-700">
                Labels
                <input
                  className="text-input"
                  value={labels}
                  onChange={(event) => setLabels(event.target.value)}
                />
              </label>
            ) : null}

            {audioTask === "events" ? (
              <label className="grid gap-1 text-sm font-medium text-zinc-700">
                RMS frames
                <input
                  className="text-input"
                  value={frameRms}
                  onChange={(event) => setFrameRms(event.target.value)}
                />
              </label>
            ) : null}

            {audioTask === "separate" ? (
              <label className="grid gap-1 text-sm font-medium text-zinc-700">
                Stems
                <input
                  className="text-input"
                  value={stems}
                  onChange={(event) => setStems(event.target.value)}
                />
              </label>
            ) : null}

            {audioTask === "generate" ? (
              <label className="grid gap-1 text-sm font-medium text-zinc-700">
                Prompt
                <input
                  className="text-input"
                  value={prompt}
                  onChange={(event) => setPrompt(event.target.value)}
                />
              </label>
            ) : null}

            <div className="grid gap-3 sm:grid-cols-2">
              <NumberField label="RMS" value={features.rms ?? 0} onChange={(rms) => setFeatures({ ...features, rms })} />
              <NumberField label="Peak" value={features.peak ?? 0} onChange={(peak) => setFeatures({ ...features, peak })} />
              <NumberField
                label="Centroid Hz"
                value={features.spectralCentroidHz ?? 0}
                onChange={(spectralCentroidHz) => setFeatures({ ...features, spectralCentroidHz })}
              />
              <NumberField
                label="Duration"
                value={durationSeconds}
                onChange={(value) => {
                  setDurationSeconds(value);
                  setFeatures({ ...features, durationSeconds: value });
                }}
              />
            </div>
          </div>
        </form>

        <section className="grid gap-5">
          <section className="panel">
            <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
              <div>
                <h2 className="section-title">Selected model</h2>
                <p className="section-copy">
                  {selectedModel?.modelId ?? selectedModelId} · {selectedModel?.runtime ?? "runtime"}
                </p>
              </div>
              <span
                className={`status-pill ${selectedModel?.supported ? "status-online" : "status-pending"}`}
              >
                {selectedModel?.supported ? "Default" : "Gated"}
              </span>
            </div>
            {selectedModel?.note ? (
              <p className="mt-4 rounded-md border border-zinc-200 bg-zinc-50 p-3 text-sm leading-6 text-zinc-700">
                {selectedModel.note}
              </p>
            ) : null}
          </section>

          <section className="panel">
            <h2 className="section-title">Result</h2>
            {json ? (
              <pre className="result-block">{json}</pre>
            ) : (
              <p className="mt-4 rounded-md border border-dashed border-zinc-300 bg-zinc-50 p-4 text-sm text-zinc-600">
                Run a task to inspect the model payload.
              </p>
            )}
            {error ? <p className="error-text">{error}</p> : null}
          </section>

          <section className="panel">
            <h2 className="section-title">Package</h2>
            <dl className="detail-list md:grid md:grid-cols-2 md:gap-3 md:space-y-0">
              <div>
                <dt>Server</dt>
                <dd>{serverBaseUrl}</dd>
              </div>
              <div>
                <dt>Library</dt>
                <dd>{metadata?.library ?? wrappedLibrary}</dd>
              </div>
              <div>
                <dt>Models</dt>
                <dd>{models.length}</dd>
              </div>
              <div>
                <dt>Health</dt>
                <dd>{health?.package ?? "Not loaded"}</dd>
              </div>
            </dl>
          </section>
        </section>
      </section>
    </main>
  );
}

function NumberField({
  label,
  onChange,
  value,
}: {
  label: string;
  value: number;
  onChange: (value: number) => void;
}) {
  return (
    <label className="grid gap-1 text-sm font-medium text-zinc-700">
      {label}
      <input
        className="text-input"
        type="number"
        step="0.01"
        value={value}
        onChange={(event) => onChange(Number(event.target.value))}
      />
    </label>
  );
}

function splitCsv(value: string): string[] {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

