import { expect, test } from "@playwright/test";

test("renders the studio shell and report overview", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByText("Video Analysis Studio")).toBeVisible();
  await expect(page.getByRole("heading", { name: "YouTube Video" })).toBeVisible();
  await expect(page.getByAltText("Scene metrics preview")).toBeVisible();
  await expect(page.getByText("Scenes").first()).toBeVisible();
  await expect(page.getByText("Frames").first()).toBeVisible();
});

test("loads architecture data through the API-backed page", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Architecture" }).click();

  await expect(page.getByRole("heading", { name: "Filters" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Packages" })).toBeVisible();
  await expect(page.getByRole("button", { name: "video-analysis-core", exact: true })).toBeVisible();
});
