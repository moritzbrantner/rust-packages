import React, { useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import {
  AssetSummary,
  CapabilityPanel,
  CliRunPanel,
  DataBucketOverview,
  EventList,
  JsonReportLoader,
  ModelObservationGrid,
  ObservationList,
  ScenePanel,
  SourceSummary,
  SplitPlanTable,
  TranscriptPanel,
  VideoSummaryCards,
  type CliRun,
  type YoutubeVideoReport,
} from "@video-analysis/ui";

import "./index.css";
import { sampleReport } from "./sampleReport";

type UseCaseId = "youtube-video";
type SourceMode = "url" | "file";
type ViewMode = "overview" | "scenes" | "signals" | "data";

interface UseCaseForm {
  sourceMode: SourceMode;
  url: string;
  input: string;
  output: string;
  workDir: string;
  sceneThreshold: number;
  minSceneLen: number;
  maxFrames: string;
  visualSampleEvery: number;
  skipTranscription: boolean;
  objectCommand: string;
  ocrCommand: string;
  textCommand: string;
}

const useCases: Array<{
  id: UseCaseId;
  name: string;
  packageName: string;
  status: string;
}> = [
  {
    id: "youtube-video",
    name: "YouTube Video",
    packageName: "video-analysis-use-cases",
    status: "Ready",
  },
];

const initialForm: UseCaseForm = {
  sourceMode: "url",
  url: "https://www.youtube.com/watch?v=...",
  input: "./video.mp4",
  output: "use-case-output/youtube-video/analysis.json",
  workDir: "use-case-output/youtube-video",
  sceneThreshold: 27,
  minSceneLen: 15,
  maxFrames: "",
  visualSampleEvery: 30,
  skipTranscription: false,
  objectCommand: "",
  ocrCommand: "",
  textCommand: "",
};

function App() {
  const [selectedUseCase, setSelectedUseCase] = useState<UseCaseId>("youtube-video");
  const [form, setForm] = useState<UseCaseForm>(initialForm);
  const [report, setReport] = useState<YoutubeVideoReport>(sampleReport);
  const [viewMode, setViewMode] = useState<ViewMode>("overview");
  const [runStatus, setRunStatus] = useState<CliRun["status"]>("pending");

  const command = useMemo(() => buildCommand(form), [form]);
  const cliRun: CliRun = {
    command: "cargo",
    args: command.slice(1),
    status: runStatus,
    exit_code: runStatus === "succeeded" ? 0 : null,
    output_files: [form.output],
    message: selectedUseCase === "youtube-video" ? "video-analysis-use-cases youtube-video" : null,
  };

  return (
    <main className="min-h-screen bg-zinc-50 text-zinc-950">
      <div className="grid min-h-screen lg:grid-cols-[280px_1fr]">
        <aside className="border-b border-zinc-200 bg-white lg:border-b-0 lg:border-r">
          <div className="sticky top-0 flex flex-col gap-5 p-4">
            <div>
              <div className="text-lg font-semibold tracking-normal">Video Analysis Studio</div>
              <div className="mt-1 text-sm text-zinc-500">Use cases and reports</div>
            </div>
            <nav className="space-y-2">
              {useCases.map((useCase) => (
                <button
                  key={useCase.id}
                  className={classNames(
                    "flex w-full items-center justify-between rounded-lg border px-3 py-3 text-left transition",
                    selectedUseCase === useCase.id
                      ? "border-zinc-950 bg-zinc-950 text-white"
                      : "border-zinc-200 bg-white text-zinc-800 hover:border-zinc-300 hover:bg-zinc-50",
                  )}
                  onClick={() => setSelectedUseCase(useCase.id)}
                >
                  <span>
                    <span className="block text-sm font-medium">{useCase.name}</span>
                    <span className="block text-xs opacity-70">{useCase.packageName}</span>
                  </span>
                  <PlayIcon className="h-4 w-4 shrink-0" />
                </button>
              ))}
            </nav>
            <div className="rounded-lg border border-zinc-200 bg-zinc-50 p-3">
              <img
                src="/goldeneye-stats.png"
                alt="Scene metrics preview"
                className="aspect-[16/9] w-full rounded-md object-cover"
              />
              <div className="mt-3 grid grid-cols-2 gap-2 text-xs text-zinc-600">
                <Metric label="Scenes" value={String(report.video.scenes.length)} />
                <Metric label="Frames" value={String(report.video.frames_processed)} />
              </div>
            </div>
          </div>
        </aside>

        <div className="min-w-0">
          <header className="border-b border-zinc-200 bg-white">
            <div className="mx-auto flex max-w-7xl flex-col gap-4 px-4 py-5 sm:px-6 xl:px-8">
              <div className="flex flex-col gap-4 xl:flex-row xl:items-center xl:justify-between">
                <div>
                  <h1 className="text-2xl font-semibold tracking-normal text-zinc-950">
                    {currentUseCase(selectedUseCase).name}
                  </h1>
                  <p className="mt-1 text-sm text-zinc-500">{report.source.local_video}</p>
                </div>
                <div className="flex flex-wrap gap-2">
                  <IconButton
                    icon={<RefreshIcon className="h-4 w-4" />}
                    label="Sample"
                    onClick={() => {
                      setReport(sampleReport);
                      setRunStatus("succeeded");
                    }}
                  />
                  <IconButton
                    icon={<DownloadIcon className="h-4 w-4" />}
                    label="JSON"
                    onClick={() => downloadReport(report)}
                  />
                </div>
              </div>
              <SegmentedControl<ViewMode>
                value={viewMode}
                options={[
                  ["overview", "Overview"],
                  ["scenes", "Scenes"],
                  ["signals", "Signals"],
                  ["data", "Data"],
                ]}
                onChange={setViewMode}
              />
            </div>
          </header>

          <div className="mx-auto grid max-w-7xl gap-4 px-4 py-5 sm:px-6 xl:grid-cols-[420px_1fr] xl:px-8">
            <section className="space-y-4">
              <UseCaseControls
                form={form}
                onChange={(next) => {
                  setForm(next);
                  setRunStatus("pending");
                }}
                onPreview={() => {
                  setReport(projectReport(sampleReport, form));
                  setRunStatus("succeeded");
                }}
              />
              <CliRunPanel run={cliRun} />
              <JsonReportLoader<YoutubeVideoReport>
                label="Load report JSON"
                onLoad={(nextReport) => {
                  setReport(nextReport);
                  setRunStatus("succeeded");
                }}
              />
            </section>

            <section className="min-w-0 space-y-4">
              {viewMode === "overview" && <Overview report={report} />}
              {viewMode === "scenes" && <Scenes report={report} />}
              {viewMode === "signals" && <Signals report={report} />}
              {viewMode === "data" && <Data report={report} />}
            </section>
          </div>
        </div>
      </div>
    </main>
  );
}

function Overview({ report }: { report: YoutubeVideoReport }) {
  return (
    <>
      <VideoSummaryCards video={report.video} />
      <div className="grid gap-4 2xl:grid-cols-2">
        <SourceSummary source={report.source} />
        <AssetSummary assets={report.assets} />
      </div>
      <CapabilityPanel capabilities={report.capabilities} />
      <ScenePanel scenes={report.video.scenes} />
    </>
  );
}

function Scenes({ report }: { report: YoutubeVideoReport }) {
  return (
    <>
      <ScenePanel scenes={report.video.scenes} />
      <SplitPlanTable scenes={report.video.scenes} videoName={basename(report.source.local_video)} />
    </>
  );
}

function Signals({ report }: { report: YoutubeVideoReport }) {
  return (
    <>
      <ObservationList observations={report.video.observations} title="Video Observations" />
      <ModelObservationGrid observations={report.video.observations} />
      <TranscriptPanel transcription={report.transcription} />
      <div className="grid gap-4 2xl:grid-cols-2">
        <EventList events={report.audio.events} title={`Audio Events (${report.audio.status})`} />
        <EventList events={report.text.events} title={`Text Events (${report.text.status})`} />
      </div>
    </>
  );
}

function Data({ report }: { report: YoutubeVideoReport }) {
  return <DataBucketOverview buckets={report.data_buckets} />;
}

function UseCaseControls({
  form,
  onChange,
  onPreview,
}: {
  form: UseCaseForm;
  onChange: (form: UseCaseForm) => void;
  onPreview: () => void;
}) {
  const set = <K extends keyof UseCaseForm>(key: K, value: UseCaseForm[K]) =>
    onChange({ ...form, [key]: value });

  return (
    <section className="rounded-lg border border-zinc-200 bg-white shadow-sm">
      <div className="border-b border-zinc-200 px-4 py-3">
        <h2 className="text-sm font-semibold text-zinc-950">Run Configuration</h2>
      </div>
      <div className="space-y-4 p-4">
        <SegmentedControl<SourceMode>
          value={form.sourceMode}
          options={[
            ["url", "URL"],
            ["file", "File"],
          ]}
          onChange={(value) => set("sourceMode", value)}
        />

        {form.sourceMode === "url" ? (
          <Field label="YouTube URL">
            <input
              className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm outline-none focus:border-zinc-950 focus:ring-2 focus:ring-zinc-950/10"
              value={form.url}
              onChange={(event) => set("url", event.target.value)}
            />
          </Field>
        ) : (
          <Field label="Input file">
            <input
              className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm outline-none focus:border-zinc-950 focus:ring-2 focus:ring-zinc-950/10"
              value={form.input}
              onChange={(event) => set("input", event.target.value)}
            />
          </Field>
        )}

        <div className="grid gap-3 sm:grid-cols-2">
          <Field label="Scene threshold">
            <input
              type="number"
              min="0"
              step="0.5"
              className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm outline-none focus:border-zinc-950 focus:ring-2 focus:ring-zinc-950/10"
              value={form.sceneThreshold}
              onChange={(event) => set("sceneThreshold", Number(event.target.value))}
            />
          </Field>
          <Field label="Minimum scene frames">
            <input
              type="number"
              min="1"
              step="1"
              className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm outline-none focus:border-zinc-950 focus:ring-2 focus:ring-zinc-950/10"
              value={form.minSceneLen}
              onChange={(event) => set("minSceneLen", Number(event.target.value))}
            />
          </Field>
        </div>

        <div className="grid gap-3 sm:grid-cols-2">
          <Field label="Max frames">
            <input
              type="number"
              min="1"
              step="1"
              placeholder="all"
              className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm outline-none focus:border-zinc-950 focus:ring-2 focus:ring-zinc-950/10"
              value={form.maxFrames}
              onChange={(event) => set("maxFrames", event.target.value)}
            />
          </Field>
          <Field label="Visual sample step">
            <input
              type="number"
              min="1"
              step="1"
              className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm outline-none focus:border-zinc-950 focus:ring-2 focus:ring-zinc-950/10"
              value={form.visualSampleEvery}
              onChange={(event) => set("visualSampleEvery", Number(event.target.value))}
            />
          </Field>
        </div>

        <label className="flex items-center gap-3 rounded-lg border border-zinc-200 px-3 py-2 text-sm text-zinc-800">
          <input
            type="checkbox"
            className="h-4 w-4 rounded border-zinc-300 text-zinc-950 focus:ring-zinc-950"
            checked={form.skipTranscription}
            onChange={(event) => set("skipTranscription", event.target.checked)}
          />
          <span>Skip transcription</span>
        </label>

        <Field label="Output JSON">
          <input
            className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm outline-none focus:border-zinc-950 focus:ring-2 focus:ring-zinc-950/10"
            value={form.output}
            onChange={(event) => set("output", event.target.value)}
          />
        </Field>

        <div className="rounded-lg bg-zinc-950 p-3">
          <pre className="max-h-40 overflow-auto whitespace-pre-wrap break-words text-xs leading-5 text-zinc-100">
            {buildCommand(form).map(shellQuote).join(" ")}
          </pre>
        </div>

        <button
          className="inline-flex w-full items-center justify-center gap-2 rounded-lg bg-zinc-950 px-4 py-2.5 text-sm font-medium text-white hover:bg-zinc-800 focus:outline-none focus:ring-2 focus:ring-zinc-950 focus:ring-offset-2"
          onClick={onPreview}
        >
          <PlayIcon className="h-4 w-4" />
          Preview run
        </button>
      </div>
    </section>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="block">
      <span className="mb-1 block text-xs font-medium uppercase text-zinc-500">{label}</span>
      {children}
    </label>
  );
}

function SegmentedControl<T extends string>({
  value,
  options,
  onChange,
}: {
  value: T;
  options: Array<[T, string]>;
  onChange: (value: T) => void;
}) {
  return (
    <div className="inline-grid w-full grid-flow-col rounded-lg border border-zinc-200 bg-zinc-100 p-1">
      {options.map(([optionValue, label]) => (
        <button
          key={optionValue}
          className={classNames(
            "rounded-md px-3 py-1.5 text-sm font-medium transition focus:outline-none focus:ring-2 focus:ring-zinc-950 focus:ring-offset-2",
            optionValue === value
              ? "bg-white text-zinc-950 shadow-sm"
              : "text-zinc-600 hover:bg-white/70 hover:text-zinc-950",
          )}
          onClick={() => onChange(optionValue)}
        >
          {label}
        </button>
      ))}
    </div>
  );
}

function IconButton({
  icon,
  label,
  onClick,
}: {
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      className="inline-flex min-h-10 items-center gap-2 rounded-lg border border-zinc-300 bg-white px-3 text-sm font-medium text-zinc-800 hover:bg-zinc-50 focus:outline-none focus:ring-2 focus:ring-zinc-950 focus:ring-offset-2"
      onClick={onClick}
    >
      {icon}
      {label}
    </button>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border border-zinc-200 bg-white p-2">
      <div className="text-[11px] uppercase text-zinc-500">{label}</div>
      <div className="font-semibold text-zinc-950">{value}</div>
    </div>
  );
}

function buildCommand(form: UseCaseForm): string[] {
  const command = [
    "cargo",
    "run",
    "-p",
    "video-analysis-use-cases",
    "--",
    "youtube-video",
    "--work-dir",
    form.workDir,
    "--output",
    form.output,
    "--scene-threshold",
    String(form.sceneThreshold),
    "--min-scene-len",
    String(form.minSceneLen),
    "--visual-sample-every",
    String(form.visualSampleEvery),
  ];

  if (form.sourceMode === "url") {
    command.push("--url", form.url);
  } else {
    command.push("--input", form.input);
  }
  if (form.maxFrames.trim()) {
    command.push("--max-frames", form.maxFrames.trim());
  }
  if (form.skipTranscription) {
    command.push("--skip-transcription");
  }
  if (form.objectCommand.trim()) {
    command.push("--object-command", form.objectCommand.trim());
  }
  if (form.ocrCommand.trim()) {
    command.push("--ocr-command", form.ocrCommand.trim());
  }
  if (form.textCommand.trim()) {
    command.push("--text-command", form.textCommand.trim());
  }

  return command;
}

function projectReport(report: YoutubeVideoReport, form: UseCaseForm): YoutubeVideoReport {
  return {
    ...report,
    source: {
      ...report.source,
      url: form.sourceMode === "url" ? form.url : null,
      local_video: form.sourceMode === "file" ? form.input : report.source.local_video,
    },
    assets: {
      ...report.assets,
      work_dir: form.workDir,
      report_path: form.output,
    },
    transcription: {
      ...report.transcription,
      status: form.skipTranscription ? "skipped" : report.transcription.status,
      segments: form.skipTranscription ? [] : report.transcription.segments,
      message: form.skipTranscription ? "Transcription skipped" : report.transcription.message,
    },
  };
}

function downloadReport(report: YoutubeVideoReport) {
  const blob = new Blob([JSON.stringify(report, null, 2)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = "video-analysis-report.json";
  anchor.click();
  URL.revokeObjectURL(url);
}

function currentUseCase(id: UseCaseId) {
  return useCases.find((useCase) => useCase.id === id) ?? useCases[0];
}

function basename(path: string): string {
  const last = path.split(/[\\/]/).pop() ?? path;
  return last.replace(/\.[^.]+$/, "") || "video";
}

function shellQuote(value: string): string {
  if (/^[A-Za-z0-9_./:=+-]+$/.test(value)) {
    return value;
  }
  return `'${value.replace(/'/g, "'\\''")}'`;
}

function classNames(...classes: Array<string | false | null | undefined>): string {
  return classes.filter(Boolean).join(" ");
}

function PlayIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 20 20" fill="currentColor" aria-hidden="true" className={className}>
      <path d="M6.5 4.9v10.2c0 .6.7 1 1.2.6l7.4-5.1c.4-.3.4-.9 0-1.2L7.7 4.3c-.5-.4-1.2 0-1.2.6Z" />
    </svg>
  );
}

function RefreshIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.8" aria-hidden="true" className={className}>
      <path d="M16 6v4h-4" strokeLinecap="round" strokeLinejoin="round" />
      <path d="M15.1 10A5.5 5.5 0 1 1 13.7 6.3L16 8.5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function DownloadIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.8" aria-hidden="true" className={className}>
      <path d="M10 3v9" strokeLinecap="round" />
      <path d="m6.5 8.5 3.5 3.5 3.5-3.5" strokeLinecap="round" strokeLinejoin="round" />
      <path d="M4 15.5h12" strokeLinecap="round" />
    </svg>
  );
}

createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
