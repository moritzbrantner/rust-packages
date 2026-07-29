import { expect, test } from "bun:test";

import { storyIdsFromIndex } from "./storybook-validation";

test("selects executable stories from the Storybook index", () => {
  expect(
    storyIdsFromIndex({
      entries: {
        "report--default": { id: "report--default", type: "story" },
        "report--docs": { id: "report--docs", type: "docs" },
        malformed: { type: "story" },
      },
    }),
  ).toEqual(["report--default"]);
});

test("rejects a Storybook index without executable stories", () => {
  expect(() => storyIdsFromIndex({ entries: {} })).toThrow(
    "Storybook index did not contain any stories.",
  );
});

test(
  "rejects a rendered story that raises a page error",
  async () => {
    const server = Bun.serve({
      hostname: "127.0.0.1",
      port: 0,
      fetch(request) {
        const pathname = new URL(request.url).pathname;
        if (pathname === "/index.json") {
          return Response.json({
            entries: {
              "synthetic--fault": {
                id: "synthetic--fault",
                type: "story",
              },
            },
          });
        }
        if (pathname === "/iframe.html") {
          return new Response(
            `<!doctype html>
              <div id="storybook-root"></div>
              <script>
                setTimeout(() => {
                  document.querySelector("#storybook-root")
                    .append(document.createElement("div"));
                  throw new Error("synthetic story page error");
                }, 0);
              </script>`,
            { headers: { "content-type": "text/html" } },
          );
        }
        return new Response("not found", { status: 404 });
      },
    });

    try {
      const child = Bun.spawn(
        [
          process.execPath,
          "scripts/run-storybook-test.ts",
          "--url",
          server.url.toString().replace(/\/$/, ""),
        ],
        {
          cwd: new URL("..", import.meta.url).pathname,
          env: process.env,
          stdout: "pipe",
          stderr: "pipe",
        },
      );
      const [exitCode, stdout, stderr] = await Promise.all([
        child.exited,
        new Response(child.stdout).text(),
        new Response(child.stderr).text(),
      ]);
      expect(stdout).not.toContain("Validated 1 Storybook stories");
      expect(exitCode).not.toBe(0);
      expect(stderr).toContain("synthetic story page error");
    } finally {
      server.stop(true);
    }
  },
  30_000,
);
