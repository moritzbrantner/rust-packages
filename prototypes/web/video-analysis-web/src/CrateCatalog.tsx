import { useEffect, useMemo, useState, type ComponentType } from "react";

import {
  packageDomainLabels,
  packageDomainOrder,
  slugifyPackageName,
  type PackageDomain,
  type WorkspaceArchitecturePackage,
  type WorkspaceArchitectureResponse,
} from "./workspaceArchitecture";
import { fetchWorkspaceArchitecture } from "./workspaceArchitectureClient";

type CatalogRoute = { kind: "home" } | { kind: "wrapper"; slug: string };
type AppModule = { App: ComponentType };
type WrapperAppState =
  | { kind: "loading" }
  | { kind: "ready"; App: ComponentType }
  | { kind: "fallback" }
  | { kind: "error"; message: string };

const wrapperAppModules = import.meta.glob<AppModule>("../../../../packages/*-app/src/App.tsx");

export function CrateCatalog() {
  const [data, setData] = useState<WorkspaceArchitectureResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [selectedDomain, setSelectedDomain] = useState<PackageDomain | "all">("all");
  const [route, setRoute] = useState<CatalogRoute>(() => catalogRouteFromLocation());

  useEffect(() => {
    const controller = new AbortController();
    setError(null);
    fetchWorkspaceArchitecture(controller.signal)
      .then((payload) => setData(payload))
      .catch((caught) => {
        if (!controller.signal.aborted) {
          setError(caught instanceof Error ? caught.message : String(caught));
        }
      });
    return () => controller.abort();
  }, []);

  useEffect(() => {
    function syncFromLocation() {
      setRoute(catalogRouteFromLocation());
    }

    window.addEventListener("popstate", syncFromLocation);
    return () => window.removeEventListener("popstate", syncFromLocation);
  }, []);

  const wrappers = useMemo(
    () => (data?.packages ?? []).filter(isServerPackage).sort(comparePackages),
    [data],
  );
  const domains = useMemo(
    () => packageDomainOrder.filter((domain) => wrappers.some((wrapper) => wrapper.domain === domain)),
    [wrappers],
  );
  const visibleWrappers = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return wrappers.filter((wrapper) => {
      if (selectedDomain !== "all" && wrapper.domain !== selectedDomain) {
        return false;
      }
      if (!needle) {
        return true;
      }
      return packageSearchText(wrapper).includes(needle);
    });
  }, [query, selectedDomain, wrappers]);
  const groupedWrappers = useMemo(
    () =>
      domains
        .map((domain) => ({
          domain,
          wrappers: visibleWrappers.filter((wrapper) => wrapper.domain === domain),
        }))
        .filter((group) => group.wrappers.length > 0),
    [domains, visibleWrappers],
  );

  const selectedWrapper =
    route.kind === "wrapper"
      ? wrappers.find((wrapper) => wrapperSlug(wrapper) === route.slug) ?? null
      : null;

  function navigate(nextRoute: CatalogRoute, hash?: string) {
    const nextUrl =
      nextRoute.kind === "wrapper"
        ? wrapperHrefFromSlug(nextRoute.slug)
        : `${rootHref()}${hash ?? ""}`;
    window.history.pushState({}, "", nextUrl);
    setRoute(nextRoute);
  }

  return (
    <div className="min-h-screen">
      <TopNavigation
        domains={domains}
        selectedDomain={selectedDomain}
        selectedWrapper={selectedWrapper}
        onHome={() => navigate({ kind: "home" })}
        onDomain={(domain) => {
          setSelectedDomain(domain);
          navigate({ kind: "home" }, domain === "all" ? undefined : `#${domain}`);
        }}
      />

      <div className="mx-auto max-w-7xl px-4 py-6 sm:px-6 xl:px-8">
        {error ? <ErrorPanel message={error} /> : null}
        {!error && !data ? <LoadingPanel /> : null}
        {!error && data && selectedWrapper ? (
          <WrapperPage
            wrapper={selectedWrapper}
            totalWrappers={wrappers.length}
          />
        ) : null}
        {!error && data && !selectedWrapper ? (
          <CatalogHome
            domains={domains}
            groupedWrappers={groupedWrappers}
            query={query}
            selectedDomain={selectedDomain}
            totalPackages={data.packages.length}
            totalWrappers={wrappers.length}
            visibleWrappers={visibleWrappers}
            onQueryChange={setQuery}
            onSelectDomain={setSelectedDomain}
            onSelectWrapper={(wrapper) => navigate({ kind: "wrapper", slug: wrapperSlug(wrapper) })}
          />
        ) : null}
      </div>
    </div>
  );
}

function TopNavigation({
  domains,
  selectedDomain,
  selectedWrapper,
  onHome,
  onDomain,
}: {
  domains: PackageDomain[];
  selectedDomain: PackageDomain | "all";
  selectedWrapper: WorkspaceArchitecturePackage | null;
  onHome: () => void;
  onDomain: (domain: PackageDomain | "all") => void;
}) {
  return (
    <header className="sticky top-0 z-20 border-b border-zinc-200 bg-white/95 backdrop-blur">
      <div className="mx-auto flex max-w-7xl flex-col gap-3 px-4 py-4 sm:px-6 xl:px-8">
        <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
          <a
            href={rootHref()}
            className="min-w-0 text-left"
            onClick={(event) => {
              event.preventDefault();
              onHome();
            }}
          >
            <div className="text-lg font-semibold tracking-normal text-zinc-950">Rust Packages</div>
            <div className="mt-0.5 text-sm text-zinc-500">
              {selectedWrapper ? serviceLibraryName(selectedWrapper) : "Wrapper catalog"}
            </div>
          </a>
          <nav aria-label="Wrapper categories" className="flex min-w-0 gap-2 overflow-x-auto pb-1">
            <NavButton active={selectedDomain === "all" && !selectedWrapper} href={rootHref()} onClick={() => onDomain("all")}>
              All
            </NavButton>
            {domains.map((domain) => (
              <NavButton
                key={domain}
                active={!selectedWrapper && selectedDomain === domain}
                href={`${rootHref()}#${domain}`}
                onClick={() => onDomain(domain)}
              >
                {packageDomainLabels[domain]}
              </NavButton>
            ))}
          </nav>
        </div>
      </div>
    </header>
  );
}

function NavButton({
  active,
  children,
  href,
  onClick,
}: {
  active: boolean;
  children: string;
  href: string;
  onClick: () => void;
}) {
  return (
    <a
      href={href}
      className={classNames(
        "inline-flex min-h-9 shrink-0 items-center rounded-md border px-3 text-sm font-medium transition",
        active
          ? "border-zinc-950 bg-zinc-950 text-white"
          : "border-zinc-200 bg-white text-zinc-700 hover:border-zinc-300 hover:bg-zinc-50",
      )}
      onClick={(event) => {
        event.preventDefault();
        onClick();
      }}
    >
      {children}
    </a>
  );
}

function CatalogHome({
  domains,
  groupedWrappers,
  query,
  selectedDomain,
  totalPackages,
  totalWrappers,
  visibleWrappers,
  onQueryChange,
  onSelectDomain,
  onSelectWrapper,
}: {
  domains: PackageDomain[];
  groupedWrappers: Array<{ domain: PackageDomain; wrappers: WorkspaceArchitecturePackage[] }>;
  query: string;
  selectedDomain: PackageDomain | "all";
  totalPackages: number;
  totalWrappers: number;
  visibleWrappers: WorkspaceArchitecturePackage[];
  onQueryChange: (query: string) => void;
  onSelectDomain: (domain: PackageDomain | "all") => void;
  onSelectWrapper: (wrapper: WorkspaceArchitecturePackage) => void;
}) {
  return (
    <div className="space-y-5">
      <section className="rounded-md border border-zinc-200 bg-white p-5">
        <div className="grid gap-5 lg:grid-cols-[1fr_360px] lg:items-end">
          <div>
            <div className="text-xs font-semibold uppercase text-zinc-500">Overview</div>
            <h1 className="mt-1 text-3xl font-semibold tracking-normal text-zinc-950">
              Wrapper frontends
            </h1>
            <p className="mt-2 max-w-3xl text-sm leading-6 text-zinc-600">
              Each entry opens the companion React app for that crate inside this overview shell.
            </p>
          </div>
          <div className="grid grid-cols-3 gap-2">
            <MetricTile label="Wrappers" value={String(totalWrappers)} />
            <MetricTile label="Shown" value={String(visibleWrappers.length)} />
            <MetricTile label="Indexed" value={String(totalPackages)} />
          </div>
        </div>
      </section>

      <section className="rounded-md border border-zinc-200 bg-white p-4">
        <div className="grid gap-3 lg:grid-cols-[1fr_auto] lg:items-center">
          <label className="block">
            <span className="mb-1 block text-xs font-semibold uppercase text-zinc-500">Search wrappers</span>
            <input
              className="min-h-10 w-full rounded-md border border-zinc-300 px-3 text-sm outline-none focus:border-zinc-950 focus:ring-2 focus:ring-zinc-950/10"
              value={query}
              placeholder="crate, wrapper, package path"
              onChange={(event) => onQueryChange(event.target.value)}
            />
          </label>
          <label className="block lg:w-60">
            <span className="mb-1 block text-xs font-semibold uppercase text-zinc-500">Category</span>
            <select
              className="min-h-10 w-full rounded-md border border-zinc-300 bg-white px-3 text-sm outline-none focus:border-zinc-950 focus:ring-2 focus:ring-zinc-950/10"
              value={selectedDomain}
              onChange={(event) => onSelectDomain(event.target.value as PackageDomain | "all")}
            >
              <option value="all">All categories</option>
              {domains.map((domain) => (
                <option key={domain} value={domain}>
                  {packageDomainLabels[domain]}
                </option>
              ))}
            </select>
          </label>
        </div>
      </section>

      {groupedWrappers.length > 0 ? (
        groupedWrappers.map((group) => (
          <WrapperGroup
            key={group.domain}
            domain={group.domain}
            wrappers={group.wrappers}
            onSelectWrapper={onSelectWrapper}
          />
        ))
      ) : (
        <section className="rounded-md border border-dashed border-zinc-300 bg-white p-6 text-sm text-zinc-500">
          No wrappers match the current filters.
        </section>
      )}
    </div>
  );
}

function WrapperGroup({
  domain,
  wrappers,
  onSelectWrapper,
}: {
  domain: PackageDomain;
  wrappers: WorkspaceArchitecturePackage[];
  onSelectWrapper: (wrapper: WorkspaceArchitecturePackage) => void;
}) {
  return (
    <section id={domain} className="scroll-mt-28 rounded-md border border-zinc-200 bg-white">
      <div className="flex items-center justify-between gap-4 border-b border-zinc-200 px-4 py-3">
        <div>
          <h2 className="text-base font-semibold text-zinc-950">{packageDomainLabels[domain]}</h2>
          <p className="mt-0.5 text-xs text-zinc-500">{wrappers.length} wrappers</p>
        </div>
      </div>
      <div className="grid gap-3 p-4 md:grid-cols-2 xl:grid-cols-3">
        {wrappers.map((wrapper) => (
          <WrapperCard key={wrapper.name} wrapper={wrapper} onSelect={() => onSelectWrapper(wrapper)} />
        ))}
      </div>
    </section>
  );
}

function WrapperCard({ wrapper, onSelect }: { wrapper: WorkspaceArchitecturePackage; onSelect: () => void }) {
  const library = serviceLibraryName(wrapper);

  return (
    <a
      href={wrapperHref(wrapper)}
      className={classNames(
        "block min-w-0 rounded-md border p-4 transition hover:-translate-y-0.5 hover:shadow-sm",
        domainBorderClass(wrapper.domain),
        domainPanelClass(wrapper.domain),
      )}
      onClick={(event) => {
        event.preventDefault();
        onSelect();
      }}
    >
      <div className="text-[11px] font-semibold uppercase text-zinc-500">{packageDomainLabels[wrapper.domain]}</div>
      <div className="mt-1 break-words text-base font-semibold text-zinc-950">{library}</div>
      <div className="mt-2 truncate text-xs text-zinc-600">{wrapper.name}</div>
      <div className="mt-1 truncate text-xs text-zinc-500">{library}-app</div>
    </a>
  );
}

function WrapperPage({
  wrapper,
  totalWrappers,
}: {
  wrapper: WorkspaceArchitecturePackage;
  totalWrappers: number;
}) {
  const library = serviceLibraryName(wrapper);

  return (
    <div className="space-y-5">
      <section className={classNames("rounded-md border p-5", domainBorderClass(wrapper.domain), domainPanelClass(wrapper.domain))}>
        <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
          <div className="min-w-0">
            <div className="text-xs font-semibold uppercase text-zinc-500">{packageDomainLabels[wrapper.domain]}</div>
            <h1 className="mt-1 break-words text-3xl font-semibold tracking-normal text-zinc-950">{library}</h1>
            <p className="mt-2 max-w-3xl text-sm leading-6 text-zinc-700">
              {wrapper.role || wrapper.description || `${library} wrapper frontend`}
            </p>
          </div>
          <div className="grid shrink-0 grid-cols-3 gap-2 text-center">
            <MetricTile label="Wrapper" value="React" />
            <MetricTile label="Category" value={packageDomainLabels[wrapper.domain]} />
            <MetricTile label="Total" value={String(totalWrappers)} />
          </div>
        </div>
      </section>

      <section className="overflow-hidden rounded-md border border-zinc-200 bg-white">
        <div className="border-b border-zinc-200 px-4 py-3">
          <h2 className="text-sm font-semibold text-zinc-950">Frontend</h2>
        </div>
        <WrapperAppMount wrapper={wrapper} />
      </section>
    </div>
  );
}

function WrapperAppMount({ wrapper }: { wrapper: WorkspaceArchitecturePackage }) {
  const [state, setState] = useState<WrapperAppState>({ kind: "loading" });
  const library = serviceLibraryName(wrapper);

  useEffect(() => {
    const modulePath = wrapperAppModulePath(library);
    const loadModule = wrapperAppModules[modulePath];
    let cancelled = false;

    if (!loadModule) {
      setState({ kind: "fallback" });
      return;
    }

    setState({ kind: "loading" });
    loadModule()
      .then((module) => {
        if (!cancelled) {
          setState({ kind: "ready", App: module.App });
        }
      })
      .catch((caught) => {
        if (!cancelled) {
          setState({ kind: "error", message: caught instanceof Error ? caught.message : String(caught) });
        }
      });

    return () => {
      cancelled = true;
    };
  }, [library]);

  if (state.kind === "loading") {
    return <div className="p-5 text-sm text-zinc-500">Loading frontend...</div>;
  }
  if (state.kind === "fallback") {
    return <WrapperMetadataApp wrapper={wrapper} library={library} />;
  }
  if (state.kind === "error") {
    return <div className="p-5 text-sm text-rose-700">{state.message}</div>;
  }

  const App = state.App;
  return (
    <div className="wrapper-app-shell">
      <App />
    </div>
  );
}

function WrapperMetadataApp({
  wrapper,
  library,
}: {
  wrapper: WorkspaceArchitecturePackage;
  library: string;
}) {
  return (
    <main className="min-h-screen bg-zinc-50 text-zinc-950">
      <section className="border-b border-zinc-200 bg-white">
        <div className="mx-auto flex max-w-6xl flex-col gap-4 px-5 py-5 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <p className="text-xs font-semibold uppercase tracking-wide text-teal-700">Package app</p>
            <h1 className="mt-1 text-2xl font-semibold">{titleFromPackageName(library)}</h1>
            <p className="mt-2 max-w-3xl text-sm leading-6 text-zinc-600">
              {wrapper.description || wrapper.role || `${library} package surface`}
            </p>
          </div>
          <span className="status-pill status-pending">Indexed</span>
        </div>
      </section>

      <section className="mx-auto grid max-w-6xl gap-5 px-5 py-6 lg:grid-cols-[minmax(0,1fr)_360px]">
        <section className="panel">
          <div>
            <h2 className="section-title">Package surface</h2>
            <p className="section-copy">Workspace metadata for {wrapper.name}.</p>
          </div>
          {wrapper.exposes.length > 0 ? (
            <ul className="mt-4 grid gap-2 text-sm text-zinc-700">
              {wrapper.exposes.map((item) => (
                <li key={item} className="rounded-md border border-zinc-200 bg-white px-3 py-2">
                  {item}
                </li>
              ))}
            </ul>
          ) : (
            <p className="mt-4 text-sm text-zinc-500">{wrapper.role || "No exposed surfaces are indexed yet."}</p>
          )}
        </section>

        <aside className="space-y-5">
          <section className="panel">
            <h2 className="section-title">Package</h2>
            <dl className="detail-list">
              <div>
                <dt>Library</dt>
                <dd>{library}</dd>
              </div>
              <div>
                <dt>Server</dt>
                <dd>{wrapper.name}</dd>
              </div>
              <div>
                <dt>App</dt>
                <dd>{library}-app</dd>
              </div>
              <div>
                <dt>Path</dt>
                <dd>{wrapper.path ?? "Workspace contract"}</dd>
              </div>
            </dl>
          </section>

          {wrapper.capabilities.length > 0 ? (
            <section className="panel">
              <h2 className="section-title">Capabilities</h2>
              <ul className="endpoint-list">
                {wrapper.capabilities.map((capability) => (
                  <li key={`${capability.kind}:${capability.entrypoint}`}>
                    {capability.kind}: {capability.entrypoint}
                  </li>
                ))}
              </ul>
            </section>
          ) : null}
        </aside>
      </section>
    </main>
  );
}

function LoadingPanel() {
  return (
    <section className="rounded-md border border-zinc-200 bg-white p-5">
      <div className="text-sm font-medium text-zinc-700">Loading wrappers...</div>
    </section>
  );
}

function ErrorPanel({ message }: { message: string }) {
  return (
    <section className="rounded-md border border-rose-200 bg-rose-50 p-5 text-sm text-rose-700">
      {message}
    </section>
  );
}

function MetricTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0 rounded-md border border-white/70 bg-white/80 px-3 py-2">
      <div className="text-[11px] uppercase text-zinc-500">{label}</div>
      <div className="mt-1 truncate text-lg font-semibold text-zinc-950">{value}</div>
    </div>
  );
}

function catalogRouteFromLocation(): CatalogRoute {
  const base = new URL(import.meta.env.BASE_URL || "/", window.location.origin).pathname;
  const pathname = window.location.pathname.startsWith(base)
    ? window.location.pathname.slice(base.length)
    : window.location.pathname.replace(/^\//, "");
  const wrapperMatch = pathname.match(/^(?:wrappers|services)\/([^/]+)/);
  if (wrapperMatch) {
    return { kind: "wrapper", slug: decodeURIComponent(wrapperMatch[1]) };
  }
  return { kind: "home" };
}

function isServerPackage(pkg: WorkspaceArchitecturePackage): boolean {
  return pkg.kind === "rust" && pkg.name.endsWith("-server");
}

function wrapperAppModulePath(library: string): keyof typeof wrapperAppModules {
  return `../../../../packages/${library}-app/src/App.tsx`;
}

function serviceLibraryName(wrapper: WorkspaceArchitecturePackage): string {
  return wrapper.name.replace(/-server$/, "");
}

function titleFromPackageName(name: string): string {
  return name
    .split("-")
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function wrapperSlug(wrapper: WorkspaceArchitecturePackage): string {
  return slugifyPackageName(serviceLibraryName(wrapper));
}

function wrapperHref(wrapper: WorkspaceArchitecturePackage): string {
  return wrapperHrefFromSlug(wrapperSlug(wrapper));
}

function wrapperHrefFromSlug(slug: string): string {
  const base = rootHref();
  return `${base}wrappers/${slug}/`;
}

function rootHref(): string {
  const base = import.meta.env.BASE_URL || "/";
  return base.endsWith("/") ? base : `${base}/`;
}

function comparePackages(a: WorkspaceArchitecturePackage, b: WorkspaceArchitecturePackage): number {
  const domainOrder = packageDomainOrder.indexOf(a.domain) - packageDomainOrder.indexOf(b.domain);
  return domainOrder === 0 ? serviceLibraryName(a).localeCompare(serviceLibraryName(b)) : domainOrder;
}

function packageSearchText(wrapper: WorkspaceArchitecturePackage): string {
  const library = serviceLibraryName(wrapper);
  return [
    wrapper.name,
    library,
    `${library}-app`,
    wrapper.path ?? "",
    wrapper.description,
    wrapper.role,
    packageDomainLabels[wrapper.domain],
    ...wrapper.exposes,
    ...wrapper.consumedBy,
    ...wrapper.tags,
  ]
    .join(" ")
    .toLowerCase();
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
