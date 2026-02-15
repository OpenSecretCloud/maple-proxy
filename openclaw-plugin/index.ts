import { ensureBinary } from "./lib/downloader.js";
import { startProxy, type RunningProxy } from "./lib/process.js";

interface PluginConfig {
  apiKey: string;
  port?: number;
  backendUrl?: string;
  debug?: boolean;
  version?: string;
}

interface PluginApi {
  config: { plugins: { entries: Record<string, { config: PluginConfig }> } };
  logger: { info: (msg: string) => void; error: (msg: string) => void };
  registerService: (service: {
    id: string;
    start: () => Promise<void>;
    stop: () => Promise<void>;
  }) => void;
  registerTool: (
    tool: {
      name: string;
      description: string;
      parameters: Record<string, unknown>;
      execute: (
        id: string,
        params: Record<string, unknown>
      ) => Promise<{ content: Array<{ type: string; text: string }> }>;
    },
    opts?: { optional?: boolean }
  ) => void;
}

export const id = "maple-openclaw-plugin";
export const name = "Maple Proxy";

const PLUGIN_CONFIG_KEY = "maple-openclaw-plugin";

export default function register(api: PluginApi) {
  let proxy: RunningProxy | null = null;

  api.registerTool({
    name: "maple_proxy_status",
    description:
      "Check the status of the local maple-proxy server. " +
      "Returns the port, version, and health status.",
    parameters: {
      type: "object",
      properties: {},
    },
    async execute() {
      if (!proxy) {
        return {
          content: [
            {
              type: "text",
              text: JSON.stringify({
                running: false,
                error: "maple-proxy is not running",
              }),
            },
          ],
        };
      }

      let healthy = false;
      try {
        const res = await fetch(
          `http://127.0.0.1:${proxy.port}/health`
        );
        healthy = res.ok;
      } catch {
        // Not healthy
      }

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify({
              running: true,
              healthy,
              port: proxy.port,
              version: proxy.version,
              endpoint: `http://127.0.0.1:${proxy.port}/v1`,
              modelsUrl: `http://127.0.0.1:${proxy.port}/v1/models`,
              chatUrl: `http://127.0.0.1:${proxy.port}/v1/chat/completions`,
            }),
          },
        ],
      };
    },
  });

  api.registerService({
    id: "maple-proxy-service",

    async start() {
      const pluginConfig =
        api.config.plugins.entries[PLUGIN_CONFIG_KEY]?.config;

      if (!pluginConfig?.apiKey) {
        api.logger.error(
          `${PLUGIN_CONFIG_KEY}: no apiKey configured. ` +
            `Set plugins.entries["${PLUGIN_CONFIG_KEY}"].config.apiKey in openclaw.json`
        );
        return;
      }

      try {
        const { binaryPath, version } = await ensureBinary(
          api.logger,
          pluginConfig.version
        );
        api.logger.info(`maple-proxy binary: ${version} at ${binaryPath}`);

        proxy = await startProxy(
          {
            binaryPath,
            apiKey: pluginConfig.apiKey,
            port: pluginConfig.port,
            backendUrl: pluginConfig.backendUrl,
            debug: pluginConfig.debug,
          },
          version,
          api.logger
        );

        api.logger.info(
          `maple-proxy is OpenAI-compatible at http://127.0.0.1:${proxy.port}/v1 ` +
            `-- configure as vLLM provider or use directly`
        );
      } catch (err) {
        api.logger.error(
          `${PLUGIN_CONFIG_KEY}: failed to start: ${err instanceof Error ? err.message : err}`
        );
      }
    },

    async stop() {
      if (proxy) {
        api.logger.info("Stopping maple-proxy...");
        proxy.kill();
        proxy = null;
      }
    },
  });
}
