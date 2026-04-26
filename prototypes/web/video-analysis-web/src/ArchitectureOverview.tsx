import { useEffect, useMemo, useState } from "react";

import {
  contractTagLabel,
  packageDomainLabels,
  packageDomainOrder,
  packageShortName,
  type PackageDomain,
  type WorkspaceArchitectureDependency,
  type WorkspaceArchitectureInterop,
  type WorkspaceArchitecturePackage,
  type WorkspaceArchitectureResponse,
} from "./workspaceArchitecture";

const allDomains = new Set<PackageDomain>(packageDomainOrder);

export function ArchitectureOverview() {
  const [data, setData] = useState<WorkspaceArchitectureResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [selectedDomains, setSelectedDomains] = useState<Set<PackageDomain>>(allDomains);
  const [selectedPackageName, setSelectedPackageName] = useState("video-analysis-use-cases");

  useEffect(() => {
    const controller = new AbortController();
    setError(null);
    fetch("/api/workspace-architecture", { signal: controller.signal })
      .then(async (response) => {
        if (!response.ok) {
          throw new Error((await response.text()) || "Could not load workspace architecture");
        }
        return response.json() as Promise<WorkspaceArchitectureResponse>;
      })
      .then((payload) => setData(payload))
      .catch((fetchError) => {
        if (controller.signal.aborted) {
          return;
        }
        setError(fetchError instanceof Error ? fetchError.message : String(fetchError));
      });

    return () => controller.abort();
  }, []);

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
      const searchable = [pkg.name, pkg.role, pkg.description, pkg.path ?? "", ...pkg.tags].join(" ").toLowerCase();
      return searchable.includes(needle);
    });
  }, [data, query, selectedDomains]);

  useEffect(() => {
    if (visiblePackages.length === 0) {
      return;
    }
    if (!visiblePackages.some((pkg) => pkg.name === selectedPackageName)) {
      setSelectedPackageName(visiblePackages[0].name);
    }
  }, [selectedPackageName, visiblePackages]);

  const packageByName = useMemo(
    () => new Map((data?.packages ?? []).map((pkg) => [pkg.name, pkg])),
    [data],
  );
  const visibleNames = useMemo(() => new Set(visiblePackages.map((pkg) => pkg.name)), [visiblePackages]);
  const selectedPackage = packageByName.get(selectedPackageName) ?? visiblePackages[0] ?? null;

  const dependenciesBySource = useMemo(
    () => groupDependenciesBySource(data?.dependencies ?? []),
    [data],
  );
  const dependentsByTarget = useMemo(
    () => groupDependentsByTarget(data?.dependencies ?? []),
    [data],
  );
  const interopByPackage = useMemo(
    () => groupInteropByPackage(data?.interop ?? []),
    [data],
  );
  const interopLookup = useMemo(
    () => buildInteropLookup(data?.interop ?? []),
    [data],
  );

  const directDependencies = selectedPackage
    ? (dependenciesBySource.get(selectedPackage.name) ?? []).map((dependency) => ({
        dependency,
        target: packageByName.get(dependency.target),
      }))
    : [];
  const directDependents = selectedPackage
    ? (dependentsByTarget.get(selectedPackage.name) ?? []).map((dependency) => ({
        dependency,
        source: packageByName.get(dependency.source),
      }))
    : [];
  const selectedInterop = selectedPackage
    ? (interopByPackage.get(selectedPackage.name) ?? [])
        .map((relation) => ({
          relation,
          package:
            packageByName.get(
              relation.packages[0] === selectedPackage.name ? relation.packages[1] : relation.packages[0],
            ) ?? null,
        }))
        .filter((entry) => entry.package)
        .slice(0, 10)
    : [];

  const visibleDependencies = (data?.dependencies ?? []).filter(
    (dependency) => visibleNames.has(dependency.source) && visibleNames.has(dependency.target),
  );
  const visibleInterop = (data?.interop ?? []).filter(
    (relation) => visibleNames.has(relation.packages[0]) && visibleNames.has(relation.packages[1]),
  );

  if (error) {
    return (
      <section className="rounded-lg border border-rose-200 bg-rose-50 p-4 text-sm text-rose-700 shadow-sm">
        {error}
      </section>
    );
  }

  if (!data) {
    return (
      <section className="rounded-lg border border-zinc-200 bg-white p-4 shadow-sm">
        <div className="text-sm font-medium text-zinc-700">Loading workspace architecture…</div>
      </section>
    );
  }

  return (
    <section className="space-y-4">
      <div className="grid gap-4 xl:grid-cols-4">
        <SummaryCard label="Packages" value={String(visiblePackages.length)} detail="Visible after filters" />
        <SummaryCard label="Dependency Edges" value={String(visibleDependencies.length)} detail="Direct package links" />
        <SummaryCard label="Interop Pairs" value={String(visibleInterop.length)} detail="Direct or shared contract ties" />
        <SummaryCard
          label="Contract Families"
          value={String(new Set(visiblePackages.flatMap((pkg) => pkg.tags)).size)}
          detail="Shared data surfaces detected"
        />
      </div>

      <section className="rounded-lg border border-zinc-200 bg-white shadow-sm">
        <div className="border-b border-zinc-200 px-4 py-3">
          <h2 className="text-sm font-semibold text-zinc-950">Filters</h2>
        </div>
        <div className="space-y-4 p-4">
          <div className="grid gap-4 xl:grid-cols-[320px_1fr]">
            <label className="block">
              <span className="mb-1 block text-xs font-medium uppercase text-zinc-500">Search packages</span>
              <input
                className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm outline-none focus:border-zinc-950 focus:ring-2 focus:ring-zinc-950/10"
                value={query}
                placeholder="video-analysis-core, transcript, radiance…"
                onChange={(event) => setQuery(event.target.value)}
              />
            </label>
            <div>
              <div className="mb-1 text-xs font-medium uppercase text-zinc-500">Domains</div>
              <div className="flex flex-wrap gap-2">
                <button
                  className={classNames(
                    "rounded-full border px-3 py-1.5 text-xs font-medium",
                    selectedDomains.size === packageDomainOrder.length
                      ? "border-zinc-950 bg-zinc-950 text-white"
                      : "border-zinc-300 bg-white text-zinc-700 hover:bg-zinc-50",
                  )}
                  onClick={() => setSelectedDomains(new Set(packageDomainOrder))}
                >
                  All
                </button>
                {packageDomainOrder.map((domain) => (
                  <button
                    key={domain}
                    className={classNames(
                      "rounded-full border px-3 py-1.5 text-xs font-medium",
                      selectedDomains.has(domain)
                        ? `${domainBorderClass(domain)} ${domainChipClass(domain)}`
                        : "border-zinc-300 bg-white text-zinc-700 hover:bg-zinc-50",
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
        </div>
      </section>

      <div className="grid gap-4 2xl:grid-cols-[320px_1fr]">
        <section className="rounded-lg border border-zinc-200 bg-white shadow-sm">
          <div className="flex items-center justify-between border-b border-zinc-200 px-4 py-3">
            <h2 className="text-sm font-semibold text-zinc-950">Packages</h2>
            <span className="text-xs text-zinc-500">{visiblePackages.length} visible</span>
          </div>
          <div className="max-h-[880px] space-y-4 overflow-auto p-4">
            {packageDomainOrder
              .map((domain) => ({
                domain,
                packages: visiblePackages.filter((pkg) => pkg.domain === domain),
              }))
              .filter((group) => group.packages.length > 0)
              .map((group) => (
                <div key={group.domain}>
                  <div className="mb-2 text-[11px] font-semibold uppercase text-zinc-500">
                    {packageDomainLabels[group.domain]}
                  </div>
                  <div className="space-y-2">
                    {group.packages.map((pkg) => (
                      <button
                        key={pkg.name}
                        className={classNames(
                          "w-full rounded-lg border px-3 py-2 text-left transition",
                          selectedPackage?.name === pkg.name
                            ? `${domainBorderClass(pkg.domain)} ${domainSelectedClass(pkg.domain)}`
                            : "border-zinc-200 bg-white text-zinc-800 hover:border-zinc-300 hover:bg-zinc-50",
                        )}
                        onClick={() => setSelectedPackageName(pkg.name)}
                      >
                        <div className="text-sm font-medium">{pkg.name}</div>
                        <div className="mt-1 text-xs text-zinc-500">{pkg.path ?? "workspace package"}</div>
                      </button>
                    ))}
                  </div>
                </div>
              ))}
          </div>
        </section>

        <section className="space-y-4">
          <section className="rounded-lg border border-zinc-200 bg-white shadow-sm">
            <div className="flex flex-col gap-3 border-b border-zinc-200 px-4 py-3 xl:flex-row xl:items-start xl:justify-between">
              <div>
                <h2 className="text-sm font-semibold text-zinc-950">Dependency Chart</h2>
                {selectedPackage && (
                  <p className="mt-1 text-sm text-zinc-600">
                    {selectedPackage.name} | {selectedPackage.path ?? "workspace package"}
                  </p>
                )}
              </div>
              {selectedPackage && (
                <div className="flex flex-wrap gap-2">
                  {selectedPackage.tags.map((tag) => (
                    <span
                      key={tag}
                      className="rounded-full bg-zinc-100 px-2.5 py-1 text-[11px] font-medium text-zinc-700"
                    >
                      {contractTagLabel(tag)}
                    </span>
                  ))}
                </div>
              )}
            </div>

            {selectedPackage ? (
              <div className="space-y-4 p-4">
                <div className={classNames("rounded-xl border p-4", domainBorderClass(selectedPackage.domain), domainPanelClass(selectedPackage.domain))}>
                  <div className="flex flex-col gap-3 xl:flex-row xl:items-start xl:justify-between">
                    <div>
                      <div className="text-lg font-semibold text-zinc-950">{selectedPackage.name}</div>
                      <div className="mt-1 text-sm text-zinc-600">{selectedPackage.role}</div>
                      <div className="mt-2 text-xs text-zinc-500">{selectedPackage.description}</div>
                    </div>
                    <div className="grid shrink-0 grid-cols-3 gap-2 text-center">
                      <MetricTile label="Depends On" value={String(directDependencies.length)} />
                      <MetricTile label="Used By" value={String(directDependents.length)} />
                      <MetricTile label="Interop" value={String(interopByPackage.get(selectedPackage.name)?.length ?? 0)} />
                    </div>
                  </div>
                </div>

                <div className="grid gap-4 xl:grid-cols-[1fr_320px_1fr]">
                  <RelationColumn
                    title="Depends On"
                    empty="No direct workspace dependencies in the current view."
                    items={directDependencies.map((entry) => ({
                      package: entry.target ?? null,
                      subtitle: entry.dependency.optional ? "optional dependency" : "direct dependency",
                    }))}
                    selectedPackageName={selectedPackage.name}
                    visibleNames={visibleNames}
                    onSelect={setSelectedPackageName}
                  />
                  <div className="rounded-lg border border-dashed border-zinc-300 bg-zinc-50 p-4">
                    <div className="text-xs font-semibold uppercase text-zinc-500">Shared Data Surface</div>
                    <div className="mt-3 space-y-2">
                      {selectedPackage.exposes.length === 0 ? (
                        <div className="text-sm text-zinc-500">No checked-in expose list for this package.</div>
                      ) : (
                        selectedPackage.exposes.slice(0, 8).map((entry) => (
                          <div key={entry} className="rounded-md border border-zinc-200 bg-white px-3 py-2 text-sm text-zinc-700">
                            {entry}
                          </div>
                        ))
                      )}
                    </div>
                  </div>
                  <RelationColumn
                    title="Used By"
                    empty="No direct workspace dependents in the current view."
                    items={directDependents.map((entry) => ({
                      package: entry.source ?? null,
                      subtitle: "direct dependent",
                    }))}
                    selectedPackageName={selectedPackage.name}
                    visibleNames={visibleNames}
                    onSelect={setSelectedPackageName}
                  />
                </div>

                <div className="rounded-lg border border-zinc-200 bg-zinc-50 p-4">
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <h3 className="text-sm font-semibold text-zinc-950">Top Interchange Partners</h3>
                      <p className="mt-1 text-xs text-zinc-500">Shared contracts or a direct dependency edge.</p>
                    </div>
                    <span className="text-xs text-zinc-500">{selectedInterop.length} shown</span>
                  </div>
                  <div className="mt-3 grid gap-3 xl:grid-cols-2">
                    {selectedInterop.length === 0 ? (
                      <div className="rounded-md border border-dashed border-zinc-300 bg-white px-3 py-4 text-sm text-zinc-500">
                        No related packages matched the current filters.
                      </div>
                    ) : (
                      selectedInterop.map((entry) => (
                        <button
                          key={`${selectedPackage.name}-${entry.package?.name}`}
                          className="rounded-lg border border-zinc-200 bg-white px-3 py-3 text-left hover:border-zinc-300 hover:bg-zinc-50"
                          onClick={() => entry.package && setSelectedPackageName(entry.package.name)}
                        >
                          <div className="flex items-center justify-between gap-3">
                            <div className="text-sm font-medium text-zinc-900">{entry.package?.name}</div>
                            <span className="rounded bg-zinc-100 px-2 py-0.5 text-[11px] font-medium text-zinc-700">
                              {entry.relation.directDependency ? "dep + data" : "shared data"}
                            </span>
                          </div>
                          <div className="mt-2 flex flex-wrap gap-1.5">
                            {entry.relation.sharedTags.map((tag) => (
                              <span
                                key={`${entry.package?.name}-${tag}`}
                                className="rounded-full bg-zinc-100 px-2 py-1 text-[11px] font-medium text-zinc-700"
                              >
                                {contractTagLabel(tag)}
                              </span>
                            ))}
                          </div>
                        </button>
                      ))
                    )}
                  </div>
                </div>
              </div>
            ) : (
              <div className="p-4 text-sm text-zinc-500">No package matched the current filters.</div>
            )}
          </section>

          <section className="rounded-lg border border-zinc-200 bg-white shadow-sm">
            <div className="flex flex-col gap-2 border-b border-zinc-200 px-4 py-3 xl:flex-row xl:items-center xl:justify-between">
              <div>
                <h2 className="text-sm font-semibold text-zinc-950">Interchange Matrix</h2>
                <p className="mt-1 text-xs text-zinc-500">
                  A filled cell means the pair has a direct dependency edge, a shared contract family, or both.
                </p>
              </div>
              <div className="flex flex-wrap gap-2 text-[11px] text-zinc-600">
                <LegendSwatch label="Direct dependency" className="bg-amber-400" />
                <LegendSwatch label="Shared contract" className="bg-sky-300" />
                <LegendSwatch label="Both" className="bg-emerald-400" />
              </div>
            </div>
            <div className="overflow-auto">
              <table className="min-w-max border-separate border-spacing-0">
                <thead>
                  <tr>
                    <th className="sticky left-0 top-0 z-20 border-b border-r border-zinc-200 bg-white px-3 py-2 text-left text-xs font-semibold uppercase text-zinc-500">
                      Package
                    </th>
                    {visiblePackages.map((columnPackage) => (
                      <th
                        key={columnPackage.name}
                        className={classNames(
                          "sticky top-0 z-10 min-w-12 border-b border-zinc-200 bg-white px-1 py-2 align-bottom",
                          selectedPackage?.name === columnPackage.name && "bg-zinc-50",
                        )}
                      >
                        <button
                          className="h-28 w-10 text-[11px] font-medium text-zinc-700"
                          style={{ writingMode: "vertical-rl", transform: "rotate(180deg)" }}
                          onClick={() => setSelectedPackageName(columnPackage.name)}
                          title={columnPackage.name}
                        >
                          {packageShortName(columnPackage.name)}
                        </button>
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {visiblePackages.map((rowPackage) => (
                    <tr key={rowPackage.name}>
                      <th
                        className={classNames(
                          "sticky left-0 z-10 border-r border-b border-zinc-200 bg-white px-3 py-2 text-left",
                          selectedPackage?.name === rowPackage.name && "bg-zinc-50",
                        )}
                      >
                        <button className="text-sm font-medium text-zinc-800" onClick={() => setSelectedPackageName(rowPackage.name)}>
                          {rowPackage.name}
                        </button>
                        <div className="mt-0.5 text-[11px] text-zinc-500">{packageDomainLabels[rowPackage.domain]}</div>
                      </th>
                      {visiblePackages.map((columnPackage) => {
                        if (rowPackage.name === columnPackage.name) {
                          return (
                            <td key={`${rowPackage.name}-${columnPackage.name}`} className="border-b border-zinc-100 bg-zinc-50/70 px-1 py-1">
                              <div className="h-7 w-7 rounded-md border border-zinc-200 bg-white/80" />
                            </td>
                          );
                        }
                        const relation = interopLookup.get(pairKey(rowPackage.name, columnPackage.name)) ?? null;
                        return (
                          <td key={`${rowPackage.name}-${columnPackage.name}`} className="border-b border-zinc-100 px-1 py-1">
                            <button
                              className={classNames(
                                "h-7 w-7 rounded-md border transition",
                                matrixCellClass(relation),
                                (selectedPackage?.name === rowPackage.name || selectedPackage?.name === columnPackage.name) &&
                                  "ring-1 ring-zinc-300",
                              )}
                              title={matrixCellTitle(rowPackage.name, columnPackage.name, relation)}
                              onClick={() => setSelectedPackageName(rowPackage.name)}
                            />
                          </td>
                        );
                      })}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>
        </section>
      </div>
    </section>
  );
}

function SummaryCard({ label, value, detail }: { label: string; value: string; detail: string }) {
  return (
    <section className="rounded-lg border border-zinc-200 bg-white p-4 shadow-sm">
      <div className="text-xs font-medium uppercase text-zinc-500">{label}</div>
      <div className="mt-2 text-2xl font-semibold text-zinc-950">{value}</div>
      <div className="mt-1 text-sm text-zinc-600">{detail}</div>
    </section>
  );
}

function MetricTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-zinc-200 bg-white px-3 py-2">
      <div className="text-[11px] uppercase text-zinc-500">{label}</div>
      <div className="mt-1 text-lg font-semibold text-zinc-950">{value}</div>
    </div>
  );
}

function RelationColumn({
  title,
  empty,
  items,
  selectedPackageName,
  visibleNames,
  onSelect,
}: {
  title: string;
  empty: string;
  items: Array<{ package: WorkspaceArchitecturePackage | null; subtitle: string }>;
  selectedPackageName: string;
  visibleNames: Set<string>;
  onSelect: (packageName: string) => void;
}) {
  return (
    <div className="rounded-lg border border-zinc-200 bg-zinc-50 p-4">
      <div className="mb-3 text-xs font-semibold uppercase text-zinc-500">{title}</div>
      <div className="space-y-2">
        {items.length === 0 ? (
          <div className="rounded-md border border-dashed border-zinc-300 bg-white px-3 py-4 text-sm text-zinc-500">
            {empty}
          </div>
        ) : (
          items.map((item) =>
            item.package ? (
              <button
                key={`${selectedPackageName}-${item.package.name}-${title}`}
                className={classNames(
                  "w-full rounded-md border px-3 py-2 text-left",
                  visibleNames.has(item.package.name)
                    ? "border-zinc-200 bg-white hover:border-zinc-300 hover:bg-zinc-50"
                    : "border-zinc-200 bg-white/70 text-zinc-500",
                )}
                onClick={() => onSelect(item.package!.name)}
              >
                <div className="text-sm font-medium text-zinc-900">{item.package.name}</div>
                <div className="mt-1 text-[11px] text-zinc-500">{item.subtitle}</div>
              </button>
            ) : null,
          )
        )}
      </div>
    </div>
  );
}

function LegendSwatch({ label, className }: { label: string; className: string }) {
  return (
    <div className="inline-flex items-center gap-2">
      <span className={classNames("h-3 w-3 rounded-sm border border-zinc-300", className)} />
      <span>{label}</span>
    </div>
  );
}

function groupDependenciesBySource(
  dependencies: WorkspaceArchitectureDependency[],
): Map<string, WorkspaceArchitectureDependency[]> {
  const result = new Map<string, WorkspaceArchitectureDependency[]>();
  for (const dependency of dependencies) {
    const current = result.get(dependency.source) ?? [];
    current.push(dependency);
    result.set(dependency.source, current);
  }
  return result;
}

function groupDependentsByTarget(
  dependencies: WorkspaceArchitectureDependency[],
): Map<string, WorkspaceArchitectureDependency[]> {
  const result = new Map<string, WorkspaceArchitectureDependency[]>();
  for (const dependency of dependencies) {
    const current = result.get(dependency.target) ?? [];
    current.push(dependency);
    result.set(dependency.target, current);
  }
  return result;
}

function groupInteropByPackage(
  interop: WorkspaceArchitectureInterop[],
): Map<string, WorkspaceArchitectureInterop[]> {
  const result = new Map<string, WorkspaceArchitectureInterop[]>();
  for (const relation of interop) {
    for (const packageName of relation.packages) {
      const current = result.get(packageName) ?? [];
      current.push(relation);
      current.sort((left, right) => right.strength - left.strength);
      result.set(packageName, current);
    }
  }
  return result;
}

function buildInteropLookup(interop: WorkspaceArchitectureInterop[]): Map<string, WorkspaceArchitectureInterop> {
  return new Map(interop.map((relation) => [pairKey(relation.packages[0], relation.packages[1]), relation]));
}

function pairKey(left: string, right: string): string {
  return [left, right].sort((a, b) => a.localeCompare(b)).join("::");
}

function matrixCellClass(relation: WorkspaceArchitectureInterop | null): string {
  if (!relation) {
    return "border-zinc-200 bg-white hover:bg-zinc-50";
  }
  if (relation.directDependency && relation.sharedTags.length > 0) {
    return "border-emerald-500 bg-emerald-400/80 hover:bg-emerald-400";
  }
  if (relation.directDependency) {
    return "border-amber-500 bg-amber-300 hover:bg-amber-400/80";
  }
  return "border-sky-400 bg-sky-200 hover:bg-sky-300";
}

function matrixCellTitle(
  rowPackageName: string,
  columnPackageName: string,
  relation: WorkspaceArchitectureInterop | null,
): string {
  if (!relation) {
    return `${rowPackageName} and ${columnPackageName}: no direct dependency or shared contract detected`;
  }
  return `${rowPackageName} and ${columnPackageName}: ${relation.reasons.join(", ")}`;
}

function domainChipClass(domain: PackageDomain): string {
  switch (domain) {
    case "facade":
      return "bg-zinc-950 text-white";
    case "apps":
      return "bg-cyan-600 text-white";
    case "ui":
      return "bg-sky-600 text-white";
    case "video":
      return "bg-emerald-600 text-white";
    case "audio":
      return "bg-amber-500 text-zinc-950";
    case "image":
      return "bg-rose-500 text-white";
    case "text":
      return "bg-fuchsia-600 text-white";
    case "vector":
      return "bg-indigo-600 text-white";
    case "three-d":
      return "bg-violet-600 text-white";
    case "comfyui":
      return "bg-orange-500 text-zinc-950";
    case "data":
      return "bg-slate-600 text-white";
    case "bindings":
      return "bg-teal-600 text-white";
    case "support":
      return "bg-zinc-500 text-white";
  }
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

function domainSelectedClass(domain: PackageDomain): string {
  return `${domainPanelClass(domain)} text-zinc-950 shadow-sm`;
}

function classNames(...classes: Array<string | false | null | undefined>): string {
  return classes.filter(Boolean).join(" ");
}
