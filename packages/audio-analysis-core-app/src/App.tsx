import { FormEvent, useEffect, useMemo, useState } from "react";

import initAudioWasm, {
  analyzeAudioSamples,
  mixToMono,
  planAudioFrames,
  type AudioSampleAnalysis,
} from "../../audio-analysis-core-wasm";

type LoadState = "loading" | "ready" | "error";

const packageDescription = "Shared audio frame conversion, windowing, and streaming helpers for video-analysis.";

export function App() {
  const [loadState, setLoadState] = useState<LoadState>("loading");
  const [error, setError] = useState<string | null>(null);
  const [sampleRate, setSampleRate] = useState(48_000);
  const [frequency, setFrequency] = useState(440);
  const [durationMs, setDurationMs] = useState(120);
  const [customSamples, setCustomSamples] = useState("1, -1, 0.5, 0.25");
  const [analysis, setAnalysis] = useState<AudioSampleAnalysis | null>(null);
  const [monoPreview, setMonoPreview] = useState<number[]>([]);

  useEffect(() => {
    initAudioWasm()
      .then(() => {
        setLoadState("ready");
        setError(null);
      })
      .catch((caught) => {
        setLoadState("error");
        setError(caught instanceof Error ? caught.message : String(caught));
      });
  }, []);

  const generatedSamples = useMemo(
    () => synthesizeSine(frequency, sampleRate, durationMs),
    [durationMs, frequency, sampleRate],
  );
  const framePlan = useMemo(
    () => (loadState === "ready" ? planAudioFrames(generatedSamples.length, 1024, 512) : null),
    [generatedSamples.length, loadState],
  );

  function analyzeGenerated() {
    setError(null);
    try {
      setAnalysis(
        analyzeAudioSamples(generatedSamples, {
          sampleRate,
          fftSize: 2048,
          frameSize: 1024,
          hopSize: 512,
        }),
      );
      setMonoPreview(Array.from(generatedSamples.slice(0, 16)));
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  }

  function analyzeCustom(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    try {
      const samples = parseSamples(customSamples);
      setAnalysis(
        analyzeAudioSamples(samples, {
          sampleRate,
          channels: 2,
          channelMix: "average",
          fftSize: 2048,
        }),
      );
      setMonoPreview(mixToMono(samples, 2, "average").slice(0, 16));
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  }

  return (
    <main className="min-h-screen bg-zinc-50 text-zinc-950">
      <section className="border-b border-zinc-200 bg-white">
        <div className="mx-auto flex max-w-6xl flex-col gap-4 px-5 py-5 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <p className="text-xs font-semibold uppercase tracking-wide text-teal-700">WASM package app</p>
            <h1 className="mt-1 text-2xl font-semibold">Audio Analysis Core</h1>
            <p className="mt-2 max-w-3xl text-sm leading-6 text-zinc-600">{packageDescription}</p>
          </div>
          <span
            className={`status-pill ${loadState === "ready" ? "status-online" : loadState === "error" ? "status-offline" : "status-pending"}`}
          >
            {loadState === "ready" ? "Ready" : loadState === "error" ? "Error" : "Loading"}
          </span>
        </div>
      </section>

      <section className="mx-auto grid max-w-6xl gap-5 px-5 py-6 lg:grid-cols-[minmax(0,1fr)_360px]">
        <section className="panel">
          <div className="grid gap-3 sm:grid-cols-3">
            <label className="grid gap-1 text-sm">
              <span className="text-xs font-semibold uppercase tracking-wide text-zinc-500">Sample rate</span>
              <input
                className="rounded-md border border-zinc-300 px-3 py-2"
                type="number"
                min={1}
                value={sampleRate}
                onChange={(event) => setSampleRate(Number(event.target.value))}
              />
            </label>
            <label className="grid gap-1 text-sm">
              <span className="text-xs font-semibold uppercase tracking-wide text-zinc-500">Tone Hz</span>
              <input
                className="rounded-md border border-zinc-300 px-3 py-2"
                type="number"
                min={1}
                value={frequency}
                onChange={(event) => setFrequency(Number(event.target.value))}
              />
            </label>
            <label className="grid gap-1 text-sm">
              <span className="text-xs font-semibold uppercase tracking-wide text-zinc-500">Duration ms</span>
              <input
                className="rounded-md border border-zinc-300 px-3 py-2"
                type="number"
                min={1}
                value={durationMs}
                onChange={(event) => setDurationMs(Number(event.target.value))}
              />
            </label>
          </div>
          <div className="mt-4 flex flex-wrap items-center gap-3">
            <button
              className="button-primary"
              type="button"
              disabled={loadState !== "ready"}
              onClick={analyzeGenerated}
            >
              Analyze Tone
            </button>
            <span className="text-sm text-zinc-600">{generatedSamples.length} generated samples</span>
          </div>

          <form className="mt-5" onSubmit={analyzeCustom}>
            <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
              <div>
                <h2 className="section-title">Stereo samples</h2>
                <p className="section-copy">Comma-separated interleaved samples, mixed to mono in WASM.</p>
              </div>
              <button className="button-secondary" type="submit" disabled={loadState !== "ready"}>
                Analyze Samples
              </button>
            </div>
            <textarea
              className="code-input mt-4 min-h-32"
              spellCheck={false}
              value={customSamples}
              onChange={(event) => setCustomSamples(event.target.value)}
            />
          </form>

          {analysis ? <pre className="result-block">{JSON.stringify(analysis, null, 2)}</pre> : null}
          {error ? <p className="error-text">{error}</p> : null}
        </section>

        <aside className="space-y-5">
          <section className="panel">
            <h2 className="section-title">Metrics</h2>
            <dl className="detail-list">
              <div>
                <dt>RMS</dt>
                <dd>{analysis ? analysis.rms.toFixed(4) : "Not run"}</dd>
              </div>
              <div>
                <dt>Peak</dt>
                <dd>{analysis ? analysis.peak.toFixed(4) : "Not run"}</dd>
              </div>
              <div>
                <dt>Dominant Hz</dt>
                <dd>{analysis?.dominantFrequencyHz?.toFixed(2) ?? "Not run"}</dd>
              </div>
              <div>
                <dt>Pitch</dt>
                <dd>{analysis?.pitch.noteName ?? "Not run"}</dd>
              </div>
            </dl>
          </section>

          <section className="panel">
            <h2 className="section-title">Frame plan</h2>
            <dl className="detail-list">
              <div>
                <dt>Frames</dt>
                <dd>{framePlan?.frameCount ?? "Loading"}</dd>
              </div>
              <div>
                <dt>Starts</dt>
                <dd>{framePlan ? framePlan.starts.slice(0, 8).join(", ") : "Loading"}</dd>
              </div>
            </dl>
          </section>

          <section className="panel">
            <h2 className="section-title">Mono preview</h2>
            <ul className="endpoint-list">
              {monoPreview.length > 0 ? monoPreview.map((sample, index) => <li key={index}>{sample.toFixed(4)}</li>) : <li>Not run</li>}
            </ul>
          </section>
        </aside>
      </section>
    </main>
  );
}

function synthesizeSine(frequency: number, sampleRate: number, durationMs: number): Float32Array {
  const samples = Math.max(1, Math.round((sampleRate * durationMs) / 1000));
  return Float32Array.from({ length: samples }, (_, index) =>
    Math.sin((index * frequency * Math.PI * 2) / sampleRate),
  );
}

function parseSamples(input: string): number[] {
  const samples = input
    .split(/[\s,]+/)
    .map((value) => value.trim())
    .filter(Boolean)
    .map(Number);
  if (samples.length === 0 || samples.some((sample) => !Number.isFinite(sample))) {
    throw new Error("Samples must be finite numbers");
  }
  return samples;
}
