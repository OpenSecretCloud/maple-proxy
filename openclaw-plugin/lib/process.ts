import { spawn, type ChildProcess } from "node:child_process";
import net from "node:net";

const DEFAULT_PORT = 8000;
const HEALTH_TIMEOUT_MS = 10000;
const MAX_RESTART_ATTEMPTS = 3;
const RESTART_BACKOFF_MS = 2000;

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
  version: string;
  kill: () => void;
}

function checkPortAvailable(port: number): Promise<boolean> {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.listen(port, "127.0.0.1", () => {
      server.close(() => resolve(true));
    });
    server.on("error", (err: NodeJS.ErrnoException) => {
      if (err.code === "EADDRINUSE") {
        resolve(false);
      } else {
        reject(err);
      }
    });
  });
}

async function waitForHealth(port: number): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < HEALTH_TIMEOUT_MS) {
    try {
      const res = await fetch(`http://127.0.0.1:${port}/health`);
      if (res.ok) return;
    } catch {
      // Not ready yet
    }
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`maple-proxy did not become healthy within ${HEALTH_TIMEOUT_MS}ms`);
}

function spawnProxy(
  config: ProxyConfig,
  port: number,
  logger: { info: (msg: string) => void; error: (msg: string) => void }
): ChildProcess {
  const env: Record<string, string> = {
    ...(process.env as Record<string, string>),
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
  });

  child.stdout?.on("data", (data: Buffer) => {
    logger.info(`[maple-proxy] ${data.toString().trim()}`);
  });

  child.stderr?.on("data", (data: Buffer) => {
    logger.error(`[maple-proxy] ${data.toString().trim()}`);
  });

  return child;
}

export async function startProxy(
  config: ProxyConfig,
  version: string,
  logger: { info: (msg: string) => void; error: (msg: string) => void }
): Promise<RunningProxy> {
  const port = config.port ?? DEFAULT_PORT;

  const available = await checkPortAvailable(port);
  if (!available) {
    throw new Error(
      `Port ${port} is already in use. ` +
        `Set a different port in plugins.entries["maple-openclaw-plugin"].config.port`
    );
  }

  let child = spawnProxy(config, port, logger);
  let stopped = false;
  let restartAttempts = 0;

  const setupCrashRecovery = (proc: ChildProcess) => {
    proc.on("exit", (code, signal) => {
      if (stopped) return;
      if (signal === "SIGINT" || signal === "SIGTERM") return;

      if (code !== null && code !== 0) {
        logger.error(`maple-proxy crashed with code ${code}`);

        if (restartAttempts < MAX_RESTART_ATTEMPTS) {
          restartAttempts++;
          const delay = RESTART_BACKOFF_MS * restartAttempts;
          logger.info(
            `Restarting maple-proxy in ${delay}ms (attempt ${restartAttempts}/${MAX_RESTART_ATTEMPTS})...`
          );
          setTimeout(async () => {
            if (stopped) return;
            try {
              child = spawnProxy(config, port, logger);
              setupCrashRecovery(child);
              await waitForHealth(port);
              logger.info(`maple-proxy restarted on http://127.0.0.1:${port}`);
              restartAttempts = 0;
            } catch (err) {
              logger.error(
                `Failed to restart maple-proxy: ${err instanceof Error ? err.message : err}`
              );
            }
          }, delay);
        } else {
          logger.error(
            `maple-proxy crashed ${MAX_RESTART_ATTEMPTS} times, giving up. ` +
              `Restart the gateway to try again.`
          );
        }
      }
    });
  };

  setupCrashRecovery(child);

  // Race health check against spawn errors so we fail fast if the binary
  // is missing, not executable, or crashes immediately on startup.
  try {
    await Promise.race([
      waitForHealth(port),
      new Promise<never>((_, reject) => {
        child.on("error", (err) => {
          reject(new Error(`maple-proxy failed to spawn: ${err.message}`));
        });
        child.on("exit", (code, signal) => {
          if (code !== null && code !== 0) {
            reject(new Error(`maple-proxy exited immediately with code ${code}`));
          } else if (signal) {
            reject(new Error(`maple-proxy killed by signal ${signal} during startup`));
          }
        });
      }),
    ]);
  } catch (err) {
    // Clean up the child process if it's still around
    if (!child.killed) {
      child.kill("SIGKILL");
    }
    stopped = true;
    throw err;
  }

  logger.info(`maple-proxy running on http://127.0.0.1:${port}`);

  return {
    process: child,
    port,
    version,
    kill: () => {
      stopped = true;
      if (!child.killed) {
        child.kill("SIGINT");
        setTimeout(() => {
          if (!child.killed) {
            child.kill("SIGTERM");
          }
        }, 3000);
      }
    },
  };
}
