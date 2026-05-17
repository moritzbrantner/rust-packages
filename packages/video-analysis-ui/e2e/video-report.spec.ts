import { expect, test } from "@playwright/test";

test("renders report components in a browser", async ({ page }) => {
  await page.goto("/e2e/");

  await expect(page.getByRole("heading", { name: "Video report fixture" })).toBeVisible();
  await expect(page.getByText("1920x1080")).toBeVisible();
  await expect(page.getByText("240")).toBeVisible();
  await expect(page.getByTitle("Scene 1 00:00:00.000-00:00:04.000")).toBeVisible();
  await expect(page.getByText("person")).toBeVisible();
  await expect(page.getByText("object-command")).toBeVisible();
});
