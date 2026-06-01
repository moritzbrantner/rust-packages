import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import { createTextResultTabs, ModelSelector, PackageSurfaceWorkbench, ResultViewer } from "./index";
import type { PackageAppConfig, SurfaceResponse } from "./types";

const operationResponse: SurfaceResponse = {
  operation: "demo.run",
  value: {
    ok: true,
    count: 1,
    title: "Demo result",
    message: "Demo operation completed.",
    summary: { count: 1 },
  },
  diagnostics: [{ code: "demo", message: "diagnostic" }],
  artifacts: [{ id: "artifact-1" }],
};

function config(overrides: Partial<PackageAppConfig> = {}): PackageAppConfig {
  return {
    library: "demo-package",
    title: "Demo Package",
    description: "Demo package workbench.",
    domain: "text",
    wasm: {
      init: vi.fn(async () => undefined),
      packageSurface: vi.fn(() => ({
        library: "demo-package",
        version: "0.1.0",
        capabilities: {},
        operations: [
          {
            id: "demo.run",
            name: "Run demo",
            description: "Runs the demo operation.",
            inputSchema: {},
            outputSchema: {},
            exampleRequest: { text: "hello" },
            wasmSupported: true,
            serverSupported: true,
          },
        ],
      })),
      runOperation: vi.fn(async () => operationResponse),
    },
    server: {
      scopedRoute: "/api/rust/packages/demo-package",
      standaloneRoute: "",
    },
    ...overrides,
  };
}

beforeEach(() => {
  localStorage.clear();
  vi.restoreAllMocks();
  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/health")) {
        return jsonResponse({ ok: true, package: "demo-package-server", library: "demo-package" });
      }
      if (url.endsWith("/api/package")) {
        return jsonResponse({
          library: "demo-package",
          version: "0.1.0",
          operations: [
            {
              id: "demo.run",
              name: "Run demo",
              description: "Runs the demo operation.",
              exampleRequest: { text: "server", includeNearDuplicates: true },
              wasmSupported: true,
              serverSupported: true,
            },
          ],
        });
      }
      if (url.endsWith("/api/models")) {
        return jsonResponse([
          {
            id: "large-model",
            label: "Large model",
            task: "demo",
            runtime: "onnx",
            supported: false,
            fallback: "small-model",
            note: "Requires optional runtime.",
          },
        ]);
      }
      if (url.endsWith("/api/run")) {
        return jsonResponse(operationResponse);
      }
      if (url.endsWith(".webm") || url.endsWith(".mp4")) {
        const type = url.endsWith(".mp4") ? "video/mp4" : "video/webm";
        return new Response(new Blob(["sample video"], { type }), {
          status: 200,
          headers: { "content-type": type },
        });
      }
      return new Response("not found", { status: 404 });
    }),
  );
});

afterEach(() => cleanup());

describe("PackageSurfaceWorkbench", () => {
  test("loads operations and model fallback metadata", async () => {
    render(<PackageSurfaceWorkbench config={config()} />);

    expect(await screen.findByRole("heading", { name: "Demo Package" })).toBeTruthy();
    expect(await screen.findByRole("combobox", { name: "Operation" })).toBeTruthy();
    expect(await screen.findByText("Large model", { exact: false })).toBeTruthy();
    expect((await screen.findAllByText("Fallback")).length).toBeGreaterThan(0);
  });

  test("edits operation input through form fields", async () => {
    const runOperation = vi.fn(async () => operationResponse);
    render(
      <PackageSurfaceWorkbench
        config={config({
          wasm: {
            init: vi.fn(async () => undefined),
            packageSurface: vi.fn(() => ({
              library: "demo-package",
              version: "0.1.0",
              capabilities: {},
              operations: [
                {
                  id: "demo.run",
                  name: "Run demo",
                  description: "Runs the demo operation.",
                  inputSchema: {},
                  outputSchema: {},
                  exampleRequest: { text: "hello", includeNearDuplicates: true },
                  wasmSupported: true,
                  serverSupported: true,
                },
              ],
            })),
            runOperation,
          },
        })}
      />,
    );

    const textInput = await screen.findByDisplayValue(/hello|server/);
    fireEvent.change(textInput, { target: { value: "updated text" } });
    const toggle = screen.getByRole("switch", { name: "Include Near Duplicates" });
    expect(toggle.getAttribute("aria-checked")).toBe("true");
    fireEvent.click(toggle);
    expect(toggle.getAttribute("aria-checked")).toBe("false");
    fireEvent.click(screen.getByRole("button", { name: "Run" }));

    await waitFor(() => {
      expect(runOperation).toHaveBeenCalledWith({
        operation: "demo.run",
        input: { text: "updated text", includeNearDuplicates: false },
      });
    });
  });

  test("runs the selected operation", async () => {
    render(<PackageSurfaceWorkbench config={config()} />);

    await screen.findByRole("combobox", { name: "Operation" });
    fireEvent.click(screen.getByRole("button", { name: "Run" }));

    await waitFor(() => expect(screen.getByText("Demo result")).toBeTruthy());
    expect(screen.getByText("Demo operation completed.")).toBeTruthy();
    expect(screen.getAllByText("Count").length).toBeGreaterThan(0);
    expect(screen.getByText("1 diagnostics")).toBeTruthy();
  });

  test("groups operations under category tabs", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
        const url = String(input);
        if (url.endsWith("/health")) {
          return jsonResponse({ ok: true, package: "demo-package-server", library: "demo-package" });
        }
        if (url.endsWith("/api/package")) {
          return jsonResponse({
            library: "demo-package",
            version: "0.1.0",
            operations: [
              {
                id: "demo.run",
                name: "Run demo",
                description: "Runs the main workflow.",
                exampleRequest: { mode: "run" },
                wasmSupported: true,
                serverSupported: true,
              },
              {
                id: "demo.inspect",
                name: "Inspect JSON",
                description: "Inspects advanced JSON.",
                exampleRequest: { mode: "inspect" },
                wasmSupported: true,
                serverSupported: true,
              },
            ],
          });
        }
        if (url.endsWith("/api/models")) {
          return jsonResponse([]);
        }
        return new Response("not found", { status: 404 });
      }),
    );

    render(
      <PackageSurfaceWorkbench
        config={config({
          wasm: undefined,
          defaultOperation: "demo.run",
          operationGroups: [
            {
              id: "workflow",
              label: "Workflow",
              operations: ["demo.run"],
            },
            {
              id: "advanced",
              label: "Debug",
              operations: ["demo.inspect"],
            },
          ],
        })}
      />,
    );

    expect(await screen.findByRole("tab", { name: "Workflow" })).toBeTruthy();
    expect(screen.getByRole("tab", { name: "Debug" })).toBeTruthy();
    expect(screen.getByRole("option", { name: "Run demo" })).toBeTruthy();
    expect(screen.queryByRole("option", { name: "Inspect JSON" })).toBeNull();

    fireEvent.click(screen.getByRole("tab", { name: "Debug" }));

    expect(await screen.findByRole("option", { name: "Inspect JSON" })).toBeTruthy();
    expect(screen.queryByRole("option", { name: "Run demo" })).toBeNull();
    expect((await screen.findByDisplayValue(/inspect/)) as HTMLTextAreaElement).toBeTruthy();
  });

  test("falls back to overview server when WASM initialization fails", async () => {
    const runOperation = vi.fn(async () => operationResponse);
    const packageConfig = config({
      wasm: {
        init: vi.fn(async () => {
          throw new Error("missing generated wasm");
        }),
        packageSurface: vi.fn(),
        runOperation,
      },
    });

    render(<PackageSurfaceWorkbench config={packageConfig} />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Overview Server" }).className).toContain("bg-zinc-950");
    });
    fireEvent.click(screen.getByRole("button", { name: "Run" }));

    await waitFor(() => expect(screen.getByText("Demo result")).toBeTruthy());
    expect(runOperation).not.toHaveBeenCalled();
    expect(fetch).toHaveBeenCalledWith(
      "http://127.0.0.1:3000/api/rust/packages/demo-package/api/run",
      expect.objectContaining({ method: "POST" }),
    );
  });

  test("defaults to overview server when configured", async () => {
    render(<PackageSurfaceWorkbench config={config({ defaultRuntime: "overview-server" })} />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Overview Server" }).className).toContain("bg-zinc-950");
    });
  });

  test("moves server-only operations away from client WASM", async () => {
    render(
      <PackageSurfaceWorkbench
        config={config({
          defaultOperation: "demo.serverOnly",
          wasm: {
            init: vi.fn(async () => undefined),
            packageSurface: vi.fn(() => ({
              library: "demo-package",
              version: "0.1.0",
              capabilities: {},
              operations: [
                {
                  id: "demo.serverOnly",
                  name: "Server only",
                  description: "Runs on the server.",
                  inputSchema: {},
                  outputSchema: {},
                  exampleRequest: { text: "server" },
                  wasmSupported: false,
                  serverSupported: true,
                },
              ],
            })),
            runOperation: vi.fn(async () => operationResponse),
          },
        })}
      />,
    );

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Overview Server" }).className).toContain("bg-zinc-950");
    });
    expect((screen.getByRole("button", { name: "Client WASM" }) as HTMLButtonElement).disabled).toBe(true);
  });

  test("loads bundled video samples into the request form", async () => {
    render(<PackageSurfaceWorkbench config={config({ domain: "video" })} />);

    await screen.findByDisplayValue(/hello|server/);
    expect(await screen.findByRole("button", { name: "Test Pattern" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Color Bars" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Moving Box" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "COLMAP Test Video" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Test Pattern" }));

    const editor = (await screen.findByDisplayValue(/data:video\/webm/)) as HTMLTextAreaElement;
    expect(editor.value).toContain("data:video/webm");
  });

  test("loads COLMAP sample patches and preview data into the request form", async () => {
    render(<PackageSurfaceWorkbench config={config({ domain: "video" })} />);

    await screen.findByDisplayValue(/hello|server/);
    fireEvent.click(screen.getByRole("button", { name: "COLMAP Test Video" }));

    await waitFor(() => {
      const editors = screen.getAllByDisplayValue(/test-video\.mp4/) as HTMLTextAreaElement[];
      expect(editors.some((editor) => editor.value.includes("prototypes/web/video-analysis-web/public/samples/video/test-video.mp4"))).toBe(true);
    });
    expect(
      (screen.getAllByDisplayValue(/\/samples\/video\/test-video\.mp4/) as HTMLTextAreaElement[]).some((editor) =>
        editor.value.includes("/samples/video/test-video.mp4"),
      ),
    ).toBe(true);
    expect((screen.getByDisplayValue(/\.external-test-tools\/colmap-runs\/test-video/) as HTMLTextAreaElement).value).toContain(
      ".external-test-tools/colmap-runs/test-video",
    );
    expect((screen.getByDisplayValue(/data:video\/mp4/) as HTMLTextAreaElement).value).toContain("data:video/mp4");
  });

  test("shows setup guidance when the optional COLMAP sample is missing", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
        const url = String(input);
        if (url.endsWith("/health")) {
          return jsonResponse({ ok: true, package: "demo-package-server", library: "demo-package" });
        }
        if (url.endsWith("/api/package")) {
          return jsonResponse({
            library: "demo-package",
            version: "0.1.0",
            operations: [
              {
                id: "demo.run",
                name: "Run demo",
                description: "Runs the demo operation.",
                exampleRequest: { text: "server" },
                wasmSupported: true,
                serverSupported: true,
              },
            ],
          });
        }
        if (url.endsWith("/api/models")) {
          return jsonResponse([]);
        }
        if (url.endsWith(".mp4")) {
          return new Response("missing", { status: 404 });
        }
        return new Response("not found", { status: 404 });
      }),
    );
    render(<PackageSurfaceWorkbench config={config({ domain: "video" })} />);

    await screen.findByDisplayValue(/hello|server/);
    fireEvent.click(screen.getByRole("button", { name: "COLMAP Test Video" }));

    expect(await screen.findByText(/bun run setup:colmap-video/)).toBeTruthy();
    await waitFor(() => {
      const editors = screen.getAllByDisplayValue(/test-video\.mp4/) as HTMLTextAreaElement[];
      expect(editors.some((editor) => editor.value.includes("test-video.mp4"))).toBe(true);
    });
  });

  test("disables Run when no runtime is available", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response("unavailable", { status: 503 })),
    );
    render(
      <PackageSurfaceWorkbench
        config={config({
          wasm: {
            init: vi.fn(async () => {
              throw new Error("missing generated wasm");
            }),
            packageSurface: vi.fn(),
            runOperation: vi.fn(),
          },
        })}
      />,
    );

    expect(await screen.findByText("No runnable runtime is available for this package.")).toBeTruthy();
    expect((screen.getByRole("button", { name: "Run" }) as HTMLButtonElement).disabled).toBe(true);
  });
});

describe("createTextResultTabs", () => {
  const tabs = createTextResultTabs({
    library: "text-demo",
    primaryOperations: {
      "text.demo": {
        title: "Demo text operation",
        summaryFields: ["count", "score"],
        listFields: ["predictions", "keywords", "results", "segments", "tokens"],
        objectFields: ["model", "metadata"],
        explanation: () => "The text operation scored the sample input and exposed focused result sections.",
      },
    },
  });

  test("renders title, message, and scalar summary cards", () => {
    render(
      <ResultViewer
        response={{
          operation: "text.demo",
          value: {
            title: "Configured result",
            message: "Completed the text run.",
            summary: { count: 3, score: 0.75 },
          },
          diagnostics: [],
          artifacts: [],
        }}
        resultTabs={tabs}
      />,
    );

    expect(screen.getByText("Configured result")).toBeTruthy();
    expect(screen.getByText("Completed the text run.")).toBeTruthy();
    expect(screen.getByText("Count")).toBeTruthy();
    expect(screen.getByText("0.750")).toBeTruthy();
    expect(screen.getByText("The text operation scored the sample input and exposed focused result sections.")).toBeTruthy();
  });

  test("renders configured list fields and keeps the raw JSON tab available", () => {
    render(
      <ResultViewer
        response={{
          operation: "text.demo",
          value: {
            operation: "text.demo",
            title: "Lists",
            message: "Lists returned.",
            summary: { count: 2 },
            predictions: [{ label: "positive", score: 0.9 }],
            keywords: [{ term: "rust", score: 0.8 }],
            results: [{ id: "doc-1", score: 0.7 }],
            segments: [{ text: "Hello", startSeconds: 1 }],
            tokens: [{ text: "Hello" }],
          },
          diagnostics: [],
          artifacts: [],
        }}
        resultTabs={tabs}
      />,
    );

    expect(screen.getByRole("heading", { name: "Predictions" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Keywords" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Results" })).toBeTruthy();
    expect(screen.getByText("positive")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: /JSON/ }));

    expect(screen.getByText(/"operation": "text.demo"/)).toBeTruthy();
  });

  test("handles missing configured fields without crashing", () => {
    render(
      <ResultViewer
        response={{
          operation: "text.demo",
          value: {
            title: "Sparse",
            message: "Sparse response.",
            summary: {},
          },
          diagnostics: [],
          artifacts: [],
        }}
        resultTabs={tabs}
      />,
    );

    expect(screen.getByText("Sparse")).toBeTruthy();
    expect(screen.getByText("Sparse response.")).toBeTruthy();
  });
});

describe("ModelSelector", () => {
  test("displays reference-only fallback messaging and metadata", () => {
    render(
      <ModelSelector
        models={[
          {
            id: "reference-model",
            label: "Reference model",
            task: "classification",
            runtime: "onnx",
            supported: false,
            loadable: false,
            fallback: "lexical_fallback",
            requiredFeature: "onnx",
            requiredSetup: "Download model weights",
            smokeOperation: "classification.classify",
            source: "overview-server",
          },
        ]}
        selectedModel="reference-model"
        onSelectModel={vi.fn()}
      />,
    );

    expect(screen.getByText("Catalog metadata only; this page will use the fallback or deterministic operation.")).toBeTruthy();
    expect(screen.getByText("overview-server")).toBeTruthy();
    expect(screen.getByText("lexical_fallback")).toBeTruthy();
    expect(screen.getByText("Download model weights")).toBeTruthy();
  });
});

function jsonResponse(value: unknown): Response {
  return new Response(JSON.stringify(value), {
    status: 200,
    headers: { "content-type": "application/json" },
  });
}
