import { spawn, type ChildProcess } from "node:child_process";
import { createServer, type AddressInfo } from "node:net";
import { fileURLToPath } from "node:url";

const workspaceRoot = fileURLToPath(new URL("../../../..", import.meta.url));
const projectRoot = fileURLToPath(new URL("..", import.meta.url));
const rustServerAddr = process.env.OVERVIEW_RUST_SERVER_ADDR ?? (await availableAddress("127.0.0.1", 3000));
const rustServerUrl = `http://${rustServerAddr}`;

const children = new Set<ChildProcess>();
let shuttingDown = false;

const rustServer = start("cargo", [
  "run",
  "--bin",
  "video-analysis-overview-server",
  "--",
  "--addr",
  rustServerAddr,
], {
  cwd: workspaceRoot,
  env: process.env,
});

await waitForRustServer();

const viteServer = start(process.execPath, ["x", "vite", "--host", "0.0.0.0"], {
  cwd: projectRoot,
  env: {
    ...process.env,
    VITE_SERVER_URL: process.env.VITE_SERVER_URL ?? rustServerUrl,
    VITE_RUST_SERVER_URL: rustServerUrl,
  },
});

for (const signal of ["SIGINT", "SIGTERM"] as const) {
  process.on(signal, () => shutdown(signal));
}

await new Promise<never>((resolve, reject) => {
  rustServer.on("exit", (code, signal) => {
    if (!shuttingDown) {
      shutdown("SIGTERM");
      reject(new Error(`Rust overview server exited with ${exitLabel(code, signal)}`));
    }
  });
  viteServer.on("exit", (code, signal) => {
    if (!shuttingDown) {
      shutdown("SIGTERM");
      reject(new Error(`Vite dev server exited with ${exitLabel(code, signal)}`));
    }
  });
});

function start(
  command: string,
  args: string[],
  options: { cwd: string; env: NodeJS.ProcessEnv },
): ChildProcess {
  const child = spawn(command, args, {
    cwd: options.cwd,
    env: options.env,
    stdio: "inherit",
  });
  children.add(child);
  child.on("exit", () => children.delete(child));
  child.on("error", (error) => {
    shutdown("SIGTERM");
    throw error;
  });
  return child;
}

async function waitForRustServer() {
  const startedAt = Date.now();
  while (Date.now() - startedAt < 120_000) {
    if (rustServer.exitCode !== null) {
      throw new Error(`Rust overview server exited with code ${rustServer.exitCode}`);
    }
    try {
      const response = await fetch(`${rustServerUrl}/health`);
      if (response.ok) {
        return;
      }
    } catch {
      await delay(250);
    }
  }
  shutdown("SIGTERM");
  throw new Error(`Rust overview server did not become ready at ${rustServerUrl}`);
}

function shutdown(signal: NodeJS.Signals) {
  shuttingDown = true;
  for (const child of children) {
    child.kill(signal);
  }
}

function delay(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function exitLabel(code: number | null, signal: NodeJS.Signals | null) {
  return signal ?? `code ${code ?? "unknown"}`;
}

async function availableAddress(host: string, preferredPort: number): Promise<string> {
  for (let port = preferredPort; port < preferredPort + 100; port += 1) {
    if (await canBind(host, port)) {
      return `${host}:${port}`;
    }
  }
  throw new Error(`No available port found from ${preferredPort} to ${preferredPort + 99}`);
}

function canBind(host: string, port: number): Promise<boolean> {
  return new Promise((resolve) => {
    const server = createServer();
    server.once("error", () => resolve(false));
    server.listen(port, host, () => {
      const address = server.address() as AddressInfo | null;
      server.close(() => resolve(address?.port === port));
    });
  });
}
