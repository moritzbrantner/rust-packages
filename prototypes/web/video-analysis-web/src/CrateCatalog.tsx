import { useEffect, useMemo, useState } from "react";
import {
  CapabilityPanel,
  DataBucketOverview,
  DetectionSummary,
  EventList,
  ModelObservationGrid,
  ScenePanel,
  TranscriptPanel,
  VideoSummaryCards,
  type DetectionResult,
} from "@video-analysis/ui";

import { sampleReport } from "./sampleReport";
import {
  contractTagLabel,
  packageDomainLabels,
  packageDomainOrder,
  packageShortName,
  slugifyPackageName,
  type PackageDomain,
  type WorkspaceArchitectureDependency,
  type WorkspaceArchitecturePackage,
  type WorkspaceArchitectureResponse,
} from "./workspaceArchitecture";
import { fetchWorkspaceArchitecture } from "./workspaceArchitectureClient";

const allDomains = new Set<PackageDomain>(packageDomainOrder);

type PlaygroundMode = "summary" | "signals" | "data";

export function CrateCatalog() {
  const [data, setData] = useState<WorkspaceArchitectureResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [selectedDomains, setSelectedDomains] = useState<Set<PackageDomain>>(allDomains);
  const [selectedPackageName, setSelectedPackageName] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    setError(null);
    fetchWorkspaceArchitecture(controller.signal)
      .then((payload) => setData(payload))
      .catch((fetchError) => {
        if (controller.signal.aborted) {
          return;
        }
        setError(fetchError instanceof Error ? fetchError.message : String(fetchError));
      });

    return () => controller.abort();
  }, []);

  useEffect(() => {
    function syncFromLocation() {
      const slug = crateSlugFromLocation();
      if (!slug || !data) {
        return;
      }
      const match = data.packages.find((pkg) => slugifyPackageName(pkg.name) === slug);
      if (match) {
        setSelectedPackageName(match.name);
      }
    }

    syncFromLocation();
    window.addEventListener("popstate", syncFromLocation);
    return () => window.removeEventListener("popstate", syncFromLocation);
  }, [data]);

  const packageByName = useMemo(
    () => new Map((data?.packages ?? []).map((pkg) => [pkg.name, pkg])),
    [data],
  );

  const visiblePackages = useMemo(() => {
    if (!data) {
      return [];
    }
    const needle = query.trim().toLowerCase();
    return data.packages.filter((pkg) => {
      if (!selectedDomains.has(pkg.domain)) {
        return false;
      }
      if (!needle) {
        return true;
      }
      const searchable = [
        pkg.name,
        pkg.path ?? "",
        pkg.description,
        pkg.role,
        ...pkg.exposes,
        ...pkg.consumedBy,
        ...pkg.tags,
      ]
        .join(" ")
        .toLowerCase();
      return searchable.includes(needle);
    });
  }, [data, query, selectedDomains]);

  useEffect(() => {
    if (!data || visiblePackages.length === 0) {
      return;
    }
    const slug = crateSlugFromLocation();
    const routedPackage = slug
      ? data.packages.find((pkg) => slugifyPackageName(pkg.name) === slug)
      : null;
    if (routedPackage) {
      setSelectedPackageName(routedPackage.name);
      return;
    }
    if (!selectedPackageName || !visiblePackages.some((pkg) => pkg.name === selectedPackageName)) {
      setSelectedPackageName(visiblePackages[0].name);
    }
  }, [data, selectedPackageName, visiblePackages]);

  const selectedPackage =
    (selectedPackageName ? packageByName.get(selectedPackageName) : null) ?? visiblePackages[0] ?? null;
  const dependencies = data?.dependencies ?? [];

  function selectPackage(pkg: WorkspaceArchitecturePackage) {
    setSelectedPackageName(pkg.name);
    window.history.pushState({}, "", crateHref(pkg.name));
  }

  if (error) {
    return (
      <section className="rounded-lg border border-rose-200 bg-rose-50 p-4 text-sm text-rose-700">
        {error}
      </section>
    );
  }

  if (!data) {
    return (
      <section className="rounded-lg border border-zinc-200 bg-white p-4">
        <div className="text-sm font-medium text-zinc-700">Loading crates...</div>
      </section>
    );
  }

  return (
    <section className="grid min-w-0 gap-4 xl:grid-cols-[340px_1fr]">
      <aside className="space-y-4">
        <section className="rounded-lg border border-zinc-200 bg-white">
          <div className="border-b border-zinc-200 px-4 py-3">
            <h2 className="text-sm font-semibold text-zinc-950">Crates</h2>
            <p className="mt-1 text-xs text-zinc-500">{visiblePackages.length} of {data.packages.length} shown</p>
          </div>
          <div className="space-y-4 p-4">
            <label className="block">
              <span className="mb-1 block text-xs font-medium uppercase text-zinc-500">Search</span>
              <input
                className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm outline-none focus:border-zinc-950 focus:ring-2 focus:ring-zinc-950/10"
                value={query}
                placeholder="crate, contract, data type"
                onChange={(event) => setQuery(event.target.value)}
              />
            </label>
            <div>
              <div className="mb-2 text-xs font-medium uppercase text-zinc-500">Domains</div>
              <div className="flex flex-wrap gap-2">
                {packageDomainOrder.map((domain) => (
                  <button
                    key={domain}
                    className={classNames(
                      "rounded-md border px-2.5 py-1.5 text-xs font-medium",
                      selectedDomains.has(domain)
                        ? `${domainBorderClass(domain)} ${domainPanelClass(domain)} text-zinc-950`
                        : "border-zinc-300 bg-white text-zinc-600 hover:bg-zinc-50",
                    )}
                    onClick={() =>
                      setSelectedDomains((current) => {
                        const next = new Set(current);
                        if (next.has(domain)) {
                          next.delete(domain);
                        } else {
                          next.add(domain);
                        }
                        return next.size === 0 ? new Set([domain]) : next;
                      })
                    }
                  >
                    {packageDomainLabels[domain]}
                  </button>
                ))}
              </div>
            </div>
          </div>
        </section>

        <section className="max-h-[980px] overflow-auto rounded-lg border border-zinc-200 bg-white p-3">
          {packageDomainOrder
            .map((domain) => ({
              domain,
              packages: visiblePackages.filter((pkg) => pkg.domain === domain),
            }))
            .filter((group) => group.packages.length > 0)
            .map((group) => (
              <div key={group.domain} className="mb-4 last:mb-0">
                <div className="mb-2 px-1 text-[11px] font-semibold uppercase text-zinc-500">
                  {packageDomainLabels[group.domain]}
                </div>
                <div className="space-y-2">
                  {group.packages.map((pkg) => (
                    <a
                      key={pkg.name}
                      href={crateHref(pkg.name)}
                      className={classNames(
                        "block rounded-lg border px-3 py-2 transition",
                        selectedPackage?.name === pkg.name
                          ? `${domainBorderClass(pkg.domain)} ${domainPanelClass(pkg.domain)}`
                          : "border-zinc-200 bg-white hover:border-zinc-300 hover:bg-zinc-50",
                      )}
                      onClick={(event) => {
                        event.preventDefault();
                        selectPackage(pkg);
                      }}
                    >
                      <div className="text-sm font-medium text-zinc-950">{pkg.name}</div>
                      <div className="mt-1 truncate text-xs text-zinc-500">{pkg.path ?? "workspace package"}</div>
                    </a>
                  ))}
                </div>
              </div>
            ))}
        </section>
      </aside>

      <section className="min-w-0 space-y-4">
        {selectedPackage ? (
          <>
            <CrateDetail pkg={selectedPackage} dependencies={dependencies} packageByName={packageByName} />
            {hasFrontendPlayground(selectedPackage) ? (
              <CratePlayground pkg={selectedPackage} />
            ) : (
              <RustCrateSurface pkg={selectedPackage} />
            )}
          </>
        ) : (
          <section className="rounded-lg border border-dashed border-zinc-300 bg-white p-6 text-sm text-zinc-500">
            No crate matched the current filters.
          </section>
        )}
      </section>
    </section>
  );
}

function CrateDetail({
  pkg,
  dependencies,
  packageByName,
}: {
  pkg: WorkspaceArchitecturePackage;
  dependencies: WorkspaceArchitectureDependency[];
  packageByName: Map<string, WorkspaceArchitecturePackage>;
}) {
  const dependsOn = dependencies
    .filter((dependency) => dependency.source === pkg.name)
    .map((dependency) => packageByName.get(dependency.target))
    .filter(Boolean) as WorkspaceArchitecturePackage[];
  const usedBy = dependencies
    .filter((dependency) => dependency.target === pkg.name)
    .map((dependency) => packageByName.get(dependency.source))
    .filter(Boolean) as WorkspaceArchitecturePackage[];

  return (
    <section className={classNames("rounded-lg border p-4", domainBorderClass(pkg.domain), domainPanelClass(pkg.domain))}>
      <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
        <div className="min-w-0">
          <div className="text-xs font-semibold uppercase text-zinc-500">{packageDomainLabels[pkg.domain]}</div>
          <h2 className="mt-1 break-words text-2xl font-semibold text-zinc-950">{pkg.name}</h2>
          <p className="mt-2 text-sm leading-6 text-zinc-700">{pkg.role || pkg.description}</p>
          <div className="mt-3 flex flex-wrap gap-2">
            {pkg.tags.map((tag) => (
              <span key={tag} className="rounded-full bg-white/80 px-2.5 py-1 text-[11px] font-medium text-zinc-700">
                {contractTagLabel(tag)}
              </span>
            ))}
          </div>
        </div>
        <div className="grid shrink-0 grid-cols-3 gap-2 text-center">
          <MetricTile label="Depends" value={String(dependsOn.length)} />
          <MetricTile label="Used By" value={String(usedBy.length)} />
          <MetricTile label="Kind" value={pkg.kind === "frontend" ? "frontend" : "rust"} />
        </div>
      </div>

      <div className="mt-4 grid gap-3 lg:grid-cols-4">
        {pkg.capabilities.map((capability) => (
          <div key={`${pkg.name}-${capability.kind}`} className="min-w-0 rounded-lg border border-white/70 bg-white/80 p-3">
            <div className="text-[11px] font-semibold uppercase text-zinc-500">{capability.kind}</div>
            <div className="mt-2 break-words text-sm font-medium text-zinc-800">{capability.entrypoint}</div>
          </div>
        ))}
      </div>

      <div className="mt-4 grid gap-3 lg:grid-cols-3">
        <RelationList title="Exposes" items={pkg.exposes} empty="No checked-in expose list." />
        <RelationList title="Depends On" items={dependsOn.map((dependency) => dependency.name)} empty="No direct workspace dependencies." />
        <RelationList title="Used By" items={usedBy.map((dependent) => dependent.name)} empty="No direct workspace dependents." />
      </div>
    </section>
  );
}

function RustCrateSurface({ pkg }: { pkg: WorkspaceArchitecturePackage }) {
  return (
    <section className="rounded-lg border border-zinc-200 bg-white">
      <div className="border-b border-zinc-200 px-4 py-3">
        <h2 className="text-sm font-semibold text-zinc-950">Rust Crate Surface</h2>
        <p className="mt-1 text-xs text-zinc-500">
          This package is represented as a Rust crate, not as a frontend.
        </p>
      </div>
      <div className="grid gap-3 p-4 lg:grid-cols-2">
        <RelationList
          title="Library Contract"
          items={pkg.capabilities
            .filter((capability) => capability.kind === "library")
            .map((capability) => capability.entrypoint)}
          empty="No library target declared."
        />
        <RelationList
          title="Contract Tags"
          items={pkg.tags.map(contractTagLabel)}
          empty="No contract tags detected."
        />
        <RelationList title="Exposes" items={pkg.exposes} empty="No checked-in expose list." />
        <RelationList title="Consumed By" items={pkg.consumedBy} empty="No checked-in consumer list." />
      </div>
    </section>
  );
}

function CratePlayground({ pkg }: { pkg: WorkspaceArchitecturePackage }) {
  const [mode, setMode] = useState<PlaygroundMode>(() => defaultPlaygroundMode(pkg));
  const [sampleSize, setSampleSize] = useState(4);
  const previewReport = useMemo(() => buildPreviewReport(pkg, sampleSize), [pkg, sampleSize]);

  useEffect(() => {
    setMode(defaultPlaygroundMode(pkg));
  }, [pkg]);

  return (
    <section className="rounded-lg border border-zinc-200 bg-white">
      <div className="flex flex-col gap-3 border-b border-zinc-200 px-4 py-3 lg:flex-row lg:items-center lg:justify-between">
        <div>
          <h2 className="text-sm font-semibold text-zinc-950">Frontend Playground</h2>
          <p className="mt-1 text-xs text-zinc-500">Interactive preview seeded from {pkg.name}</p>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <label className="flex items-center gap-2 text-xs font-medium uppercase text-zinc-500">
            Rows
            <input
              className="w-24 accent-zinc-950"
              type="range"
              min="2"
              max="8"
              value={sampleSize}
              onChange={(event) => setSampleSize(Number(event.target.value))}
            />
            <span className="w-4 text-right text-zinc-700">{sampleSize}</span>
          </label>
          <div className="inline-grid grid-flow-col rounded-lg border border-zinc-200 bg-zinc-100 p-1">
            {(["summary", "signals", "data"] as const).map((option) => (
              <button
                key={option}
                className={classNames(
                  "rounded-md px-3 py-1.5 text-xs font-medium capitalize",
                  mode === option ? "bg-white text-zinc-950 shadow-sm" : "text-zinc-600 hover:bg-white/70",
                )}
                onClick={() => setMode(option)}
              >
                {option}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="space-y-4 p-4">
        {mode === "summary" && <SummaryPreview pkg={pkg} report={previewReport} />}
        {mode === "signals" && <SignalsPreview pkg={pkg} report={previewReport} />}
        {mode === "data" && <DataBucketOverview buckets={previewReport.data_buckets} />}
      </div>
    </section>
  );
}

function hasFrontendPlayground(pkg: WorkspaceArchitecturePackage): boolean {
  return pkg.kind === "frontend";
}

function SummaryPreview({ pkg, report }: { pkg: WorkspaceArchitecturePackage; report: typeof sampleReport }) {
  if (pkg.name.includes("detector") || pkg.tags.includes("scenes")) {
    return <DetectionSummary result={detectionResultFromReport(report)} detector={pkg.name} />;
  }
  if (pkg.domain === "video" || pkg.domain === "image" || pkg.domain === "three-d") {
    return (
      <>
        <VideoSummaryCards video={report.video} />
        <ScenePanel scenes={report.video.scenes} />
      </>
    );
  }
  return (
    <CapabilityPanel
      capabilities={{
        completed: pkg.capabilities.map((capability) => `${capability.kind}: ${capability.entrypoint}`),
        skipped: pkg.consumedBy.length > 0 ? [] : ["No downstream consumer declared in docs/API_CONTRACTS.md"],
      }}
    />
  );
}

function SignalsPreview({ pkg, report }: { pkg: WorkspaceArchitecturePackage; report: typeof sampleReport }) {
  if (pkg.domain === "text") {
    return (
      <>
        <TranscriptPanel transcription={report.transcription} />
        <EventList events={report.text.events} title={`${pkg.name} text events`} />
      </>
    );
  }
  if (pkg.domain === "audio") {
    return <EventList events={report.audio.events} title={`${pkg.name} audio events`} />;
  }
  return (
    <>
      <ModelObservationGrid observations={report.video.observations} />
      <EventList events={[...report.audio.events, ...report.text.events]} title={`${pkg.name} events`} />
    </>
  );
}

function RelationList({ title, items, empty }: { title: string; items: string[]; empty: string }) {
  return (
    <div className="rounded-lg border border-white/70 bg-white/80 p-3">
      <div className="text-[11px] font-semibold uppercase text-zinc-500">{title}</div>
      <div className="mt-2 space-y-1.5">
        {items.length === 0 ? (
          <div className="text-sm text-zinc-500">{empty}</div>
        ) : (
          items.slice(0, 8).map((item) => (
            <div key={`${title}-${item}`} className="rounded-md bg-zinc-50 px-2.5 py-1.5 text-sm text-zinc-700">
              {item}
            </div>
          ))
        )}
      </div>
    </div>
  );
}

function MetricTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-white/70 bg-white/80 px-3 py-2">
      <div className="text-[11px] uppercase text-zinc-500">{label}</div>
      <div className="mt-1 truncate text-lg font-semibold text-zinc-950">{value}</div>
    </div>
  );
}

function buildPreviewReport(pkg: WorkspaceArchitecturePackage, sampleSize: number): typeof sampleReport {
  const scenes = sampleReport.video.scenes.slice(0, sampleSize).map((scene, index) => ({
    ...scene,
    index: index + 1,
    observations: sampleReport.video.observations.slice(0, Math.max(1, Math.min(sampleSize, 3))),
  }));
  const observations = sampleReport.video.observations.slice(0, sampleSize).map((observation, index) => ({
    ...observation,
    analyzer: pkg.name,
    label: observation.label ?? `${packageShortName(pkg.name)} ${index + 1}`,
  }));

  return {
    ...sampleReport,
    capabilities: {
      completed: pkg.exposes.length > 0 ? pkg.exposes.slice(0, sampleSize) : pkg.capabilities.map((item) => item.entrypoint),
      skipped: pkg.consumedBy.slice(0, Math.max(0, sampleSize - 2)),
    },
    video: {
      ...sampleReport.video,
      frames_processed: sampleReport.video.frames_processed + sampleSize * 17,
      scenes,
      observations,
    },
    audio: {
      ...sampleReport.audio,
      events: sampleReport.audio.events.slice(0, sampleSize).map((event) => ({ ...event, analyzer: pkg.name })),
    },
    text: {
      ...sampleReport.text,
      events: sampleReport.text.events.slice(0, sampleSize).map((event) => ({ ...event, analyzer: pkg.name })),
    },
    transcription: {
      ...sampleReport.transcription,
      segments: sampleReport.transcription.segments.slice(0, sampleSize),
    },
    data_buckets: sampleReport.data_buckets.slice(0, Math.max(1, Math.min(sampleSize, sampleReport.data_buckets.length))),
  };
}

function detectionResultFromReport(report: typeof sampleReport): DetectionResult {
  return {
    frames_processed: report.video.frames_processed,
    scenes: report.video.scenes.map((scene) => ({
      start: { frame_index: scene.start_frame, timestamp: { pts: scene.start_frame, timebase: { num: 1, den: 30 }, seconds: scene.start_seconds } },
      end: { frame_index: scene.end_frame, timestamp: { pts: scene.end_frame, timebase: { num: 1, den: 30 }, seconds: scene.end_seconds } },
    })),
    cuts: report.video.scenes.slice(1).map((scene, index) => ({
      position: {
        frame_index: scene.start_frame,
        timestamp: { pts: scene.start_frame, timebase: { num: 1, den: 30 }, seconds: scene.start_seconds },
      },
      detector: "scene-delta",
      score: 0.72 + index * 0.03,
    })),
  };
}

function defaultPlaygroundMode(pkg: WorkspaceArchitecturePackage): PlaygroundMode {
  if (pkg.domain === "data" || pkg.domain === "vector" || pkg.tags.includes("data_buckets")) {
    return "data";
  }
  if (pkg.domain === "audio" || pkg.domain === "text" || pkg.tags.includes("analysis_events")) {
    return "signals";
  }
  return "summary";
}

function crateHref(packageName: string): string {
  const base = import.meta.env.BASE_URL || "/";
  return `${base.endsWith("/") ? base : `${base}/`}crates/${slugifyPackageName(packageName)}/`;
}

function crateSlugFromLocation(): string | null {
  const base = new URL(import.meta.env.BASE_URL || "/", window.location.origin).pathname;
  const pathname = window.location.pathname.startsWith(base)
    ? window.location.pathname.slice(base.length)
    : window.location.pathname.replace(/^\//, "");
  const match = pathname.match(/^crates\/([^/]+)/);
  return match ? decodeURIComponent(match[1]) : null;
}

function domainBorderClass(domain: PackageDomain): string {
  switch (domain) {
    case "facade":
      return "border-zinc-950";
    case "apps":
      return "border-cyan-300";
    case "ui":
      return "border-sky-300";
    case "video":
      return "border-emerald-300";
    case "audio":
      return "border-amber-300";
    case "image":
      return "border-rose-300";
    case "text":
      return "border-fuchsia-300";
    case "vector":
      return "border-indigo-300";
    case "three-d":
      return "border-violet-300";
    case "comfyui":
      return "border-orange-300";
    case "data":
      return "border-slate-300";
    case "bindings":
      return "border-teal-300";
    case "support":
      return "border-zinc-300";
  }
}

function domainPanelClass(domain: PackageDomain): string {
  switch (domain) {
    case "facade":
      return "bg-zinc-100";
    case "apps":
      return "bg-cyan-50";
    case "ui":
      return "bg-sky-50";
    case "video":
      return "bg-emerald-50";
    case "audio":
      return "bg-amber-50";
    case "image":
      return "bg-rose-50";
    case "text":
      return "bg-fuchsia-50";
    case "vector":
      return "bg-indigo-50";
    case "three-d":
      return "bg-violet-50";
    case "comfyui":
      return "bg-orange-50";
    case "data":
      return "bg-slate-50";
    case "bindings":
      return "bg-teal-50";
    case "support":
      return "bg-zinc-50";
  }
}

function classNames(...classes: Array<string | false | null | undefined>): string {
  return classes.filter(Boolean).join(" ");
}
