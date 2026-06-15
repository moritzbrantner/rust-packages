import { existsSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { expect, test, type Page } from "@playwright/test";

interface WorkspaceArchitecturePackage {
  name: string;
  kind: "rust" | "frontend";
}

interface WorkspaceArchitectureResponse {
  packages: WorkspaceArchitecturePackage[];
}

const workspaceRoot = fileURLToPath(new URL("../../../..", import.meta.url));
const operationSmokePackages = [
  "animation-core",
  "text-core",
  "dense-data",
  "text-analysis",
  "video-analysis-core",
];
const generatedWasmSmokePackages = ["text-core", "dense-data", "text-analysis"];
const textOperationSmokePackages = [
  "text-analysis",
  "text-core",
  "text-classification",
  "text-embeddings",
  "text-generation",
  "text-generation-linguistics",
  "text-index",
  "text-lexical",
  "text-linguistics",
  "text-model-runtime",
  "text-question-answering",
  "text-retrieval",
  "text-transcripts",
];
const textOperationMatrix: Record<string, string[]> = {
  "text-core": ["describe", "text.statistics", "text.normalize", "text.tokenize", "text.boundaries"],
  "text-lexical": ["describe", "lexical.analyze", "lexical.keywords", "lexical.corpusSearch", "lexical.corpusStats"],
  "text-linguistics": ["describe", "linguistics.analyze", "linguistics.entities", "linguistics.language"],
  "text-embeddings": [
    "describe",
    "embeddings.backends",
    "embeddings.embed",
    "embeddings.similarity",
    "embeddings.semanticSearch",
    "embeddings.relatedTerms",
  ],
  "text-retrieval": ["describe", "retrieval.chunk", "retrieval.search", "retrieval.rerank", "retrieval.snapshotPlan"],
  "text-index": [
    "describe",
    "index.build",
    "index.open",
    "index.addDocuments",
    "index.removeDocuments",
    "index.search",
    "index.inspect",
    "index.snapshotPlan",
  ],
  "text-analysis": ["describe", "analysis.describe", "analysis.document", "analysis.corpus", "analysis.similarity"],
  "text-classification": [
    "describe",
    "classification.models",
    "classification.schema",
    "classification.classify",
    "classification.sentiment",
    "classification.zeroShot",
  ],
  "text-question-answering": ["describe", "qa.models", "qa.answer", "qa.answerWithIndex", "qa.answerWithRetrieval", "qa.answerBatch"],
  "text-generation": [
    "describe",
    "generation.markovPredict",
    "generation.markovGenerate",
    "generation.perplexity",
    "generation.synthesizeTerms",
  ],
  "text-generation-linguistics": [
    "describe",
    "generationLinguistics.analysisTerms",
    "generationLinguistics.synthesizeFromAnalysis",
    "generationLinguistics.trainAnalysis",
  ],
  "text-model-runtime": [
    "describe",
    "runtime.tokenizeSummary",
    "runtime.bundleCheck",
    "runtime.downloadBundle",
    "runtime.onnxQaProbe",
    "runtime.tokenizerProbe",
    "runtime.softmax",
  ],
  "text-transcripts": [
    "describe",
    "transcripts.parse",
    "transcripts.normalize",
    "transcripts.importWhisperX",
    "transcripts.formatSrt",
    "transcripts.formatWebVtt",
    "transcripts.toTextSegments",
  ],
};

test("catalog exposes every server-backed wrapper route with a mounted frontend", async ({ page }) => {
  test.setTimeout(120_000);

  const wrappers = await fetchWrappers(page);
  const appPackages = availablePackageApps();
  const appPackageSet = new Set(appPackages);
  const mountedWrappers = wrappers.filter((wrapper) => appPackageSet.has(wrapper));

  expect(mountedWrappers.length).toBeGreaterThan(0);
  for (const wrapper of textOperationSmokePackages) {
    expect(mountedWrappers).toContain(wrapper);
  }

  for (const wrapper of mountedWrappers) {
    await page.goto(wrapperHref(wrapper));

    await expect(page.getByRole("heading", { name: wrapper }).first()).toBeVisible();
    await expect(page.getByRole("heading", { name: "Frontend" })).toBeVisible();
    await expect(page.getByRole("group", { name: "Runtime mode" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Overview Server" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Standalone Server" })).toBeVisible();

    const runButton = page.getByRole("button", { name: "Run", exact: true });
    await expect(runButton).toBeVisible();
    const disabledReason = page.getByText(
      /No runnable runtime is available|Client WASM is unavailable|Overview Server is unavailable|No operations are available|server-only|not supported by the selected/,
    );
    await expect
      .poll(
        async () => {
          if (await runButton.isEnabled()) {
            return "enabled";
          }
          if ((await disabledReason.count()) > 0 && await disabledReason.first().isVisible()) {
            return "disabled-with-reason";
          }
          return "pending";
        },
        { timeout: 10_000 },
      )
      .not.toBe("pending");
    await expect(page.getByText("Failed to fetch dynamically imported module")).toHaveCount(0);
  }
});

test("text category mounts every audited package frontend", async ({ page }) => {
  test.setTimeout(90_000);

  await page.goto("/categories/text/");

  await expect(page.getByRole("heading", { name: "Text", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Category Frontends" })).toBeVisible();
  for (const wrapper of Object.keys(textOperationMatrix)) {
    await expect(page.getByRole("heading", { name: wrapper }).first()).toBeVisible();
  }
});

for (const wrapper of operationSmokePackages) {
  test(`runs ${wrapper} through the overview server`, async ({ page }) => {
    await page.goto(wrapperHref(wrapper));
    await page.getByRole("button", { name: "Overview Server" }).click();

    await runAndExpectResult(page);
    await expect(page.getByText("Failed to fetch dynamically imported module")).toHaveCount(0);
  });
}

for (const wrapper of generatedWasmSmokePackages) {
  test(`runs ${wrapper} through client WASM`, async ({ page }) => {
    await page.goto(wrapperHref(wrapper));
    await page.getByRole("button", { name: "Client WASM" }).click();

    await runAndExpectResult(page);
    await expect(page.getByText("Failed to fetch dynamically imported module")).toHaveCount(0);
  });
}

for (const wrapper of textOperationSmokePackages) {
  test(`runs ${wrapper} text workflow through overview server and client WASM`, async ({ page }) => {
    test.setTimeout(90_000);

    await page.goto(wrapperHref(wrapper));

    await page.getByRole("button", { name: "Overview Server" }).click();
    await runAndExpectStructuredTextResult(page);

    const clientWasmButton = page.getByRole("button", { name: "Client WASM" });
    if (await clientWasmButton.isEnabled()) {
      await clientWasmButton.click();
      await runAndExpectStructuredTextResult(page);
    }

    if ((await page.getByRole("heading", { name: "Models" }).count()) > 0) {
      await expect(page.getByRole("heading", { name: "Models" })).toBeVisible();
      await expect(
        page.getByText(
          /No model presets|This runtime can be used|Registered and supported|Catalog metadata only|Loadable|Supported|Reference/,
        ).first(),
      ).toBeVisible();
    }
    await expect(page.getByText("Failed to fetch dynamically imported module")).toHaveCount(0);
  });
}

test("runs every audited text operation through supported runtimes", async ({ page }) => {
  test.setTimeout(300_000);

  for (const [wrapper, operations] of Object.entries(textOperationMatrix)) {
    for (const operation of operations) {
      await runTextOperationAndExpectStructuredResult(page, wrapper, operation, "overview-server");
      await runTextOperationAndExpectStructuredResult(page, wrapper, operation, "client-wasm");
    }
  }
});

test("missing generated WASM falls back to the overview server", async ({ page }) => {
  await page.goto(wrapperHref("animation-core"));

  await expect(page.getByRole("button", { name: "Overview Server" })).toHaveClass(/bg-zinc-950/);
  await expect(page.getByRole("button", { name: "Client WASM" })).not.toHaveClass(/bg-zinc-950/);

  await runAndExpectResult(page);
  await expect(page.getByText("Failed to fetch dynamically imported module")).toHaveCount(0);
});

async function fetchWrappers(page: Page): Promise<string[]> {
  const response = await page.request.get("/api/workspace-architecture");
  expect(response.ok()).toBe(true);
  const architecture = (await response.json()) as WorkspaceArchitectureResponse;
  return architecture.packages
    .filter((pkg) => pkg.kind === "rust" && pkg.name.endsWith("-server"))
    .map((pkg) => pkg.name.replace(/^moritzbrantner-/, "").replace(/-server$/, ""))
    .sort((left, right) => left.localeCompare(right));
}

function availablePackageApps(): string[] {
  return readdirSync(`${workspaceRoot}/packages`)
    .filter((name) => name.endsWith("-app") && existsSync(`${workspaceRoot}/packages/${name}/src/App.tsx`))
    .map((name) => name.replace(/-app$/, ""))
    .sort((left, right) => left.localeCompare(right));
}

async function runAndExpectResult(page: Page) {
  await expect(page.getByRole("group", { name: "Runtime mode" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Run", exact: true })).toBeEnabled();
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(page.getByRole("button", { name: /JSON/ })).toBeVisible();
  await page.getByRole("button", { name: /JSON/ }).click();
  await expect(page.locator("pre").first()).toContainText('"operation"');
}

async function runAndExpectStructuredTextResult(page: Page) {
  await expect(page.getByRole("group", { name: "Runtime mode" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Run", exact: true })).toBeEnabled();
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expectStructuredJsonResult(page);
}

async function runTextOperationAndExpectStructuredResult(
  page: Page,
  wrapper: string,
  operation: string,
  runtime: "overview-server" | "client-wasm",
) {
  await page.goto("/");
  await page.goto(`${wrapperHref(wrapper)}?operation=${encodeURIComponent(operation)}&runtime=${runtime}`);
  await expect(page.getByRole("group", { name: "Runtime mode" })).toBeVisible();
  await selectWorkbenchOperation(page, operation);

  const runtimeButton = page.getByRole("button", { name: runtime === "overview-server" ? "Overview Server" : "Client WASM" });
  if (runtime === "client-wasm" && (await runtimeButton.isDisabled())) {
    return;
  }
  await expect(runtimeButton, `${wrapper} ${operation} ${runtime} runtime must be selectable`).toBeEnabled();
  await runtimeButton.click();

  const runButton = page.getByRole("button", { name: "Run", exact: true });
  await expect(runButton, `${wrapper} ${operation} ${runtime} run button`).toBeEnabled();
  await runButton.click();
  const response = await expectStructuredJsonResult(page, operation);
  await expectReadableDiagnostics(page, wrapper, operation, runtime, response);
}

async function expectStructuredJsonResult(page: Page, expectedOperation?: string): Promise<SurfaceResponseJson> {
  await expect(page.getByRole("button", { name: /JSON/ })).toBeVisible();
  await page.getByRole("button", { name: /JSON/ }).click();
  const raw = page.locator("pre").last();
  const parsed = await parseJsonPre<SurfaceResponseJson>(raw);
  if (expectedOperation) {
    expect(parsed.operation).toBe(expectedOperation);
  }
  expect(typeof parsed.operation).toBe("string");
  expect(parsed.value?.operation).toBe(parsed.operation);
  expect(typeof parsed.value?.title).toBe("string");
  expect(typeof parsed.value?.message).toBe("string");
  expect(parsed.value?.summary && typeof parsed.value.summary === "object" && !Array.isArray(parsed.value.summary)).toBe(true);
  expect(Object.hasOwn(parsed.value ?? {}, "result")).toBe(true);
  return parsed;
}

async function selectWorkbenchOperation(page: Page, operation: string) {
  const operationSelect = page.getByRole("combobox", { name: "Operation" });
  if ((await operationSelect.count()) > 0) {
    await operationSelect.selectOption(operation);
    return;
  }

  const scenarioSelect = page.getByRole("combobox", { name: "Scenario" });
  if ((await scenarioSelect.count()) === 0) {
    return;
  }

  const rawScenario = `operation:${operation}`;
  const scenarioValues = await scenarioSelect.locator("option").evaluateAll((options) =>
    options.map((option) => (option as HTMLOptionElement).value),
  );
  if (scenarioValues.includes(rawScenario)) {
    await scenarioSelect.selectOption(rawScenario);
  }
}

async function expectReadableDiagnostics(
  page: Page,
  wrapper: string,
  operation: string,
  runtime: string,
  response: SurfaceResponseJson,
) {
  await expect(page.getByRole("button", { name: /Diagnostics/ })).toBeVisible();
  await page.getByRole("button", { name: /Diagnostics/ }).click();
  const diagnostics = await parseJsonPre<unknown[]>(page.locator("pre").first());
  expect(Array.isArray(diagnostics)).toBe(true);
  expect(diagnostics).toEqual(response.diagnostics);
  const unexpectedDiagnostics = diagnostics.filter((diagnostic) => !isExpectedRuntimeDiagnostic(diagnostic));
  expect(
    unexpectedDiagnostics,
    `${wrapper} ${operation} ${runtime} emitted unexpected diagnostics: ${JSON.stringify(unexpectedDiagnostics, null, 2)}`,
  ).toEqual([]);
}

async function parseJsonPre<T>(locator: ReturnType<Page["locator"]>): Promise<T> {
  return expect
    .poll(
      async () => {
        const text = await locator.textContent();
        if (!text) {
          return null;
        }
        try {
          return JSON.parse(text) as T;
        } catch {
          return null;
        }
      },
      { timeout: 10_000 },
    )
    .not.toBeNull()
    .then(async () => JSON.parse((await locator.textContent()) ?? "null") as T);
}

function isExpectedRuntimeDiagnostic(diagnostic: unknown): boolean {
  const text = typeof diagnostic === "string" ? diagnostic : JSON.stringify(diagnostic);
  return /autoDownload|auto download|bundle|download|fallback|feature|model|native|onnx|candle|unavailable|unsupported/i.test(text);
}

function wrapperHref(wrapper: string): string {
  return `/wrappers/${slugify(wrapper)}/`;
}

function slugify(value: string): string {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

interface SurfaceResponseJson {
  artifacts: unknown[];
  diagnostics: unknown[];
  operation: string;
  value: {
    operation?: unknown;
    title?: unknown;
    message?: unknown;
    summary?: unknown;
    result?: unknown;
  };
}
