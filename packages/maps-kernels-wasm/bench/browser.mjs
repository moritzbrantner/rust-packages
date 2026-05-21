import { chromium } from "@playwright/test";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const hostname = "127.0.0.1";

const contentTypes = new Map([
  [".html", "text/html; charset=utf-8"],
  [".js", "text/javascript; charset=utf-8"],
  [".wasm", "application/wasm"],
  [".json", "application/json; charset=utf-8"],
]);

function responseForPath(requestPath) {
  const relativePath = requestPath === "/" ? "bench/browser.html" : decodeURIComponent(requestPath.slice(1));
  const filePath = path.resolve(packageRoot, relativePath);

  if (!filePath.startsWith(`${packageRoot}${path.sep}`) || !existsSync(filePath)) {
    return new Response("Not found", { status: 404 });
  }

  const extension = path.extname(filePath);
  return new Response(Bun.file(filePath), {
    headers: {
      "content-type": contentTypes.get(extension) ?? "application/octet-stream",
    },
  });
}

function printResults(result) {
  console.log(`Browser: ${result.userAgent}`);
  console.log(`WASM init: ${result.initMs.toFixed(3)} ms`);
  console.log("");
  console.log("benchmark,iterations,total_ms,average_ms,output_count");
  for (const entry of result.results) {
    console.log(
      [
        entry.name,
        entry.iterations,
        entry.totalMs.toFixed(3),
        entry.averageMs.toFixed(3),
        entry.outputCount,
      ].join(","),
    );
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
  browser = await chromium.launch(resolveLaunchOptions());
  const page = await browser.newPage();
  page.setDefaultTimeout(30_000);
  page.on("pageerror", (error) => {
    console.error(error);
  });
  await page.goto(`http://${hostname}:${server.port}/bench/browser.html`);
  await page.waitForFunction(() => typeof globalThis.runMapsKernelsWasmBench === "function");
  const result = await page.evaluate(() => globalThis.runMapsKernelsWasmBench());
  printResults(result);
} finally {
  await browser?.close();
  server.stop(true);
}

function resolveLaunchOptions() {
  const executablePath =
    process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH ??
    [
      "/usr/bin/google-chrome",
      "/usr/bin/google-chrome-stable",
      "/usr/bin/chromium",
      "/usr/bin/chromium-browser",
    ].find((candidate) => existsSync(candidate));

  return executablePath ? { executablePath } : {};
}
