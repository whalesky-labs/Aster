import { existsSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { arch, platform, release } from "node:os";

const root = process.cwd();
const packageDir = join(root, "docs", "acceptance-package");
const archivesDir = join(root, "docs", "acceptance-archives");
const evidenceDir = join(root, "docs", "release-evidence");
const manifestPath = join(packageDir, "acceptance-package-manifest.json");

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
    shell: process.platform === "win32",
  });
  if (result.status !== 0) {
    console.error(result.stdout);
    console.error(result.stderr);
    process.exit(result.status ?? 1);
  }
  return result;
}

function powerShellQuote(value) {
  return `'${value.replaceAll("'", "''")}'`;
}

function createArchive(archivePath) {
  if (process.platform === "win32") {
    const source = join("docs", "acceptance-package");
    const command = `Compress-Archive -Path ${powerShellQuote(source)} -DestinationPath ${powerShellQuote(archivePath)} -Force`;
    return {
      commandText: `powershell.exe -NoProfile -ExecutionPolicy Bypass -Command ${command}`,
      result: run("powershell.exe", ["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", command]),
    };
  }
  return {
    commandText: `zip -r -q ${archivePath} docs/acceptance-package`,
    result: run("zip", ["-r", "-q", archivePath, "docs/acceptance-package"]),
  };
}

run("npm", ["run", "acceptance:package"]);
run("npm", ["run", "verify:acceptance-package"]);

if (!existsSync(packageDir) || !existsSync(manifestPath)) {
  console.error("[acceptance-archive] docs/acceptance-package 不完整，无法归档");
  process.exit(1);
}

const manifest = readJson(manifestPath);
const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
const platformName = manifest.generatedOn?.evidencePlatform ?? process.platform;
const archiveName = `aster-acceptance-package-${platformName}-${timestamp}.zip`;
const archivePath = join(archivesDir, archiveName);
mkdirSync(archivesDir, { recursive: true });
mkdirSync(evidenceDir, { recursive: true });
if (existsSync(archivePath)) {
  rmSync(archivePath, { force: true });
}

const archiveResult = createArchive(archivePath);
const archiveStat = statSync(archivePath);
const archiveSha256 = sha256(archivePath);
const report = {
  generatedAt: new Date().toISOString(),
  status: "passed",
  platform: platform(),
  platformRelease: release(),
  arch: arch(),
  command: archiveResult.commandText,
  exitCode: archiveResult.result.status,
  archive: {
    path: archivePath,
    name: basename(archivePath),
    bytes: archiveStat.size,
    sha256: archiveSha256,
  },
  acceptancePackage: {
    manifestPath,
    generatedAt: manifest.generatedAt,
    fileInventoryCount: manifest.fileInventory?.length ?? 0,
    fileInventorySha256: manifest.fileInventorySha256 ?? null,
    remainingEvidence: manifest.remainingEvidence ?? null,
  },
};
const reportPath = join(evidenceDir, `acceptance-archive-${process.platform}-${timestamp}.json`);
writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);

console.log(`[acceptance-archive] Archive: ${archivePath}`);
console.log(`[acceptance-archive] SHA256: ${archiveSha256}`);
console.log(`[acceptance-archive] Report: ${reportPath}`);
