import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { createServer as createNetServer, type AddressInfo } from "node:net";
import { fileURLToPath } from "node:url";
import { createServer, type ViteDevServer } from "vite";

let server: ViteDevServer;
let baseUrl: string;
const apiTestTimeoutMs = 60_000;

beforeAll(async () => {
  const port = await availablePort();
  server = await createServer({
    configFile: fileURLToPath(new URL("../vite.config.ts", import.meta.url)),
    logLevel: "silent",
    server: {
      host: "127.0.0.1",
      port,
      strictPort: true,
    },
  });
  await server.listen();

  const address = server.httpServer?.address() as AddressInfo | null;
  if (!address) {
    throw new Error("Vite test server did not expose an HTTP address");
  }
  baseUrl = `http://127.0.0.1:${address.port}`;
}, 120_000);

afterAll(async () => {
  await server?.close();
});

describe("workspace API integration", () => {
  it("serves the package catalog over HTTP", async () => {
    const response = await fetch(`${baseUrl}/api/packages?name=video-analysis-core`);

    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.name).toBe("video-analysis-core");
    expect(body.capabilities.map((capability: { kind: string }) => capability.kind)).toEqual([
      "library",
      "cli",
      "api",
      "ui",
    ]);
  }, apiTestTimeoutMs);

  it("reports adapter packages without fabricated nested companion surfaces", async () => {
    const response = await fetch(`${baseUrl}/api/packages?name=maps-kernels-core-cli`);

    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.capabilities.map((capability: { kind: string }) => capability.kind)).toEqual([
      "library",
      "cli",
    ]);
    expect(
      body.capabilities.some((capability: { entrypoint: string }) =>
        capability.entrypoint.includes("maps-kernels-core-cli-cli"),
      ),
    ).toBe(false);
  }, apiTestTimeoutMs);

  it("reports missing packages as HTTP 404", async () => {
    const response = await fetch(`${baseUrl}/api/packages?name=missing-package`);

    expect(response.status).toBe(404);
    await expect(response.json()).resolves.toMatchObject({
      message: "unknown package `missing-package`",
    });
  }, apiTestTimeoutMs);

  it("serves workspace architecture with dependencies and interop pairs", async () => {
    const response = await fetch(`${baseUrl}/api/workspace-architecture`);

    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.packages.some((pkg: { name: string }) => pkg.name === "@video-analysis/ui")).toBe(true);
    expect(body.dependencies.length).toBeGreaterThan(0);
    expect(body.interop.length).toBeGreaterThan(0);
  }, apiTestTimeoutMs);
});

function availablePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const probe = createNetServer();
    probe.on("error", reject);
    probe.listen(0, "127.0.0.1", () => {
      const address = probe.address() as AddressInfo | null;
      if (!address) {
        probe.close(() => reject(new Error("Could not allocate a test port")));
        return;
      }
      const port = address.port;
      probe.close((error) => {
        if (error) {
          reject(error);
        } else {
          resolve(port);
        }
      });
    });
  });
}
