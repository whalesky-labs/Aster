import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { basename, join, relative, resolve } from "node:path";
import { createHash } from "node:crypto";

const root = process.cwd();
const sourceArg = process.argv[2];
if (!sourceArg) {
  console.error("[import-windows-artifacts] Usage: npm run acceptance:import-windows-artifacts -- <downloaded-artifacts-dir>");
  process.exit(1);
}

const sourceDir = resolve(root, sourceArg);
if (!existsSync(sourceDir) || !statSync(sourceDir).isDirectory()) {
  console.error(`[import-windows-artifacts] Source directory not found: ${sourceDir}`);
  process.exit(1);
}

const releaseEvidenceDir = join(root, "docs", "release-evidence");
const installerDir = join(root, "docs", "manual-acceptance", "windows-installers");
mkdirSync(releaseEvidenceDir, { recursive: true });
mkdirSync(installerDir, { recursive: true });

function collectFiles(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    return entry.isDirectory() ? collectFiles(path) : [path];
  });
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function projectRelative(path) {
  return relative(root, path).replaceAll("\\", "/");
}

const allFiles = collectFiles(sourceDir);
const evidenceFiles = allFiles.filter((path) =>
  /[/\\](aster-windows-x64-release-evidence|docs[/\\]release-evidence)[/\\]/.test(path) &&
  /(verify-release|execution-coverage|no-placeholder-scan|manual-acceptance-summary)-win32-.*\.json$/i.test(basename(path)),
);
const installerFiles = allFiles.filter((path) =>
  /[/\\]aster-windows-x64[/\\]/.test(path) && /\.(exe|msi)$/i.test(path),
);

if (!evidenceFiles.some((path) => basename(path).startsWith("verify-release-win32-"))) {
  console.error("[import-windows-artifacts] Missing verify-release-win32-*.json under aster-windows-x64-release-evidence.");
  process.exit(1);
}
if (installerFiles.length === 0) {
  console.error("[import-windows-artifacts] Missing Windows .exe or .msi under aster-windows-x64.");
  process.exit(1);
}

const copiedEvidence = evidenceFiles.map((source) => {
  const target = join(releaseEvidenceDir, basename(source));
  copyFileSync(source, target);
  return target;
});

const copiedInstallers = installerFiles.map((source) => {
  const target = join(installerDir, basename(source));
  copyFileSync(source, target);
  return target;
});

const releaseReports = copiedEvidence
  .filter((path) => basename(path).startsWith("verify-release-win32-"))
  .map((path) => ({ path, data: readJson(path) }))
  .sort((a, b) => new Date(b.data.generatedAt ?? 0).getTime() - new Date(a.data.generatedAt ?? 0).getTime());
const latestReleaseReport = releaseReports[0];
const expectedArtifactHashes = new Set(
  (latestReleaseReport.data.artifacts ?? [])
    .filter((artifact) => artifact?.type === "file" && /\.(exe|msi)$/i.test(artifact.path ?? ""))
    .map((artifact) => artifact.sha256)
    .filter(Boolean),
);
if (expectedArtifactHashes.size === 0) {
  console.error("[import-windows-artifacts] verify-release-win32 report does not contain Windows installer artifacts.");
  process.exit(1);
}

const installerResults = copiedInstallers.map((path) => ({
  path,
  sha256: sha256(path),
  matchesReleaseEvidence: expectedArtifactHashes.has(sha256(path)),
}));
if (!installerResults.some((item) => item.matchesReleaseEvidence)) {
  console.error("[import-windows-artifacts] No copied Windows installer SHA256 matches verify-release-win32 evidence.");
  for (const item of installerResults) {
    console.error(`- ${projectRelative(item.path)} ${item.sha256}`);
  }
  process.exit(1);
}

const report = {
  generatedAt: new Date().toISOString(),
  sourceDir,
  copiedEvidence: copiedEvidence.map(projectRelative),
  copiedInstallers: copiedInstallers.map(projectRelative),
  verifyReleaseReport: projectRelative(latestReleaseReport.path),
  installerResults: installerResults.map((item) => ({
    ...item,
    path: projectRelative(item.path),
  })),
};
const reportPath = join(releaseEvidenceDir, `windows-artifacts-import-${new Date().toISOString().replace(/[:.]/g, "-")}.json`);
writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);

console.log(`[import-windows-artifacts] Copied evidence: ${copiedEvidence.length}`);
console.log(`[import-windows-artifacts] Copied installers: ${copiedInstallers.length}`);
console.log(`[import-windows-artifacts] Report: ${reportPath}`);
