import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import { PackageSurfaceWorkbench } from "./index";
import type { PackageAppConfig, SurfaceResponse } from "./types";

const operationResponse: SurfaceResponse = {
  operation: "demo.run",
  value: { ok: true, count: 1 },
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
    expect(screen.getByText(/\"diagnostics\": 1/)).toBeTruthy();
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
