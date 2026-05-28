import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import { PackageSurfaceWorkbench } from "./index";
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
              exampleRequest: { text: "server" },
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

  test("validates JSON before running", async () => {
    render(<PackageSurfaceWorkbench config={config()} />);

    const editor = await screen.findByDisplayValue(/hello|server/);
    fireEvent.change(editor, { target: { value: "{" } });
    fireEvent.click(screen.getByRole("button", { name: "Run" }));

    expect(await screen.findByText(/Parse error/)).toBeTruthy();
  });

  test("runs the selected operation", async () => {
    render(<PackageSurfaceWorkbench config={config()} />);

    await screen.findByRole("combobox", { name: "Operation" });
    fireEvent.click(screen.getByRole("button", { name: "Run" }));

    await waitFor(() => expect(screen.getByText(/\"ok\": true/)).toBeTruthy());
    expect(screen.getByText(/\"title\": \"Demo result\"/)).toBeTruthy();
    expect(screen.getByText(/\"message\": \"Demo operation completed.\"/)).toBeTruthy();
    expect(screen.getByText(/\"summary\":/)).toBeTruthy();
    expect(screen.getByText(/\"diagnostics\": 1/)).toBeTruthy();
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

    await waitFor(() => expect(screen.getByText(/\"ok\": true/)).toBeTruthy());
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

  test("loads bundled video samples into the JSON input", async () => {
    render(<PackageSurfaceWorkbench config={config({ domain: "video" })} />);

    await screen.findByDisplayValue(/hello|server/);
    expect(await screen.findByRole("button", { name: "Test Pattern" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Color Bars" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Moving Box" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "COLMAP Test Video" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Test Pattern" }));

    const editor = (await screen.findByDisplayValue(/videoDataUrl/)) as HTMLTextAreaElement;
    expect(editor.value).toContain("data:video/webm");
  });

  test("loads COLMAP sample patches and preview data into the JSON input", async () => {
    render(<PackageSurfaceWorkbench config={config({ domain: "video" })} />);

    await screen.findByDisplayValue(/hello|server/);
    fireEvent.click(screen.getByRole("button", { name: "COLMAP Test Video" }));

    await waitFor(() => {
      const editor = screen.getByDisplayValue(/videoPath/) as HTMLTextAreaElement;
      expect(editor.value).toContain("prototypes/web/video-analysis-web/public/samples/video/test-video.mp4");
      expect(editor.value).toContain("/samples/video/test-video.mp4");
      expect(editor.value).toContain(".external-test-tools/colmap-runs/test-video");
      expect(editor.value).toContain("data:video/mp4");
    });
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
    const editor = (await screen.findByDisplayValue(/videoPath/)) as HTMLTextAreaElement;
    expect(editor.value).toContain("test-video.mp4");
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

function jsonResponse(value: unknown): Response {
  return new Response(JSON.stringify(value), {
    status: 200,
    headers: { "content-type": "application/json" },
  });
}
