import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { arch, platform, release } from "node:os";

const root = process.cwd();
const evidenceDir = join(root, "docs", "release-evidence");
const packageDir = join(root, "docs", "acceptance-package");
const currentPlatform = process.env.ASTER_ACCEPTANCE_PLATFORM || process.platform;
const evidencePlatform =
  currentPlatform === "darwin" ? "darwin" : currentPlatform === "win32" ? "win32" : currentPlatform;

const requiredAutomaticEvidence = [
  {
    prefix: `verify-all-local-${evidencePlatform}-`,
    label: "verify-all-local",
    statusPath: "status",
    expectedStatus: "passed",
  },
  {
    prefix: `verify-release-${evidencePlatform}-`,
    label: "verify-release",
    statusPath: "status",
    expectedStatus: "passed",
  },
  {
    prefix: `execution-coverage-${evidencePlatform}-`,
    label: "execution-coverage",
    statusPath: "coverageStatus",
    expectedStatus: "covered",
  },
  {
    prefix: `no-placeholder-scan-${evidencePlatform}-`,
    label: "no-placeholder-scan",
    statusPath: "status",
    expectedStatus: "passed",
  },
];
const optionalAutomaticEvidence = [
  {
    prefix: `acceptance-archive-${evidencePlatform}-`,
    label: "acceptance-archive",
    statusPath: "status",
    expectedStatus: "passed",
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

function generatedTime(report) {
  const value = report?.generatedAt;
  const time = value ? new Date(value).getTime() : NaN;
  return Number.isFinite(time) ? time : null;
}

function packageHasEvidence(name) {
  return existsSync(join(packageDir, "release-evidence", name));
}

const checks = [];
const blockers = [];

const manifestPath = join(packageDir, "acceptance-package-manifest.json");
const manifest = existsSync(manifestPath) ? readJson(manifestPath) : null;
if (!manifest) {
  blockers.push("验收交接包 manifest 缺失或无法解析，请先运行 npm run acceptance:package");
} else {
  checks.push({
    id: "acceptance_package_manifest",
    status: "passed",
    path: manifestPath,
  });
  if (manifest.generatedOn?.evidencePlatform !== evidencePlatform) {
    blockers.push(`验收交接包 evidencePlatform 必须为 ${evidencePlatform}`);
  }
}

for (const item of requiredAutomaticEvidence) {
  const path = latestEvidence(item.prefix);
  const name = path ? basename(path) : null;
  const report = path ? readJson(path) : null;
  const actualStatus = report ? valueAt(report, item.statusPath) : null;
  const ok = Boolean(report) && actualStatus === item.expectedStatus;
  checks.push({
    id: item.label,
    status: ok ? "passed" : "failed",
    path,
    expectedStatus: item.expectedStatus,
    actualStatus,
  });
  if (!ok) {
    blockers.push(`${item.label} 最新证据必须为 ${item.expectedStatus}`);
    continue;
  }
  if (item.label !== "verify-all-local") {
    const allLocalCheck = checks.find((check) => check.id === "verify-all-local");
    const allLocalReport = allLocalCheck?.path ? readJson(allLocalCheck.path) : null;
    const allLocalTime = generatedTime(allLocalReport);
    const itemTime = generatedTime(report);
    if (allLocalTime !== null && itemTime !== null && allLocalTime < itemTime) {
      blockers.push(`verify-all-local 早于最新 ${item.label} 证据，请重新执行 npm run verify:all-local`);
    }
    if (manifest && !manifest.copiedEvidence?.includes(`docs/acceptance-package/release-evidence/${name}`)) {
      blockers.push(`验收交接包 manifest 未记录最新 ${item.label} 证据：${name}`);
    }
    if (name && !packageHasEvidence(name)) {
      blockers.push(`验收交接包未携带最新 ${item.label} 证据：${name}`);
    }
  }
}

for (const item of optionalAutomaticEvidence) {
  const path = latestEvidence(item.prefix);
  const report = path ? readJson(path) : null;
  checks.push({
    id: item.label,
    status: report ? valueAt(report, item.statusPath) : "not-generated",
    path,
    expectedStatus: item.expectedStatus,
    optional: true,
  });
}

const summaryPath = latestEvidence(`manual-acceptance-summary-${evidencePlatform}-`);
const manualSummary = summaryPath ? readJson(summaryPath) : null;
const remainingEvidence = manualSummary?.remainingEvidence ?? manifest?.remainingEvidence ?? null;
const missingWindowsGitHubEvidence = (remainingEvidence?.items ?? []).some((item) => item.id === "windows_record");
if (!manualSummary) {
  blockers.push("缺少 strict manual acceptance 汇总，请先运行 npm run verify:manual-acceptance -- --strict");
} else {
  checks.push({
    id: "manual_acceptance_summary",
    status: "checked",
    path: summaryPath,
    strict: manualSummary.strict,
    acceptanceStatus: manualSummary.status,
  });
}

if (!remainingEvidence) {
  blockers.push("缺少 remainingEvidence，无法判断实机验收缺口");
} else if (!Number.isInteger(remainingEvidence.remainingCount)) {
  blockers.push("remainingEvidence.remainingCount 必须为整数");
}

const status =
  blockers.length > 0
    ? "blocked"
    : remainingEvidence?.status === "complete"
      ? "ready-for-final-archive"
      : "ready-for-manual-evidence";

const report = {
  generatedAt: new Date().toISOString(),
  status,
  platform: platform(),
  platformRelease: release(),
  arch: arch(),
  evidencePlatform,
  checks,
  blockers,
  remainingEvidence,
  nextCommand:
    status === "ready-for-final-archive"
      ? "npm run verify:manual-acceptance -- --strict"
      : missingWindowsGitHubEvidence
        ? "先执行 GITHUB_TOKEN=<token> npm run acceptance:download-windows-artifacts，或从 GitHub Actions 的 Build Desktop Bundles 下载 aster-windows-x64 和 aster-windows-x64-release-evidence 后执行 npm run acceptance:import-windows-artifacts -- <下载目录>，再补齐 REMAINING_EVIDENCE.md 后执行 npm run verify:manual-acceptance -- --strict"
        : "补齐 REMAINING_EVIDENCE.md 中列出的实机证据后执行 npm run verify:manual-acceptance -- --strict",
};

mkdirSync(evidenceDir, { recursive: true });
const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
const reportPath = join(evidenceDir, `readiness-${evidencePlatform}-${timestamp}.json`);
writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);

if (blockers.length > 0) {
  console.error("[verify-readiness] Readiness is blocked:");
  for (const item of blockers) {
    console.error(`- ${item}`);
  }
  console.error(`[verify-readiness] Report: ${reportPath}`);
  process.exit(1);
}

console.log(`[verify-readiness] ${status}`);
console.log(`[verify-readiness] Remaining evidence groups: ${remainingEvidence?.remainingCount ?? 0}`);
console.log(`[verify-readiness] Report: ${reportPath}`);
