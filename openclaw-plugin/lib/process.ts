import { spawn, type ChildProcess } from "node:child_process";
import net from "node:net";

export interface ProxyConfig {
  binaryPath: string;
  apiKey: string;
  port?: number;
  backendUrl?: string;
  debug?: boolean;
}

export interface RunningProxy {
  process: ChildProcess;
  port: number;
  kill: () => void;
}

async function findFreePort(preferred: number): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.listen(preferred, "127.0.0.1", () => {
      const addr = server.address();
      const port = typeof addr === "object" && addr ? addr.port : preferred;
      server.close(() => resolve(port));
    });
    server.on("error", () => {
      // Preferred port busy, let OS pick one
      const fallback = net.createServer();
      fallback.listen(0, "127.0.0.1", () => {
        const addr = fallback.address();
        const port = typeof addr === "object" && addr ? addr.port : 0;
        fallback.close(() => resolve(port));
      });
      fallback.on("error", reject);
    });
  });
}

async function waitForHealth(port: number, timeoutMs: number = 10000): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const res = await fetch(`http://127.0.0.1:${port}/health`);
      if (res.ok) return;
    } catch {
      // Not ready yet
    }
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`maple-proxy did not become healthy within ${timeoutMs}ms`);
}

export async function startProxy(
  config: ProxyConfig,
  logger: { info: (msg: string) => void; error: (msg: string) => void }
): Promise<RunningProxy> {
  const port = await findFreePort(config.port ?? 8080);

  const env: Record<string, string> = {
    ...process.env as Record<string, string>,
    MAPLE_HOST: "127.0.0.1",
    MAPLE_PORT: String(port),
    MAPLE_API_KEY: config.apiKey,
  };

  if (config.backendUrl) {
    env.MAPLE_BACKEND_URL = config.backendUrl;
  }
  if (config.debug) {
    env.MAPLE_DEBUG = "true";
  }

  const child = spawn(config.binaryPath, [], {
    env,
    stdio: ["ignore", "pipe", "pipe"],
    detached: false,
  });

  child.stdout?.on("data", (data: Buffer) => {
    logger.info(`[maple-proxy] ${data.toString().trim()}`);
  });

  child.stderr?.on("data", (data: Buffer) => {
    logger.error(`[maple-proxy] ${data.toString().trim()}`);
  });

  child.on("exit", (code) => {
    if (code !== null && code !== 0) {
      logger.error(`maple-proxy exited with code ${code}`);
    }
  });

  await waitForHealth(port);
  logger.info(`maple-proxy running on http://127.0.0.1:${port}`);

  return {
    process: child,
    port,
    kill: () => {
      if (!child.killed) {
        child.kill("SIGTERM");
      }
    },
  };
}
