import { storyIdsFromIndex } from "./storybook-validation";

const url = readArg("--url") ?? "http://127.0.0.1:6006";

const { chromium } = await import("@playwright/test");
const indexResponse = await fetch(`${url.replace(/\/$/, "")}/index.json`);
if (!indexResponse.ok) {
  throw new Error(`Unable to load Storybook index: ${indexResponse.status}`);
}

const index = await indexResponse.json();
const storyIds = storyIdsFromIndex(index);

const browser = await chromium.launch();
try {
  const page = await browser.newPage();
  page.setDefaultTimeout(20_000);
  for (const storyId of storyIds) {
    let pageError: Error | undefined;
    const recordPageError = (error: Error) => {
      pageError ??= error;
    };
    page.on("pageerror", recordPageError);
    const storyUrl = `${url.replace(/\/$/, "")}/iframe.html?id=${encodeURIComponent(storyId)}&viewMode=story`;
    try {
      await page.goto(storyUrl);
      await page.waitForFunction(() => {
        const root = document.querySelector("#storybook-root");
        return Boolean(root?.childElementCount);
      });
      await page.waitForTimeout(50);
      if (pageError) {
        throw new Error(
          `Storybook story ${storyId} raised a page error: ${pageError.message}`,
        );
      }
    } finally {
      page.off("pageerror", recordPageError);
    }
  }
  console.log(`Validated ${storyIds.length} Storybook stories with Playwright.`);
} finally {
  await browser.close();
}

function readArg(name: string): string | undefined {
  const index = process.argv.indexOf(name);
  if (index === -1) {
    return undefined;
  }
  return process.argv[index + 1];
}
