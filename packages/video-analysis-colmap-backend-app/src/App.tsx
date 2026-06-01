import {
  COLMAP_TEST_VIDEO_OUTPUT_DIR,
  COLMAP_TEST_VIDEO_PATH,
  COLMAP_TEST_VIDEO_URL,
  PackageSurfaceWorkbench,
  type PackageAppConfig,
  type SurfaceResponse,
} from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/video-analysis-colmap-backend-wasm";

import { ColmapSceneViewer } from "./ColmapSceneViewer";

const testVideoInput = {
  videoPath: COLMAP_TEST_VIDEO_PATH,
  videoUrl: COLMAP_TEST_VIDEO_URL,
  outputDir: COLMAP_TEST_VIDEO_OUTPUT_DIR,
  frameFps: 2,
  maxFrames: 80,
  imageWidth: 1280,
  clean: false,
};

const packageAppConfig: PackageAppConfig = {
  library: "video-analysis-colmap-backend",
  title: "Video Analysis Colmap Backend",
  description: "COLMAP compatibility backend and parity reporting for video-analysis.",
  domain: "video",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/video-analysis-colmap-backend",
    standaloneRoute: "",
  },
  featuredOperations: ["video.colmap.reconstructVideo"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the native video-to-sparse-reconstruction path.",
      operations: ["video.colmap.reconstructVideo"],
    },
    {
      id: "advanced",
      label: "Debug",
      description: "Inspect COLMAP command plans and already-parsed JSON inputs without running external tools.",
      operations: ["video.colmap.commandPlan", "video.colmap.imageList", "video.colmap.sparseSummary", "describe"],
    },
  ],
  defaultOperation: "video.colmap.reconstructVideo",
  defaultRuntime: "overview-server",
  presets: [
    {
      id: "test-video",
      label: "Test Video",
      operation: "video.colmap.reconstructVideo",
      input: testVideoInput,
      description: "Local COLMAP reconstruction input for the downloaded test-video.mp4.",
    },
  ],
  resultTabs: [
    {
      id: "3d-view",
      label: "3D View",
      render: (response) => <ColmapSceneViewer response={response} />,
    },
  ],
  children: ({ input, response }) => <ColmapWorkflowPanel input={input} response={response} />,
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}

function ColmapWorkflowPanel({ input, response }: { input: unknown; response: SurfaceResponse | null }) {
  const value = response?.value && typeof response.value === "object" && !Array.isArray(response.value) ? (response.value as Record<string, unknown>) : {};
  const summary = value.summary && typeof value.summary === "object" && !Array.isArray(value.summary) ? (value.summary as Record<string, unknown>) : {};
  const frames = value.frames && typeof value.frames === "object" && !Array.isArray(value.frames) ? (value.frames as Record<string, unknown>) : {};
  const colmap = value.colmap && typeof value.colmap === "object" && !Array.isArray(value.colmap) ? (value.colmap as Record<string, unknown>) : {};
  const diagnostics = response?.diagnostics ?? [];
  const artifacts = response?.artifacts ?? [];

  return (
    <section className="rounded-md border border-zinc-200 bg-white p-4">
      <div className="flex items-center justify-between gap-3">
        <h2 className="text-sm font-semibold text-zinc-950">COLMAP Run</h2>
        <span className={diagnostics.length ? "rounded-md bg-amber-50 px-2 py-1 text-xs font-semibold text-amber-800" : "rounded-md bg-emerald-50 px-2 py-1 text-xs font-semibold text-emerald-800"}>
          {response ? (diagnostics.length ? `${diagnostics.length} diagnostics` : "Ready") : "Not run"}
        </span>
      </div>
      <VideoPreview input={input} />
      <dl className="mt-4 grid gap-3 text-sm sm:grid-cols-2">
        <WorkflowMetric label="Frames" value={stringValue(frames.count, "0")} />
        <WorkflowMetric label="Cameras" value={stringValue(summary.registeredImageCount, "0")} />
        <WorkflowMetric label="Sparse points" value={stringValue(summary.sparsePointCount, "0")} />
        <WorkflowMetric label="Artifacts" value={String(artifacts.length)} />
      </dl>
      <div className="mt-4 grid gap-2 text-xs text-zinc-600">
        <PathRow label="Frames" value={frames.dir} />
        <PathRow label="Database" value={colmap.databasePath} />
        <PathRow label="Sparse text" value={colmap.sparseTextDir} />
      </div>
    </section>
  );
}

function VideoPreview({ input }: { input: unknown }) {
  const object = input && typeof input === "object" && !Array.isArray(input) ? (input as Record<string, unknown>) : {};
  const videoUrl = typeof object.videoUrl === "string" ? object.videoUrl : "";
  const videoDataUrl = typeof object.videoDataUrl === "string" ? object.videoDataUrl : "";
  const src = videoDataUrl || videoUrl;

  return (
    <div className="mt-3">
      {src ? (
        <video className="mt-3 aspect-video w-full rounded-md bg-black object-contain" controls src={src} />
      ) : (
        <div className="mt-3 flex aspect-video items-center justify-center rounded-md border border-dashed border-zinc-300 bg-zinc-50 px-4 text-center text-sm text-zinc-500">
          No preview video selected.
        </div>
      )}
    </div>
  );
}

function WorkflowMetric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md bg-zinc-50 px-3 py-2">
      <dt className="text-xs font-semibold uppercase text-zinc-500">{label}</dt>
      <dd className="mt-1 font-mono text-zinc-950">{value}</dd>
    </div>
  );
}

function PathRow({ label, value }: { label: string; value: unknown }) {
  if (typeof value !== "string" || value.length === 0) {
    return null;
  }
  return (
    <div>
      <span className="font-semibold text-zinc-700">{label}</span>
      <span className="ml-2 break-all font-mono">{value}</span>
    </div>
  );
}

function stringValue(value: unknown, fallback: string) {
  return typeof value === "number" || typeof value === "string" ? String(value) : fallback;
}
