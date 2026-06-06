import path from "node:path";

const root = path.resolve("storybook-static");
const hostname = process.env.STORYBOOK_HOST ?? "127.0.0.1";
const port = Number(process.env.STORYBOOK_PORT ?? "6006");

const contentTypes = new Map([
  [".css", "text/css; charset=utf-8"],
  [".html", "text/html; charset=utf-8"],
  [".js", "text/javascript; charset=utf-8"],
  [".json", "application/json; charset=utf-8"],
  [".map", "application/json; charset=utf-8"],
  [".png", "image/png"],
  [".svg", "image/svg+xml"],
  [".txt", "text/plain; charset=utf-8"],
  [".woff", "font/woff"],
  [".woff2", "font/woff2"],
]);

function responseForPath(requestPath: string): Response {
  const decoded = decodeURIComponent(requestPath === "/" ? "/index.html" : requestPath);
  const filePath = path.resolve(root, decoded.slice(1));

  if (!filePath.startsWith(`${root}${path.sep}`)) {
    return new Response("Not found", { status: 404 });
  }

  const file = Bun.file(filePath);
  return new Response(file, {
    headers: {
      "cache-control": "no-store",
      "content-type": contentTypes.get(path.extname(filePath)) ?? "application/octet-stream",
    },
  });
}

const server = Bun.serve({
  hostname,
  port,
  fetch(request) {
    return responseForPath(new URL(request.url).pathname);
  },
});

console.log(`Serving Storybook static build at http://${server.hostname}:${server.port}`);
