import { FormEvent, useEffect, useMemo, useState } from "react";

import initVideoWasm, {
  analyzeVideoFrame,
  frameTimecode,
  parseFrameTimecode,
  scenesFromCutFrames,
  type SceneInterval,
  type VideoFrameAnalysis,
} from "../../video-analysis-core-wasm";

type LoadState = "loading" | "ready" | "error";

const packageDescription = "Core media, timing, detection, and analyzer contracts for video-analysis.";

export function App() {
  const [loadState, setLoadState] = useState<LoadState>("loading");
  const [error, setError] = useState<string | null>(null);
  const [width, setWidth] = useState(2);
  const [height, setHeight] = useState(2);
  const [frameIndex, setFrameIndex] = useState(12);
  const [fpsNumerator, setFpsNumerator] = useState(24);
  const [fpsDenominator, setFpsDenominator] = useState(1);
  const [pixelData, setPixelData] = useState("255,0,0, 0,255,0, 0,0,255, 255,255,255");
  const [timecodeInput, setTimecodeInput] = useState("00:00:02.00");
  const [cutFrames, setCutFrames] = useState("24, 48");
  const [analysis, setAnalysis] = useState<VideoFrameAnalysis | null>(null);
  const [scenes, setScenes] = useState<SceneInterval[]>([]);

  useEffect(() => {
    initVideoWasm()
      .then(() => {
        setLoadState("ready");
        setError(null);
      })
      .catch((caught) => {
        setLoadState("error");
        setError(caught instanceof Error ? caught.message : String(caught));
      });
  }, []);

  const currentTimecode = useMemo(
    () =>
      loadState === "ready"
        ? frameTimecode(frameIndex, fpsNumerator, fpsDenominator, 3)
        : null,
    [fpsDenominator, fpsNumerator, frameIndex, loadState],
  );

  function submitFrame(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    try {
      setAnalysis(
        analyzeVideoFrame(
          Uint8Array.from(parseByteValues(pixelData)),
          width,
          height,
          "rgb24",
          frameIndex,
          fpsNumerator,
          fpsDenominator,
          3,
        ),
      );
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  }

  function submitTimeline(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    try {
      const parsed = parseFrameTimecode(timecodeInput, fpsNumerator, fpsDenominator, 3);
      setFrameIndex(parsed.frameIndex);
      setScenes(
        scenesFromCutFrames(
          parseNumberValues(cutFrames),
          Math.max(parsed.frameIndex + 48, 1),
          fpsNumerator,
          fpsDenominator,
        ),
      );
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
            <h1 className="mt-1 text-2xl font-semibold">Video Analysis Core</h1>
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
        <section className="space-y-5">
          <form className="panel" onSubmit={submitFrame}>
            <div className="grid gap-3 sm:grid-cols-5">
              <NumberField label="Width" value={width} min={1} onChange={setWidth} />
              <NumberField label="Height" value={height} min={1} onChange={setHeight} />
              <NumberField label="Frame" value={frameIndex} min={0} onChange={setFrameIndex} />
              <NumberField label="FPS num" value={fpsNumerator} min={1} onChange={setFpsNumerator} />
              <NumberField label="FPS den" value={fpsDenominator} min={1} onChange={setFpsDenominator} />
            </div>
            <textarea
              className="code-input mt-4 min-h-32"
              spellCheck={false}
              value={pixelData}
              onChange={(event) => setPixelData(event.target.value)}
            />
            <div className="mt-4 flex flex-wrap items-center gap-3">
              <button className="button-primary" type="submit" disabled={loadState !== "ready"}>
                Analyze Frame
              </button>
              <span className="text-sm text-zinc-600">{currentTimecode?.timecode ?? "Loading"}</span>
            </div>
          </form>

          <form className="panel" onSubmit={submitTimeline}>
            <div className="grid gap-3 sm:grid-cols-2">
              <label className="grid gap-1 text-sm">
                <span className="text-xs font-semibold uppercase tracking-wide text-zinc-500">Timecode</span>
                <input
                  className="rounded-md border border-zinc-300 px-3 py-2"
                  value={timecodeInput}
                  onChange={(event) => setTimecodeInput(event.target.value)}
                />
              </label>
              <label className="grid gap-1 text-sm">
                <span className="text-xs font-semibold uppercase tracking-wide text-zinc-500">Cut frames</span>
                <input
                  className="rounded-md border border-zinc-300 px-3 py-2"
                  value={cutFrames}
                  onChange={(event) => setCutFrames(event.target.value)}
                />
              </label>
            </div>
            <button className="button-secondary mt-4" type="submit" disabled={loadState !== "ready"}>
              Build Scenes
            </button>
          </form>

          {analysis ? <pre className="result-block">{JSON.stringify(analysis, null, 2)}</pre> : null}
          {error ? <p className="error-text">{error}</p> : null}
        </section>

        <aside className="space-y-5">
          <section className="panel">
            <h2 className="section-title">Pixels</h2>
            <div
              className="mt-4 grid overflow-hidden rounded-md border border-zinc-200"
              style={{ gridTemplateColumns: `repeat(${width}, minmax(0, 1fr))` }}
            >
              {previewPixels(pixelData, width * height).map((pixel, index) => (
                <div
                  key={index}
                  className="aspect-square border border-white"
                  style={{ backgroundColor: `rgb(${pixel[0]}, ${pixel[1]}, ${pixel[2]})` }}
                />
              ))}
            </div>
          </section>

          <section className="panel">
            <h2 className="section-title">Frame summary</h2>
            <dl className="detail-list">
              <div>
                <dt>Timecode</dt>
                <dd>{analysis?.timecode ?? currentTimecode?.timecode ?? "Not run"}</dd>
              </div>
              <div>
                <dt>Mean RGB</dt>
                <dd>
                  {analysis
                    ? `${analysis.meanRgb.r.toFixed(1)}, ${analysis.meanRgb.g.toFixed(1)}, ${analysis.meanRgb.b.toFixed(1)}`
                    : "Not run"}
                </dd>
              </div>
              <div>
                <dt>Center</dt>
                <dd>
                  {analysis
                    ? `${analysis.center.r}, ${analysis.center.g}, ${analysis.center.b}`
                    : "Not run"}
                </dd>
              </div>
            </dl>
          </section>

          <section className="panel">
            <h2 className="section-title">Scenes</h2>
            <ul className="endpoint-list">
              {scenes.length > 0 ? (
                scenes.map((scene) => (
                  <li key={`${scene.startFrame}-${scene.endFrame}`}>
                    {scene.startFrame}-{scene.endFrame}
                  </li>
                ))
              ) : (
                <li>Not run</li>
              )}
            </ul>
          </section>
        </aside>
      </section>
    </main>
  );
}

function NumberField({
  label,
  min,
  value,
  onChange,
}: {
  label: string;
  min: number;
  value: number;
  onChange: (value: number) => void;
}) {
  return (
    <label className="grid gap-1 text-sm">
      <span className="text-xs font-semibold uppercase tracking-wide text-zinc-500">{label}</span>
      <input
        className="rounded-md border border-zinc-300 px-3 py-2"
        type="number"
        min={min}
        value={value}
        onChange={(event) => onChange(Number(event.target.value))}
      />
    </label>
  );
}

function parseByteValues(input: string): number[] {
  return parseNumberValues(input).map((value) => Math.max(0, Math.min(255, value)));
}

function parseNumberValues(input: string): number[] {
  const values = input
    .split(/[\s,]+/)
    .map((value) => value.trim())
    .filter(Boolean)
    .map(Number);
  if (values.length === 0 || values.some((value) => !Number.isFinite(value))) {
    throw new Error("Values must be finite numbers");
  }
  return values.map((value) => Math.max(0, Math.round(value)));
}

function previewPixels(input: string, pixelCount: number): Array<[number, number, number]> {
  try {
    const values = parseByteValues(input);
    return Array.from({ length: pixelCount }, (_, index) => [
      values[index * 3] ?? 0,
      values[index * 3 + 1] ?? 0,
      values[index * 3 + 2] ?? 0,
    ]);
  } catch {
    return Array.from({ length: pixelCount }, () => [0, 0, 0]);
  }
}
