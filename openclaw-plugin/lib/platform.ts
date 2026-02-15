import os from "node:os";
import path from "node:path";

const GITHUB_REPO = "OpenSecretCloud/maple-proxy";

export interface PlatformArtifact {
  name: string;
  archiveType: "tar.gz" | "zip";
}

export function getArtifact(): PlatformArtifact {
  const platform = os.platform();
  const arch = os.arch();

  if (platform === "linux" && arch === "x64") {
    return { name: "maple-proxy-linux-x86_64", archiveType: "tar.gz" };
  }
  if (platform === "linux" && arch === "arm64") {
    return { name: "maple-proxy-linux-aarch64", archiveType: "tar.gz" };
  }
  if (platform === "darwin" && arch === "arm64") {
    return { name: "maple-proxy-macos-aarch64", archiveType: "tar.gz" };
  }
  if (platform === "win32" && arch === "x64") {
    return { name: "maple-proxy-windows-x86_64", archiveType: "zip" };
  }

  throw new Error(
    `Unsupported platform: ${platform}/${arch}. ` +
      `Supported: linux/x64, linux/arm64, darwin/arm64, win32/x64`
  );
}

export function getReleaseUrl(version: string, artifact: PlatformArtifact): string {
  const ext = artifact.archiveType === "zip" ? "zip" : "tar.gz";
  return `https://github.com/${GITHUB_REPO}/releases/download/${version}/${artifact.name}.${ext}`;
}

export function getChecksumUrl(version: string, artifact: PlatformArtifact): string {
  const ext = artifact.archiveType === "zip" ? "zip" : "tar.gz";
  return `https://github.com/${GITHUB_REPO}/releases/download/${version}/${artifact.name}.${ext}.sha256`;
}

export function getCacheDir(): string {
  return path.join(os.homedir(), ".openclaw", "tools", "maple-proxy");
}

export function getBinaryName(): string {
  return os.platform() === "win32" ? "maple-proxy.exe" : "maple-proxy";
}

export function getBinaryPath(version: string): string {
  return path.join(getCacheDir(), version, getBinaryName());
}

export async function getLatestVersion(): Promise<string> {
  const url = `https://api.github.com/repos/${GITHUB_REPO}/releases/latest`;
  const res = await fetch(url, {
    headers: { Accept: "application/vnd.github.v3+json" },
  });
  if (!res.ok) {
    throw new Error(`Failed to fetch latest release: ${res.status} ${res.statusText}`);
  }
  const data = (await res.json()) as { tag_name: string };
  return data.tag_name;
}
