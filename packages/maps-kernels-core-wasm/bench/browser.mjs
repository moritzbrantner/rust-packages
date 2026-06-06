import path from "node:path";
import { fileURLToPath } from "node:url";

const playwright = await import("playwright").catch(() => null);
if (!playwright) {
  console.error("Playwright is not installed. Run `bun install` before `bun run maps-wasm:bench`.");
  process.exit(0);
}
const { chromium } = playwright;

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const hostname = "127.0.0.1";

const contentTypes = new Map([
  [".html", "text/html; charset=utf-8"],
  [".js", "text/javascript; charset=utf-8"],
  [".wasm", "application/wasm"],
  [".json", "application/json; charset=utf-8"],
]);

const scenarios = [
  {
    packageName: "maps-kernels-core",
    operation: "maps.pathSummary",
    input: { coordinates: [0, 0, 3, 4] },
    iterations: 200,
    outputCountPath: ["bounds"],
  },
  {
    packageName: "maps-kernels-core",
    operation: "maps.kernelSummary",
    input: { coordinates: [0, 0, 1, 0, 2, 1, 3, 1], closed: false },
    iterations: 200,
    outputCountPath: ["bbox"],
  },
  {
    packageName: "maps-kernels-core",
    operation: "maps.simplifyLine",
    input: { coordinates: [0, 0, 1, 0.01, 2, 0, 3, 0.5, 4, 0], tolerance: 0.05 },
    iterations: 200,
    outputCountPath: ["coordinates"],
  },
];

function responseForPath(requestPath) {
  const relativePath = requestPath === "/" ? "bench/browser.html" : decodeURIComponent(requestPath.slice(1));
  const filePath = path.resolve(packageRoot, relativePath);

  if (!filePath.startsWith(`${packageRoot}${path.sep}`)) {
    return new Response("Not found", { status: 404 });
  }

  return new Response(Bun.file(filePath), {
    headers: {
      "content-type": contentTypes.get(path.extname(filePath)) ?? "application/octet-stream",
    },
  });
}

function printResults(results) {
  console.log(`Browser: ${results.userAgent}`);
  console.log("package,operation,iterations,total_ms,average_ms,ops_per_second,output_count,status");
  for (const entry of results.entries) {
    console.log(
      [
        entry.packageName,
        entry.operation,
        entry.iterations,
        entry.totalMs?.toFixed?.(3) ?? "",
        entry.averageMs?.toFixed?.(3) ?? "",
        entry.opsPerSecond?.toFixed?.(2) ?? "",
        entry.outputCount ?? "",
        entry.status,
      ].join(","),
    );
    if (entry.error) {
      console.error(`${entry.operation}: ${entry.error}`);
    }
  }
}

const server = Bun.serve({
  hostname,
  port: 0,
  fetch(request) {
    return responseForPath(new URL(request.url).pathname);
  },
});

let browser;
try {
  browser = await chromium.launch();
  const page = await browser.newPage();
  page.setDefaultTimeout(30_000);
  page.on("pageerror", (error) => console.error(error));
  await page.goto(`http://${hostname}:${server.port}/bench/browser.html`);
  await page.waitForFunction(() => typeof globalThis.runMapsWasmBench === "function");
  const results = await page.evaluate(async (inputScenarios) => globalThis.runMapsWasmBench(inputScenarios), scenarios);
  printResults(results);
} finally {
  await browser?.close();
  server.stop(true);
}
