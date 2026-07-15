import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { basename, join } from "node:path";
import { platform } from "node:os";
import { createHash } from "node:crypto";

const root = process.cwd();
const packageDir = join(root, "docs", "acceptance-package");
const currentPlatform = process.env.ASTER_ACCEPTANCE_PLATFORM || platform();
const evidencePlatform =
  currentPlatform === "darwin" ? "darwin" : currentPlatform === "win32" ? "win32" : currentPlatform;
const manualPlatform = evidencePlatform === "darwin" ? "macos" : evidencePlatform === "win32" ? "windows" : evidencePlatform;

const requiredFiles = [
  "README.md",
  "REMAINING_EVIDENCE.md",
  "acceptance-package-manifest.json",
  "docs/README.md",
  "docs/docs__ASTER_EXECUTION_PLAN.md",
  "docs/docs__ASTER_CROSS_PLATFORM_ACCEPTANCE.md",
  "docs/docs__ASTER_DISASTER_RECOVERY_RUNBOOK.md",
  "docs/docs__manual-acceptance__README.md",
  "manual-acceptance/manual-acceptance-windows-template-note.json",
  "manual-acceptance/manual-acceptance-macos-template-note.json",
  "runners/README.md",
  "runners/run-windows-acceptance.ps1",
  "runners/run-macos-acceptance.sh",
];

const copiedDocSources = [
  "README.md",
  "docs/ASTER_EXECUTION_PLAN.md",
  "docs/ASTER_CROSS_PLATFORM_ACCEPTANCE.md",
  "docs/ASTER_DISASTER_RECOVERY_RUNBOOK.md",
  "docs/manual-acceptance/README.md",
];

const evidencePrefixes = [
  `verify-all-local-${evidencePlatform}-`,
  `verify-release-${evidencePlatform}-`,
  `execution-coverage-${evidencePlatform}-`,
  `no-placeholder-scan-${evidencePlatform}-`,
  `readiness-${evidencePlatform}-`,
  `manual-acceptance-summary-${evidencePlatform}-`,
];
const requiredReleaseCommands = [
  "npm run build",
  "npm run verify:coverage",
  "npm run verify:no-placeholders",
  "cargo fmt --check",
  "cargo test",
  "npm run tauri -- build",
];
const commandMatchers = {
  "npm run tauri -- build": (command) => command === "npm run tauri -- build" || command.startsWith("npm run tauri -- build "),
};

const missing = [];

function readJson(path) {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch {
    return null;
  }
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function normalizeManifestPath(path) {
  return path?.replaceAll("\\", "/");
}

function latestEvidenceName(prefix) {
  const evidenceDir = join(root, "docs", "release-evidence");
  if (!existsSync(evidenceDir)) return null;
  const files = readdirSync(evidenceDir)
    .filter((name) => name.startsWith(prefix) && name.endsWith(".json"))
    .map((name) => join(evidenceDir, name))
    .sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs || b.localeCompare(a));
  return files[0] ? basename(files[0]) : null;
}

function latestManualAcceptanceRelativePath(prefix, suffix) {
  const manualDir = join(root, "docs", "manual-acceptance");
  if (!existsSync(manualDir)) return null;
  const files = readdirSync(manualDir, { recursive: true })
    .filter((name) => name.startsWith(prefix) && name.endsWith(suffix))
    .map((name) => join(manualDir, name))
    .sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs || b.localeCompare(a));
  return files[0]
    ? normalizeManifestPath(files[0].replace(manualDir, "").replace(/^[/\\]/, ""))
    : null;
}

if (!existsSync(packageDir)) {
  missing.push("docs/acceptance-package 目录不存在，请先运行 npm run acceptance:package");
} else {
  for (const relativePath of requiredFiles) {
    const path = join(packageDir, relativePath);
    if (!existsSync(path) || statSync(path).size === 0) {
      missing.push(`${relativePath} 缺失或为空`);
    }
  }

  const evidenceDir = join(packageDir, "release-evidence");
  const evidenceFiles = existsSync(evidenceDir) ? readdirSync(evidenceDir) : [];
  for (const prefix of evidencePrefixes) {
    const latestName = latestEvidenceName(prefix);
    if (!evidenceFiles.some((name) => name.startsWith(prefix) && name.endsWith(".json"))) {
      missing.push(`release-evidence 缺少 ${prefix}*.json`);
    } else if (latestName && !evidenceFiles.includes(latestName)) {
      missing.push(`release-evidence 未包含当前最新证据：${latestName}`);
    }
  }

  const manifestPath = join(packageDir, "acceptance-package-manifest.json");
  const manifest = readJson(manifestPath);
  const packageReadmePath = join(packageDir, "README.md");
  const packageReadme = existsSync(packageReadmePath) ? readFileSync(packageReadmePath, "utf8") : "";
  for (const expectedText of [
    "REMAINING_EVIDENCE.md",
    "acceptance-package-manifest.json",
    "manual-acceptance/",
    "release-evidence/",
    "npm run verify:release",
    "npm run verify:no-placeholders",
  ]) {
    if (!packageReadme.includes(expectedText)) {
      missing.push(`README.md 缺少交接说明：${expectedText}`);
    }
  }
  if (!manifest) {
    missing.push("acceptance-package-manifest.json 无法解析");
  } else {
    if (manifest.generatedOn?.platform !== currentPlatform) {
      missing.push(`manifest 平台必须为 ${currentPlatform}`);
    }
    if (manifest.generatedOn?.evidencePlatform !== evidencePlatform) {
      missing.push(`manifest evidencePlatform 必须为 ${evidencePlatform}`);
    }
    if (!Array.isArray(manifest.fileInventory) || manifest.fileInventory.length === 0) {
      missing.push("manifest 缺少 fileInventory");
    } else {
      const inventoryHash = createHash("sha256")
        .update(JSON.stringify(manifest.fileInventory))
        .digest("hex");
      if (manifest.fileInventorySha256 !== inventoryHash) {
        missing.push("manifest fileInventorySha256 必须匹配 fileInventory");
      }
      for (const item of manifest.fileInventory) {
        const path = join(root, item.path);
        if (!item.path?.startsWith("docs/acceptance-package/")) {
          missing.push(`fileInventory 路径必须位于 docs/acceptance-package：${item.path}`);
          continue;
        }
        if (!existsSync(path)) {
          missing.push(`fileInventory 文件不存在：${item.path}`);
          continue;
        }
        const stat = statSync(path);
        if (stat.size !== item.bytes) {
          missing.push(`fileInventory 文件大小不匹配：${item.path}`);
        }
        if (sha256(path) !== item.sha256) {
          missing.push(`fileInventory SHA256 不匹配：${item.path}`);
        }
      }
    }
    const copiedEvidence = new Set((manifest.copiedEvidence ?? []).map(normalizeManifestPath));
    for (const prefix of evidencePrefixes) {
      const latestName = latestEvidenceName(prefix);
      if (latestName && !copiedEvidence.has(`docs/acceptance-package/release-evidence/${latestName}`)) {
        missing.push(`manifest 未记录当前最新证据：${latestName}`);
      }
    }
    const releaseName = latestEvidenceName(`verify-release-${evidencePlatform}-`);
    const releaseReport = releaseName ? readJson(join(packageDir, "release-evidence", releaseName)) : null;
    if (!releaseReport || releaseReport.status !== "passed") {
      missing.push("verify-release 最新证据状态必须为 passed");
    } else {
      const failedCommands = (releaseReport.commandResults ?? []).filter((item) => item?.status !== "passed");
      if (failedCommands.length > 0) {
        missing.push("verify-release 最新证据不能包含失败命令");
      }
      const commands = Array.isArray(releaseReport.commands) ? releaseReport.commands : [];
      const commandResults = Array.isArray(releaseReport.commandResults) ? releaseReport.commandResults : [];
      for (const command of requiredReleaseCommands) {
        const matchesCommand = commandMatchers[command] ?? ((candidate) => candidate === command);
        const recordedCommand = commands.find(matchesCommand);
        if (!recordedCommand) {
          missing.push(`verify-release 最新证据缺少命令记录：${command}`);
        }
        const result = commandResults.find((item) => matchesCommand(item?.command));
        if (!result) {
          missing.push(`verify-release 最新证据缺少命令执行结果：${command}`);
        } else if (result.status !== "passed" || result.exitCode !== 0) {
          missing.push(`verify-release 最新证据命令未通过：${command}`);
        }
      }
    }
    const allLocalName = latestEvidenceName(`verify-all-local-${evidencePlatform}-`);
    const allLocalReport = allLocalName ? readJson(join(packageDir, "release-evidence", allLocalName)) : null;
    if (!allLocalReport || allLocalReport.status !== "passed") {
      missing.push("verify-all-local 最新证据状态必须为 passed");
    } else {
      const commands = Array.isArray(allLocalReport.commands) ? allLocalReport.commands : [];
      const commandResults = Array.isArray(allLocalReport.commandResults) ? allLocalReport.commandResults : [];
      for (const command of [
        "npm run build",
        "npm run test:manual-acceptance",
        "npm run verify:no-placeholders",
        "npm run verify:coverage",
        "cargo fmt --check",
        "cargo test",
        "npm run verify:release",
        "npm run acceptance:package",
        "npm run verify:readiness",
      ]) {
        if (!commands.includes(command)) {
          missing.push(`verify-all-local 最新证据缺少命令记录：${command}`);
        }
        const result = commandResults.find((item) => item?.command === command);
        if (!result) {
          missing.push(`verify-all-local 最新证据缺少命令执行结果：${command}`);
        } else if (result.status !== "passed" || result.exitCode !== 0) {
          missing.push(`verify-all-local 最新证据命令未通过：${command}`);
        }
      }
    }
    const readinessName = latestEvidenceName(`readiness-${evidencePlatform}-`);
    const readinessReport = readinessName ? readJson(join(packageDir, "release-evidence", readinessName)) : null;
    if (!readinessReport || !["ready-for-manual-evidence", "ready-for-final-archive"].includes(readinessReport.status)) {
      missing.push("readiness 最新证据状态必须为 ready-for-manual-evidence 或 ready-for-final-archive");
    } else if ((readinessReport.blockers ?? []).length > 0) {
      missing.push("readiness 最新证据 blockers 必须为空");
    }
    const coverageName = latestEvidenceName(`execution-coverage-${evidencePlatform}-`);
    const coverageReport = coverageName ? readJson(join(packageDir, "release-evidence", coverageName)) : null;
    if (!coverageReport || coverageReport.coverageStatus !== "covered") {
      missing.push("execution-coverage 最新证据状态必须为 covered");
    } else if ((coverageReport.features ?? []).some((feature) => feature?.status !== "covered")) {
      missing.push("execution-coverage 最新证据所有功能组必须为 covered");
    }
    const noPlaceholderName = latestEvidenceName(`no-placeholder-scan-${evidencePlatform}-`);
    const noPlaceholderReport = noPlaceholderName ? readJson(join(packageDir, "release-evidence", noPlaceholderName)) : null;
    if (!noPlaceholderReport || noPlaceholderReport.status !== "passed") {
      missing.push("no-placeholder-scan 最新证据状态必须为 passed");
    } else if ((noPlaceholderReport.findings ?? []).length > 0) {
      missing.push("no-placeholder-scan 最新证据 findings 必须为空");
    }
    const summaryName = latestEvidenceName(`manual-acceptance-summary-${evidencePlatform}-`);
    const summaryPath = summaryName ? join(packageDir, "release-evidence", summaryName) : null;
    const summary = summaryPath && existsSync(summaryPath) ? readJson(summaryPath) : null;
    if (!manifest.remainingEvidence) {
      missing.push("manifest 缺少 remainingEvidence");
    } else if (!summary?.remainingEvidence) {
      missing.push("manual acceptance summary 缺少 remainingEvidence");
    } else {
      if (manifest.remainingEvidence.status !== summary.remainingEvidence.status) {
        missing.push("manifest remainingEvidence.status 必须匹配最新 summary");
      }
      if (manifest.remainingEvidence.remainingCount !== summary.remainingEvidence.remainingCount) {
        missing.push("manifest remainingEvidence.remainingCount 必须匹配最新 summary");
      }
      const remainingReadmePath = join(packageDir, "REMAINING_EVIDENCE.md");
      const remainingReadme = existsSync(remainingReadmePath) ? readFileSync(remainingReadmePath, "utf8") : "";
      if (!remainingReadme.includes(`剩余证据组数：${summary.remainingEvidence.remainingCount}`)) {
        missing.push("REMAINING_EVIDENCE.md 剩余证据组数必须匹配最新 summary");
      }
      for (const item of summary.remainingEvidence.items ?? []) {
        if (!remainingReadme.includes(item.label)) {
          missing.push(`REMAINING_EVIDENCE.md 缺少剩余证据分组：${item.label}`);
        }
      }
    }
    const copiedManualAcceptance = new Set(
      (manifest.copiedManualAcceptance ?? []).map(normalizeManifestPath),
    );
    const manualRecordPath = latestManualAcceptanceRelativePath(`manual-acceptance-${manualPlatform}-`, ".json");
    const manualChecklistPath = latestManualAcceptanceRelativePath(`manual-acceptance-${manualPlatform}-`, "-checklist.md");
    const manualAttachmentReadme = latestManualAcceptanceRelativePath(`evidence-${manualPlatform}-`, "README.md");
    for (const expected of [
      manualRecordPath,
      manualChecklistPath,
      manualAttachmentReadme,
    ].filter(Boolean)) {
      const packageRelativePath = `docs/acceptance-package/manual-acceptance/${expected}`;
      if (!copiedManualAcceptance.has(packageRelativePath)) {
        missing.push(`manifest 未记录当前手工验收样例：${expected}`);
      }
      if (!existsSync(join(root, packageRelativePath))) {
        missing.push(`manual-acceptance 缺少当前手工验收样例：${expected}`);
      }
      if (!packageReadme.includes(`manual-acceptance/${expected.replaceAll("\\", "/")}`)) {
        missing.push(`README.md 未列出当前手工验收样例：${expected}`);
      }
    }
    if (!manualRecordPath || !manualChecklistPath || !manualAttachmentReadme) {
      missing.push(`缺少 ${manualPlatform} 当前手工验收样例，请先运行 npm run acceptance:collect -- ${manualPlatform} --force`);
    }
    for (const relativePath of copiedDocSources) {
      const sourcePath = join(root, relativePath);
      const packagePath = join(packageDir, "docs", relativePath.replaceAll("/", "__"));
      if (existsSync(sourcePath) && existsSync(packagePath) && readFileSync(sourcePath, "utf8") !== readFileSync(packagePath, "utf8")) {
        missing.push(`交接包文档不是最新：${relativePath}`);
      }
    }
    for (const relativePath of [manualRecordPath, manualChecklistPath, manualAttachmentReadme].filter(Boolean)) {
      const sourcePath = join(root, "docs", "manual-acceptance", relativePath);
      const packagePath = join(packageDir, "manual-acceptance", relativePath);
      if (existsSync(sourcePath) && existsSync(packagePath) && readFileSync(sourcePath, "utf8") !== readFileSync(packagePath, "utf8")) {
        missing.push(`交接包手工验收样例不是最新：${relativePath}`);
      }
    }
  }
}

if (missing.length > 0) {
  console.error("[verify-acceptance-package] Acceptance package is incomplete:");
  for (const item of missing) {
    console.error(`- ${item}`);
  }
  process.exit(1);
}

console.log("[verify-acceptance-package] Acceptance package contains required docs, runners, and evidence.");
