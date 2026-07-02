import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { basename, join } from "node:path";
import { arch, platform, release } from "node:os";

const root = process.cwd();
const evidenceDir = join(root, "docs", "release-evidence");
const packageManifestPath = join(root, "docs", "acceptance-package", "acceptance-package-manifest.json");
const summaryOutput = process.argv.includes("--summary");
const requestedPlatform = process.env.ASTER_ACCEPTANCE_PLATFORM || process.platform;
const evidencePlatform =
  requestedPlatform === "darwin" ? "darwin" : requestedPlatform === "win32" ? "win32" : requestedPlatform;

const evidenceChecks = [
  {
    id: "verify-all-local",
    prefix: `verify-all-local-${evidencePlatform}-`,
    statusPath: "status",
    expected: "passed",
    required: true,
  },
  {
    id: "verify-release",
    prefix: `verify-release-${evidencePlatform}-`,
    statusPath: "status",
    expected: "passed",
    required: true,
  },
  {
    id: "execution-coverage",
    prefix: `execution-coverage-${evidencePlatform}-`,
    statusPath: "coverageStatus",
    expected: "covered",
    required: true,
  },
  {
    id: "no-placeholder-scan",
    prefix: `no-placeholder-scan-${evidencePlatform}-`,
    statusPath: "status",
    expected: "passed",
    required: true,
  },
  {
    id: "readiness",
    prefix: `readiness-${evidencePlatform}-`,
    statusPath: "status",
    expected: "ready-for-manual-evidence",
    alsoAccepted: ["ready-for-final-archive"],
    required: true,
  },
  {
    id: "manual-acceptance-summary",
    prefix: `manual-acceptance-summary-${evidencePlatform}-`,
    statusPath: "status",
    expected: "complete",
    alsoAccepted: ["incomplete"],
    required: true,
  },
  {
    id: "acceptance-archive",
    prefix: `acceptance-archive-${evidencePlatform}-`,
    statusPath: "status",
    expected: "passed",
    required: false,
  },
  {
    id: "acceptance-finalize",
    prefix: `acceptance-finalize-${evidencePlatform}-`,
    statusPath: "status",
    expected: "passed",
    alsoAccepted: ["failed"],
    required: false,
  },
];

function readJson(path) {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch {
    return null;
  }
}

function latestEvidence(prefix) {
  if (!existsSync(evidenceDir)) return null;
  const files = readdirSync(evidenceDir)
    .filter((name) => name.startsWith(prefix) && name.endsWith(".json"))
    .map((name) => join(evidenceDir, name))
    .sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs || b.localeCompare(a));
  return files[0] ?? null;
}

function valueAt(data, path) {
  return path.split(".").reduce((value, key) => value?.[key], data);
}

function statusFor(check) {
  const path = latestEvidence(check.prefix);
  const data = path ? readJson(path) : null;
  const actual = data ? valueAt(data, check.statusPath) : null;
  const accepted = [check.expected, ...(check.alsoAccepted ?? [])];
  const ok = data && accepted.includes(actual);
  return {
    id: check.id,
    required: check.required,
    ok: Boolean(ok),
    expected: check.expected,
    actual,
    path,
    file: path ? basename(path) : null,
    data,
  };
}

function summaryMarker(check) {
  if (check.id === "acceptance-finalize" && check.actual === "failed") {
    return "WAIT";
  }
  return check.ok ? "OK" : "FAIL";
}

function generatedTime(check) {
  const value = check?.data?.generatedAt;
  const time = value ? new Date(value).getTime() : NaN;
  return Number.isFinite(time) ? time : null;
}

const checks = evidenceChecks.map(statusFor);
const manifest = existsSync(packageManifestPath) ? readJson(packageManifestPath) : null;
const readiness = checks.find((item) => item.id === "readiness")?.data ?? null;
const manualSummary = checks.find((item) => item.id === "manual-acceptance-summary")?.data ?? null;
const archive = checks.find((item) => item.id === "acceptance-archive")?.data ?? null;
const requiredFreshnessChecks = checks.filter((item) =>
  ["verify-release", "execution-coverage", "no-placeholder-scan"].includes(item.id),
);
const freshnessBlockers = [];
const allLocalCheck = checks.find((item) => item.id === "verify-all-local");
for (const item of requiredFreshnessChecks) {
  if (!item.file) continue;
  const packageEvidencePath = `docs/acceptance-package/release-evidence/${item.file}`;
  if (manifest && !manifest.copiedEvidence?.includes(packageEvidencePath)) {
    freshnessBlockers.push(`验收交接包未记录最新 ${item.id} 证据：${item.file}`);
  }
  if (!existsSync(join(root, packageEvidencePath))) {
    freshnessBlockers.push(`验收交接包未携带最新 ${item.id} 证据：${item.file}`);
  }
  const allLocalTime = generatedTime(allLocalCheck);
  const itemTime = generatedTime(item);
  if (allLocalTime !== null && itemTime !== null && allLocalTime < itemTime) {
    freshnessBlockers.push(`verify-all-local 早于最新 ${item.id} 证据，请重新执行 npm run verify:all-local`);
  }
}
const readinessCheck = checks.find((item) => item.id === "readiness");
if (readinessCheck?.file && manifest) {
  const readinessPackagePath = `docs/acceptance-package/release-evidence/${readinessCheck.file}`;
  if (!manifest.copiedEvidence?.includes(readinessPackagePath)) {
    freshnessBlockers.push(`验收交接包未记录最新 readiness 证据：${readinessCheck.file}`);
  }
  if (!existsSync(join(root, readinessPackagePath))) {
    freshnessBlockers.push(`验收交接包未携带最新 readiness 证据：${readinessCheck.file}`);
  }
  const allLocalTime = generatedTime(allLocalCheck);
  const readinessTime = generatedTime(readinessCheck);
  if (allLocalTime !== null && readinessTime !== null && allLocalTime < readinessTime) {
    freshnessBlockers.push("verify-all-local 早于最新 readiness 证据，请重新执行 npm run verify:all-local");
  }
}
const remainingEvidence = readiness?.remainingEvidence ?? manualSummary?.remainingEvidence ?? manifest?.remainingEvidence ?? null;
const remainingEvidenceItems = remainingEvidence?.items ?? [];
const missingWindowsGitHubEvidence = remainingEvidenceItems.some((item) => item.id === "windows_record");
const missingRequired = checks.filter((item) => item.required && !item.ok);
const automaticBlockers = [...(readiness?.blockers ?? []), ...freshnessBlockers];
const readyForFinalArchive = readiness?.status === "ready-for-final-archive" && remainingEvidence?.status === "complete";
const readyForManualEvidence =
  missingRequired.length === 0 &&
  automaticBlockers.length === 0 &&
  readiness?.status === "ready-for-manual-evidence" &&
  remainingEvidence?.remainingCount > 0;
const overallStatus = readyForFinalArchive
  ? "ready-for-final-archive"
  : readyForManualEvidence
    ? "ready-for-manual-evidence"
    : missingRequired.length > 0 || automaticBlockers.length > 0
      ? "blocked"
      : "needs-review";
const needsAllLocalRefresh = automaticBlockers.some((item) => item.includes("verify-all-local 早于最新"));
const nextCommand =
  overallStatus === "ready-for-final-archive"
    ? "npm run acceptance:finalize"
    : overallStatus === "ready-for-manual-evidence"
      ? missingWindowsGitHubEvidence
        ? "先执行 GITHUB_TOKEN=<token> npm run acceptance:download-windows-artifacts，或从 GitHub Actions 的 Build Desktop Bundles 下载 aster-windows-x64 和 aster-windows-x64-release-evidence 后执行 npm run acceptance:import-windows-artifacts -- <下载目录>，再补齐 REMAINING_EVIDENCE.md 后执行 npm run acceptance:finalize"
        : "补齐 REMAINING_EVIDENCE.md 中列出的实机证据后执行 npm run acceptance:finalize"
      : needsAllLocalRefresh
        ? "最新自动证据晚于 verify-all-local，请执行 npm run verify:all-local 刷新整条本机门禁和交接包"
        : "先修复缺失或失败的自动证据，再执行 npm run verify:readiness";

const report = {
  generatedAt: new Date().toISOString(),
  status: overallStatus,
  platform: platform(),
  platformRelease: release(),
  arch: arch(),
  evidencePlatform,
  checks: checks.map((item) => ({
    id: item.id,
    required: item.required,
    ok: item.ok,
    expected: item.expected,
    actual: item.actual,
    path: item.path,
  })),
  manifest: {
    path: existsSync(packageManifestPath) ? packageManifestPath : null,
    evidencePlatform: manifest?.generatedOn?.evidencePlatform ?? null,
    remainingEvidenceCount: manifest?.remainingEvidence?.remainingCount ?? null,
  },
  remainingEvidence,
  automaticBlockers,
  archive: archive
    ? {
        path: archive.archive?.path ?? null,
        name: archive.archive?.name ?? null,
        sha256: archive.archive?.sha256 ?? null,
      }
    : null,
  archiveSha256: archive?.archive?.sha256 ?? null,
  nextCommand,
};

if (summaryOutput) {
  const lines = [
    "Aster acceptance status",
    `Status: ${report.status}`,
    `Evidence platform: ${report.evidencePlatform}`,
    `Automatic blockers: ${report.automaticBlockers.length}`,
    `Remaining evidence groups: ${report.remainingEvidence?.remainingCount ?? "unknown"}`,
    `Archive: ${report.archive?.name ?? "not generated"}`,
    `Archive SHA256: ${report.archiveSha256 ?? "not generated"}`,
    "",
    "Checks:",
  ];
  for (const check of report.checks) {
    const marker = summaryMarker(check);
    lines.push(`- ${marker} ${check.id}: ${check.actual ?? "missing"}`);
  }
  if (report.automaticBlockers.length > 0) {
    lines.push("");
    lines.push("Automatic blockers:");
    for (const item of report.automaticBlockers) {
      lines.push(`- ${item}`);
    }
  }
  if ((report.remainingEvidence?.items ?? []).length > 0) {
    lines.push("");
    lines.push("Remaining evidence:");
    for (const item of report.remainingEvidence.items) {
      lines.push(`- ${item.label}: ${item.missing?.length ?? 0} item(s)`);
    }
  }
  lines.push("");
  lines.push(`Next: ${report.nextCommand}`);
  console.log(lines.join("\n"));
} else {
  console.log(JSON.stringify(report, null, 2));
}

if (overallStatus === "blocked") {
  process.exit(1);
}
