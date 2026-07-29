const url = readArg("--url") ?? "http://127.0.0.1:6006";

if (await hasRealNode()) {
  const result = Bun.spawnSync(["node", "node_modules/.bin/test-storybook", "--url", url], {
    stdin: "inherit",
    stdout: "inherit",
    stderr: "inherit",
  });
  process.exit(result.exitCode ?? 1);
}

const { chromium } = await import("@playwright/test");
const indexResponse = await fetch(`${url.replace(/\/$/, "")}/index.json`);
if (!indexResponse.ok) {
  throw new Error(`Unable to load Storybook index: ${indexResponse.status}`);
}

const index = await indexResponse.json();
const stories = Object.values(index.entries ?? {}).filter(
  (entry): entry is { id: string; type: string } =>
    Boolean(entry) &&
    typeof entry === "object" &&
    (entry as { type?: unknown }).type === "story" &&
    typeof (entry as { id?: unknown }).id === "string",
);

if (stories.length === 0) {
  throw new Error("Storybook index did not contain any stories.");
}

const browser = await chromium.launch();
try {
  const page = await browser.newPage();
  page.setDefaultTimeout(20_000);
  for (const story of stories) {
    const storyUrl = `${url.replace(/\/$/, "")}/iframe.html?id=${encodeURIComponent(story.id)}&viewMode=story`;
    await page.goto(storyUrl);
    await page.waitForFunction(() => {
      const root = document.querySelector("#storybook-root");
      return Boolean(root?.childElementCount);
    });
  }
  console.log(`Validated ${stories.length} Storybook stories with Playwright.`);
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

async function hasRealNode(): Promise<boolean> {
  const result = Bun.spawnSync({
    cmd: ["node", "--eval", "process.stdout.write(process.versions.bun ? 'bun' : 'node')"],
    stdout: "pipe",
    stderr: "ignore",
  });
  return result.exitCode === 0 && result.stdout.toString() === "node";
}
