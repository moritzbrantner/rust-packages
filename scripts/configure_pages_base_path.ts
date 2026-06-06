const repository = process.env.GITHUB_REPOSITORY;
const token = process.env.GITHUB_TOKEN ?? process.env.GH_PACKAGES_TOKEN ?? process.env.NODE_AUTH_TOKEN;
const apiUrl = process.env.GITHUB_API_URL ?? "https://api.github.com";

if (!repository) {
  throw new Error("GITHUB_REPOSITORY is required");
}

const [owner, repo] = repository.split("/");
let baseUrl = "";

if (token) {
  const response = await fetch(`${apiUrl}/repos/${repository}/pages`, {
    headers: {
      Accept: "application/vnd.github+json",
      Authorization: `Bearer ${token}`,
      "X-GitHub-Api-Version": "2022-11-28",
    },
  });

  if (response.ok) {
    const pages = await response.json();
    baseUrl = String(pages.html_url ?? "").replace(/\/$/, "");
  } else if (response.status !== 404) {
    throw new Error(`GitHub Pages metadata request failed with status ${response.status}`);
  }
}

if (!baseUrl) {
  const isUserOrOrganizationSite = repo.toLowerCase() === `${owner.toLowerCase()}.github.io`;
  baseUrl = `https://${owner}.github.io${isUserOrOrganizationSite ? "" : `/${repo}`}`;
}

const url = new URL(baseUrl);
const basePath = url.pathname.replace(/\/$/, "");
const pagesBasePath = `${basePath}/`.replace(/^\/?/, "/");

if (process.env.GITHUB_OUTPUT) {
  const existingOutput = await Bun.file(process.env.GITHUB_OUTPUT).text().catch(() => "");
  await Bun.write(
    process.env.GITHUB_OUTPUT,
    existingOutput +
      [
        `base_url=${baseUrl}`,
        `origin=${url.origin}`,
        `host=${url.host}`,
        `base_path=${basePath}`,
        "",
      ].join("\n"),
  );
}

if (process.env.GITHUB_ENV) {
  const existingEnv = await Bun.file(process.env.GITHUB_ENV).text().catch(() => "");
  await Bun.write(process.env.GITHUB_ENV, `${existingEnv}PAGES_BASE_PATH=${pagesBasePath}\n`);
}

console.log(`Configured GitHub Pages base path: ${pagesBasePath}`);
