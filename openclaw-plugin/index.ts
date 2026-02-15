import { ensureBinary } from "./lib/downloader.js";
import { startProxy, type RunningProxy } from "./lib/process.js";

interface PluginConfig {
  apiKey: string;
  port?: number;
  backendUrl?: string;
  debug?: boolean;
}

interface PluginApi {
  config: { plugins: { entries: Record<string, { config: PluginConfig }> } };
  logger: { info: (msg: string) => void; error: (msg: string) => void };
  registerService: (service: {
    id: string;
    start: () => Promise<void>;
    stop: () => Promise<void>;
  }) => void;
}

export const id = "maple-proxy-openclaw-plugin";
export const name = "Maple Proxy";

export default function register(api: PluginApi) {
  let proxy: RunningProxy | null = null;

  api.registerService({
    id: "maple-proxy-service",

    async start() {
      const pluginConfig =
        api.config.plugins.entries["maple-proxy-openclaw-plugin"]?.config;

      if (!pluginConfig?.apiKey) {
        api.logger.error(
          "maple-proxy-openclaw-plugin: no apiKey configured. " +
            'Set plugins.entries["maple-proxy-openclaw-plugin"].config.apiKey in openclaw.json'
        );
        return;
      }

      try {
        const { binaryPath, version } = await ensureBinary(api.logger);
        api.logger.info(`maple-proxy binary: ${version} at ${binaryPath}`);

        proxy = await startProxy(
          {
            binaryPath,
            apiKey: pluginConfig.apiKey,
            port: pluginConfig.port,
            backendUrl: pluginConfig.backendUrl,
            debug: pluginConfig.debug,
          },
          api.logger
        );
      } catch (err) {
        api.logger.error(
          `maple-proxy-openclaw-plugin: failed to start: ${err instanceof Error ? err.message : err}`
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
