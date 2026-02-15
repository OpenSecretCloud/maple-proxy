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

async function downloadFile(url: string, dest: string): Promise<void> {
  const res = await fetch(url, { redirect: "follow" });
  if (!res.ok) {
    throw new Error(`Download failed: ${res.status} ${res.statusText} (${url})`);
  }
  const buffer = Buffer.from(await res.arrayBuffer());
  await fsp.writeFile(dest, buffer);
}

async function verifyChecksum(filePath: string, checksumUrl: string): Promise<void> {
  const res = await fetch(checksumUrl, { redirect: "follow" });
  if (!res.ok) {
    // Checksum file may not exist for older releases; warn but don't fail
    return;
  }

  const expectedLine = (await res.text()).trim();
  // Format: "<hash>  <filename>" or just "<hash>"
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
      "-Command",
      `Expand-Archive -Path '${archivePath}' -DestinationPath '${destDir}' -Force`,
    ]);
  } else {
    await execFileAsync("unzip", ["-o", archivePath, "-d", destDir]);
  }
}

export interface DownloadResult {
  binaryPath: string;
  version: string;
}

export async function ensureBinary(logger: { info: (msg: string) => void }): Promise<DownloadResult> {
  const version = await getLatestVersion();
  const binaryPath = getBinaryPath(version);

  if (fs.existsSync(binaryPath)) {
    logger.info(`maple-proxy ${version} already cached at ${binaryPath}`);
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
  await verifyChecksum(archivePath, checksumUrl);

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
  return { binaryPath, version };
}
