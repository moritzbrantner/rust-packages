import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { type AddressInfo } from "node:net";
import { fileURLToPath } from "node:url";
import { createServer, type ViteDevServer } from "vite";

let server: ViteDevServer;
let baseUrl: string;

beforeAll(async () => {
  server = await createServer({
    configFile: fileURLToPath(new URL("../vite.config.ts", import.meta.url)),
    logLevel: "silent",
    server: {
      host: "127.0.0.1",
      port: 0,
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
  });

  it("reports missing packages as HTTP 404", async () => {
    const response = await fetch(`${baseUrl}/api/packages?name=missing-package`);

    expect(response.status).toBe(404);
    await expect(response.json()).resolves.toMatchObject({
      message: "unknown package `missing-package`",
    });
  });

  it("serves workspace architecture with dependencies and interop pairs", async () => {
    const response = await fetch(`${baseUrl}/api/workspace-architecture`);

    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.packages.some((pkg: { name: string }) => pkg.name === "@video-analysis/ui")).toBe(true);
    expect(body.dependencies.length).toBeGreaterThan(0);
    expect(body.interop.length).toBeGreaterThan(0);
  });
});
