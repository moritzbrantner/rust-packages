import { existsSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { expect, test, type Page } from "@playwright/test";

import { unprefixedPackageName } from "../src/workspaceArchitecture";

interface WorkspaceArchitecturePackage {
  name: string;
  kind: "rust" | "frontend";
}

interface WorkspaceArchitectureResponse {
  packages: WorkspaceArchitecturePackage[];
}

const workspaceRoot = fileURLToPath(new URL("../../../..", import.meta.url));
const operationSmokePackages = ["animation-core", "comfyui-models", "three-d-scene-svg"];

test("catalog exposes every server-backed wrapper route with a mounted frontend", async ({ page }) => {
  test.setTimeout(120_000);

  const wrappers = await fetchWrappers(page);
  const appPackages = availablePackageApps();
  const appPackageSet = new Set(appPackages);
  const mountedWrappers = wrappers.filter((wrapper) => appPackageSet.has(wrapper));

  expect(mountedWrappers.length).toBeGreaterThan(0);
  for (const wrapper of operationSmokePackages) {
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

for (const wrapper of operationSmokePackages) {
  test(`runs ${wrapper} through the overview server`, async ({ page }) => {
    await page.goto(wrapperHref(wrapper));
    await page.getByRole("button", { name: "Overview Server" }).click();

    await runAndExpectResult(page);
    await expect(page.getByText("Failed to fetch dynamically imported module")).toHaveCount(0);
  });
}

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
    .map((pkg) => unprefixedPackageName(pkg.name).replace(/-server$/, ""))
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
  await expect(page.getByRole("button", { name: /^JSON/ })).toBeVisible();
  await page.getByRole("button", { name: /^JSON/ }).click();
  await expect(page.locator("pre").first()).toContainText('"operation"');
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
