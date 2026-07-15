import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { arch, platform, release } from "node:os";

const root = process.cwd();
const evidenceDir = join(root, "docs", "release-evidence");
const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
const currentPlatform = process.env.ASTER_ACCEPTANCE_PLATFORM || platform();
const reportPath = join(evidenceDir, `acceptance-finalize-${currentPlatform}-${timestamp}.json`);
const commandResults = [];

function commandFromEnv(envName, fallbackArgs, fallbackText) {
  const override = process.env[envName];
  if (!override) {
    return {
      command: "npm",
      args: fallbackArgs,
      text: fallbackText,
    };
  }
  return {
    command: process.execPath,
    args: ["-e", override],
    text: `${envName} override`,
  };
}

const commands = [
  commandFromEnv(
    "ASTER_FINALIZE_VERIFY_STRICT_COMMAND",
    ["run", "verify:manual-acceptance", "--", "--strict"],
    "npm run verify:manual-acceptance -- --strict",
  ),
  commandFromEnv("ASTER_FINALIZE_PACKAGE_COMMAND", ["run", "acceptance:package"], "npm run acceptance:package"),
  commandFromEnv("ASTER_FINALIZE_READINESS_COMMAND", ["run", "verify:readiness"], "npm run verify:readiness"),
  commandFromEnv("ASTER_FINALIZE_PACKAGE_COMMAND", ["run", "acceptance:package"], "npm run acceptance:package"),
  commandFromEnv(
    "ASTER_FINALIZE_VERIFY_PACKAGE_COMMAND",
    ["run", "verify:acceptance-package"],
    "npm run verify:acceptance-package",
  ),
  commandFromEnv("ASTER_FINALIZE_ARCHIVE_COMMAND", ["run", "acceptance:archive"], "npm run acceptance:archive"),
];

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function latestEvidence(prefix) {
  if (!existsSync(evidenceDir)) return null;
  const files = readdirSync(evidenceDir)
    .filter((name) => name.startsWith(prefix) && name.endsWith(".json"))
    .map((name) => join(evidenceDir, name))
    .sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs || b.localeCompare(a));
  return files[0] ?? null;
}

function writeReport(status, failure = null) {
  mkdirSync(evidenceDir, { recursive: true });
  const readinessPath = latestEvidence(`readiness-${currentPlatform}-`);
  const summaryPath = latestEvidence(`manual-acceptance-summary-${currentPlatform}-`);
  const archivePath = latestEvidence(`acceptance-archive-${currentPlatform}-`);
  const readiness = readinessPath ? readJson(readinessPath) : null;
  const summary = summaryPath ? readJson(summaryPath) : null;
  const archive = archivePath ? readJson(archivePath) : null;
  const report = {
    generatedAt: new Date().toISOString(),
    status,
    platform: currentPlatform,
    platformRelease: release(),
    arch: arch(),
    commands: commands.map((item) => item.text),
    commandResults,
    failure,
    evidence: {
      readiness: readinessPath,
      manualAcceptanceSummary: summaryPath,
      acceptanceArchive: archivePath,
    },
    readinessStatus: readiness?.status ?? null,
    manualAcceptanceStatus: summary?.status ?? null,
    remainingEvidenceCount:
      readiness?.remainingEvidence?.remainingCount ?? summary?.remainingEvidence?.remainingCount ?? null,
    archiveSha256: archive?.archive?.sha256 ?? null,
  };
  writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
  console.log(`[acceptance-finalize] Report: ${reportPath}`);
}

function runCommand(item) {
  const startedAt = new Date();
  console.log(`\n$ ${item.text}`);
  const result = spawnSync(item.command, item.args, {
    cwd: root,
    env: process.env,
    shell: process.platform === "win32",
    stdio: "inherit",
  });
  const finishedAt = new Date();
  const commandResult = {
    command: item.text,
    cwd: root,
    status: result.status === 0 ? "passed" : "failed",
    exitCode: result.status,
    signal: result.signal ?? null,
    startedAt: startedAt.toISOString(),
    finishedAt: finishedAt.toISOString(),
    durationMs: finishedAt.getTime() - startedAt.getTime(),
  };
  commandResults.push(commandResult);
  if (result.status !== 0) {
    writeReport("failed", {
      failedCommand: commandResult,
    });
    process.exit(result.status ?? 1);
  }
}

for (const item of commands) {
  runCommand(item);
}

const readinessPath = latestEvidence(`readiness-${currentPlatform}-`);
const readiness = readinessPath ? readJson(readinessPath) : null;
if (readiness?.status !== "ready-for-final-archive") {
  const failure = {
    message: "最终归档要求 strict manual acceptance 已完整通过，readiness 状态必须为 ready-for-final-archive。",
    readinessPath,
    readinessStatus: readiness?.status ?? null,
    remainingEvidence: readiness?.remainingEvidence ?? null,
  };
  console.error(`[acceptance-finalize] ${failure.message}`);
  writeReport("failed", failure);
  process.exit(1);
}

writeReport("passed");
console.log("[acceptance-finalize] Final acceptance archive completed.");
