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

test("loads the colmap backend app on its wrapper URL", async ({ page }) => {
  await page.goto("/wrappers/video-analysis-colmap-backend/");

  await expect(page.getByRole("heading", { name: "video-analysis-colmap-backend" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Frontend" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Video Analysis Colmap Backend" })).toBeVisible();
  await expect(page.getByText("POST payload for video-analysis-colmap-backend-server.")).toBeVisible();
});
