import { expect, test } from "@playwright/test";

test("renders the wrapper catalog home page", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("heading", { name: "Wrapper frontends" })).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Wrapper categories" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Audio" })).toBeVisible();
  await expect(page.getByRole("link", { name: /audio-analysis-core/ })).toBeVisible();
});

test("opens a wrapper route with the package frontend inside the overview shell", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("link", { name: /audio-analysis-core/ }).click();

  await expect(page).toHaveURL(/\/wrappers\/audio-analysis-core\//);
  await expect(page.getByRole("heading", { name: "audio-analysis-core" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Frontend" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Audio Analysis Core" })).toBeVisible();
});

test("opens the direct video category route", async ({ page }) => {
  await page.goto("/video/");

  await expect(page).toHaveURL(/\/video\//);
  await expect(page.getByRole("heading", { name: "Video", exact: true })).toBeVisible();
  await expect(page.getByRole("link", { name: /video-analysis-sfm/ })).toBeVisible();
});

test("loads the colmap backend app on its wrapper URL", async ({ page }) => {
  await page.goto("/wrappers/video-analysis-sfm/");

  await expect(page.getByRole("heading", { name: "video-analysis-sfm" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Frontend" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Video Analysis Colmap Backend" })).toBeVisible();
  await expect(page.getByRole("group", { name: "Runtime mode" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Client WASM" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Overview Server" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Standalone Server" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Test Pattern" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Color Bars" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Moving Box" })).toBeVisible();
  await expect(page.getByRole("button", { name: "COLMAP Test Video" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Test Video", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "COLMAP Run" })).toBeVisible();
  await expect(page.getByRole("button", { name: "3D View" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Support" })).toBeVisible();
});
