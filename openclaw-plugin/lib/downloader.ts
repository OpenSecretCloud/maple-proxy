import fs from "node:fs";
import fsp from "node:fs/promises";
import path from "node:path";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import {
  getArtifact,
  getReleaseUrl,
  getChecksumUrl,
  getBinaryPath,
  getCacheDir,
  getLatestVersion,
} from "./platform.js";

const execFileAsync = promisify(execFile);

const VERSION_CHECK_TTL_MS = 24 * 60 * 60 * 1000; // 24 hours
const MAX_KEPT_VERSIONS = 2; // current + one previous

function parseVer(v: string): number[] {
  return v.replace(/^v/, "").split(".").map(Number);
}

export function compareVersionsDesc(a: string, b: string): number {
  const [aMaj = 0, aMin = 0, aPat = 0] = parseVer(a);
  const [bMaj = 0, bMin = 0, bPat = 0] = parseVer(b);
  return bMaj - aMaj || bMin - aMin || bPat - aPat;
}

async function downloadFile(url: string, dest: string): Promise<void> {
  const res = await fetch(url, { redirect: "follow" });
  if (!res.ok) {
    throw new Error(`Download failed: ${res.status} ${res.statusText} (${url})`);
  }
  const buffer = Buffer.from(await res.arrayBuffer());
  await fsp.writeFile(dest, buffer);
}

async function verifyChecksum(
  filePath: string,
  checksumUrl: string,
  logger: { info: (msg: string) => void }
): Promise<void> {
  const res = await fetch(checksumUrl, { redirect: "follow" });
  if (!res.ok) {
    if (res.status === 404) {
      logger.info(
        `Warning: checksum file not found (404), skipping verification for ${path.basename(filePath)}`
      );
      return;
    }
    throw new Error(
      `Failed to fetch checksum for ${path.basename(filePath)}: ${res.status} ${res.statusText}. ` +
        `This may indicate GitHub rate limiting or a server error.`
    );
  }

  const expectedLine = (await res.text()).trim();
  const expectedHash = expectedLine.split(/\s+/)[0];

  const { createHash } = await import("node:crypto");
  const fileBuffer = await fsp.readFile(filePath);
  const actualHash = createHash("sha256").update(fileBuffer).digest("hex");

  if (actualHash !== expectedHash) {
    await fsp.unlink(filePath);
    throw new Error(
      `Checksum mismatch for ${path.basename(filePath)}: ` +
        `expected ${expectedHash}, got ${actualHash}`
    );
  }
}

async function extractTarGz(archivePath: string, destDir: string): Promise<void> {
  await execFileAsync("tar", ["-xzf", archivePath, "-C", destDir]);
}

async function extractZip(archivePath: string, destDir: string): Promise<void> {
  if (process.platform === "win32") {
    await execFileAsync("powershell", [
      "-NoProfile",
      "-Command",
      "Expand-Archive",
      "-Path",
      archivePath,
      "-DestinationPath",
      destDir,
      "-Force",
    ]);
  } else {
    await execFileAsync("unzip", ["-o", archivePath, "-d", destDir]);
  }
}

async function resolveVersion(
  logger: { info: (msg: string) => void },
  pinnedVersion?: string
): Promise<string> {
  if (pinnedVersion) {
    return pinnedVersion;
  }

  const cacheDir = getCacheDir();
  const cacheFile = path.join(cacheDir, ".latest-version");

  try {
    const stat = await fsp.stat(cacheFile);
    const age = Date.now() - stat.mtimeMs;
    if (age < VERSION_CHECK_TTL_MS) {
      const cached = (await fsp.readFile(cacheFile, "utf-8")).trim();
      if (cached) {
        logger.info(`Using cached latest version: ${cached} (checked ${Math.round(age / 60000)}m ago)`);
        return cached;
      }
    }
  } catch {
    // No cache file or unreadable
  }

  logger.info("Checking GitHub for latest maple-proxy release...");
  const version = await getLatestVersion();

  await fsp.mkdir(cacheDir, { recursive: true });
  await fsp.writeFile(cacheFile, version, "utf-8");

  return version;
}

async function cleanupOldVersions(
  currentVersion: string,
  logger: { info: (msg: string) => void }
): Promise<void> {
  const cacheDir = getCacheDir();

  let entries: string[];
  try {
    entries = await fsp.readdir(cacheDir);
  } catch {
    return;
  }

  const versionDirs = entries
    .filter((e) => e.startsWith("v"))
    .sort(compareVersionsDesc);

  if (versionDirs.length <= MAX_KEPT_VERSIONS) {
    return;
  }

  // Always keep the current version; keep most recent others up to MAX_KEPT_VERSIONS
  const toKeep = new Set<string>([currentVersion]);
  for (const dir of versionDirs) {
    if (toKeep.size >= MAX_KEPT_VERSIONS) break;
    toKeep.add(dir);
  }

  for (const dir of versionDirs) {
    if (toKeep.has(dir)) continue;
    const dirPath = path.join(cacheDir, dir);
    try {
      await fsp.rm(dirPath, { recursive: true, force: true });
      logger.info(`Cleaned up old maple-proxy version: ${dir}`);
    } catch {
      // Best-effort cleanup
    }
  }
}

export interface DownloadResult {
  binaryPath: string;
  version: string;
}

export async function ensureBinary(
  logger: { info: (msg: string) => void },
  pinnedVersion?: string
): Promise<DownloadResult> {
  const version = await resolveVersion(logger, pinnedVersion);
  const binaryPath = getBinaryPath(version);

  if (fs.existsSync(binaryPath)) {
    logger.info(`maple-proxy ${version} already cached at ${binaryPath}`);
    await cleanupOldVersions(version, logger);
    return { binaryPath, version };
  }

  const artifact = getArtifact();
  const cacheDir = getCacheDir();
  const versionDir = path.join(cacheDir, version);
  await fsp.mkdir(versionDir, { recursive: true });

  const ext = artifact.archiveType === "zip" ? "zip" : "tar.gz";
  const archivePath = path.join(versionDir, `${artifact.name}.${ext}`);

  logger.info(`Downloading maple-proxy ${version} for ${artifact.name}...`);
  const releaseUrl = getReleaseUrl(version, artifact);
  await downloadFile(releaseUrl, archivePath);

  const checksumUrl = getChecksumUrl(version, artifact);
  await verifyChecksum(archivePath, checksumUrl, logger);

  logger.info(`Extracting to ${versionDir}...`);
  if (artifact.archiveType === "zip") {
    await extractZip(archivePath, versionDir);
  } else {
    await extractTarGz(archivePath, versionDir);
  }

  await fsp.unlink(archivePath);

  if (process.platform !== "win32") {
    await fsp.chmod(binaryPath, 0o755);
  }

  logger.info(`maple-proxy ${version} ready at ${binaryPath}`);
  await cleanupOldVersions(version, logger);
  return { binaryPath, version };
}
