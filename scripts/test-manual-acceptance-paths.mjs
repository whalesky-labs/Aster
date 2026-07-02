import { copyFileSync, existsSync, mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { createHash } from "node:crypto";
import http from "node:http";

const root = process.cwd();
const tempRoot = mkdtempSync(join(tmpdir(), "aster-manual-acceptance-"));
const acceptanceDir = join(tempRoot, "docs", "manual-acceptance");
const evidenceDir = join(tempRoot, "docs", "release-evidence");
mkdirSync(acceptanceDir, { recursive: true });
mkdirSync(evidenceDir, { recursive: true });

function writeJson(path, data) {
  writeFileSync(path, `${JSON.stringify(data, null, 2)}\n`);
}

function spawnNodeAsync(args, options) {
  return new Promise((resolveProcess) => {
    const child = spawn("node", args, {
      ...options,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.on("close", (status, signal) => {
      resolveProcess({
        status,
        signal,
        stdout,
        stderr,
      });
    });
  });
}

function latestSummary(dir) {
  const files = readdirSync(dir)
    .filter((name) => name.startsWith("manual-acceptance-summary-") && name.endsWith(".json"))
    .map((name) => join(dir, name))
    .sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs || b.localeCompare(a));
  return files[0] ? JSON.parse(readFileSync(files[0], "utf8")) : null;
}

function evidence(prefix, data) {
  const path = join(evidenceDir, `${prefix}-placeholder.json`);
  writeJson(path, data);
  return path;
}

const sha = "a".repeat(64);
const featureIds = [
  "workspace_dashboard",
  "master_data",
  "stock_lifecycle",
  "stocktake",
  "reports_export_print",
  "excel_import",
  "backup_disaster_recovery",
  "users_permissions",
  "host_client_consistency",
  "budget_approval",
  "cross_platform_packaging",
];
const requiredReleaseCommands = [
  "npm run build",
  "npm run verify:coverage",
  "npm run verify:no-placeholders",
  "cargo fmt --check",
  "cargo test",
  "npm run tauri -- build",
];
const requiredAllLocalCommands = [
  "npm run build",
  "npm run test:manual-acceptance",
  "npm run verify:no-placeholders",
  "npm run verify:coverage",
  "cargo fmt --check",
  "cargo test",
  "npm run verify:release",
  "npm run acceptance:package",
  "npm run verify:acceptance-package",
  "npm run verify:readiness",
];

function releaseEvidence(platform) {
  const commands =
    platform === "win32"
      ? requiredReleaseCommands.map((command) =>
          command === "npm run tauri -- build"
            ? "npm run tauri -- build --target x86_64-pc-windows-msvc"
            : command,
        )
      : requiredReleaseCommands;
  return {
    platform,
    status: "passed",
    tauriConfig: {
      version: "0.1.0",
      buildTarget: platform === "win32" ? "x86_64-pc-windows-msvc" : null,
    },
    commands,
    commandResults: commands.map((command, index) => ({
      command,
      cwd: platform === "win32" ? "C:\\Aster" : "/tmp/Aster",
      status: "passed",
      exitCode: 0,
      signal: null,
      startedAt: `2026-06-30T10:0${index}:00.000Z`,
      finishedAt: `2026-06-30T10:0${index}:01.000Z`,
      durationMs: 1000,
    })),
    artifacts: [
      {
        path: platform === "win32" ? "C:\\Aster\\AsterSetup.exe" : "/Applications/Aster.app",
        type: "file",
        sha256: sha
      }
    ]
  };
}

function coverageEvidence(platform) {
  return {
    platform,
    coverageStatus: "covered",
    features: featureIds.map((id) => ({
      id,
      status: "covered"
    }))
  };
}

function noPlaceholderEvidence(platform) {
  return {
    generatedAt: "2026-06-30T12:00:00.000Z",
    platform,
    status: "passed",
    findings: [],
  };
}

function allLocalEvidence(platform) {
  return {
    generatedAt: "2026-06-30T12:00:00.000Z",
    platform,
    status: "passed",
    commands: requiredAllLocalCommands,
    commandResults: requiredAllLocalCommands.map((command, index) => ({
      command,
      cwd: platform === "win32" ? "C:\\Aster" : "/tmp/Aster",
      status: "passed",
      exitCode: 0,
      signal: null,
      startedAt: `2026-06-30T11:0${index}:00.000Z`,
      finishedAt: `2026-06-30T11:0${index}:01.000Z`,
      durationMs: 1000,
    })),
  };
}

function readinessEvidence(platform) {
  return {
    generatedAt: "2026-06-30T12:00:00.000Z",
    platform,
    status: "ready-for-manual-evidence",
    evidencePlatform: platform,
    blockers: [],
    remainingEvidence: {
      status: "missing-evidence",
      remainingCount: 1,
      items: [
        {
          id: `${platform}_fixture_evidence`,
          label: `${platform} 夹具剩余证据`,
          owner: platform,
          status: "missing-evidence",
          missing: ["夹具用于验证 readiness 透传"],
        },
      ],
    },
  };
}

evidence("verify-release-win32", releaseEvidence("win32"));
evidence("execution-coverage-win32", coverageEvidence("win32"));
evidence("no-placeholder-scan-win32", noPlaceholderEvidence("win32"));
evidence("verify-all-local-win32", allLocalEvidence("win32"));
evidence("readiness-win32", readinessEvidence("win32"));
evidence("verify-release-darwin", releaseEvidence("darwin"));
evidence("execution-coverage-darwin", coverageEvidence("darwin"));
evidence("no-placeholder-scan-darwin", noPlaceholderEvidence("darwin"));
evidence("verify-all-local-darwin", allLocalEvidence("darwin"));
evidence("readiness-darwin", readinessEvidence("darwin"));

const baseRecord = {
  schemaVersion: 1,
  generatedAt: "2026-06-30T10:00:00.000Z",
  machine: {
    operator: "tester",
    computerName: "machine",
    osVersion: "test-os",
    cpuArch: "x64",
    appVersion: "0.1.0"
  },
  releaseEvidence: {
    verifyReleaseReport: "",
    executionCoverageReport: "",
    installerPath: "C:\\Aster\\AsterSetup.exe",
    installerSha256: sha
  },
  localInstall: {
    installerGenerated: true,
    installerOpenedOrInstalled: true,
    firstLaunchOk: true,
    defaultAdminLoginOk: true,
    databaseCreatedInSystemAppDataDir: true,
    appDataDir: "C:\\Users\\tester\\AppData\\Roaming\\Aster"
  },
  excel: {
    importPreviewReadLegacyWorkbook: true,
    importRunCreatedRecords: true,
    monthlyReportExported: true,
    exportedWorkbookOpenedWithoutOfficeOrWpsDependency: true,
    exportedWorkbookPath: "C:\\Aster\\reports\\monthly.xlsx"
  },
  backup: {
    manualBackupCreated: true,
    secondBackupDirWritable: true,
    backupZipContainsRequiredEntries: true,
    backupFile: "C:\\Aster\\backups\\aster-backup.zip",
    sourceHostName: "windows-host",
    sourceOs: "windows",
    backupSha256: sha
  },
  restore: {
    beforeRestoreBackupCreated: true,
    beforeRestoreBackupFile: "C:\\Aster\\backups\\before-restore.zip",
    restoredBackupFromOtherPlatform: true,
    restoredFromPlatform: "macos",
    restoredBackupFile: "C:\\Aster\\restore\\macos-backup.zip",
    restoredBackupSourceHostName: "macos-host",
    healthCheckOkAfterRestore: true,
    dataConsistentAfterRestore: true
  },
  evidenceFiles: {
    installScreenshot: "C:\\Aster\\evidence\\install.png",
    loginScreenshot: "C:\\Aster\\evidence\\login.png",
    databaseLocationScreenshot: "C:\\Aster\\evidence\\database-location.png",
    excelImportPreviewScreenshot: "C:\\Aster\\evidence\\excel-import-preview.png",
    exportedWorkbookScreenshot: "C:\\Aster\\evidence\\exported-workbook.png",
    backupRecordScreenshot: "C:\\Aster\\evidence\\backup-record.png",
    restorePreviewScreenshot: "C:\\Aster\\evidence\\restore-preview.png",
    restoreResultScreenshot: "C:\\Aster\\evidence\\restore-result.png",
    hostModeScreenshot: "C:\\Aster\\evidence\\host-mode.png",
    clientModeScreenshot: "C:\\Aster\\evidence\\client-mode.png"
  },
  sampleData: {
    importedWorkbookName: "legacy-hotel.xlsx",
    exportedReportMonth: "2026-06",
    backupRecordId: "backup-001",
    restoredItemCount: 12,
    restoredMovementCount: 34,
    asHostBusinessDocumentNo: "OUT-20260630-001",
    asClientBusinessDocumentNo: "IN-20260630-001"
  },
  hostClient: {
    asHost: {
      tested: true,
      peerPlatform: "macos",
      hostIp: "192.168.1.10",
      hostPort: 17871,
      pairCodeDisplayed: true,
      peerPaired: true,
      disconnectDetectedByPeer: true,
      reconnectDetectedByPeer: true,
      peerBusinessWriteSucceeded: true,
      hostInventoryAndReportsConsistent: true
    },
    asClient: {
      tested: true,
      peerPlatform: "macos",
      connectedByDiscoveryOrManualAddress: true,
      paired: true,
      disconnectDetected: true,
      reconnectDetected: true,
      clientBusinessWriteSucceeded: true,
      hostInventoryAndReportsConsistent: true
    }
  }
};

function macosRecord(overrides = {}) {
  return {
    ...baseRecord,
    generatedAt: "2026-06-30T11:00:00.000Z",
    platform: "macos",
    releaseEvidence: {
      ...baseRecord.releaseEvidence,
      installerPath: "/Applications/Aster.app"
    },
    localInstall: {
      ...baseRecord.localInstall,
      appDataDir: "/Users/tester/Library/Application Support/Aster"
    },
    excel: {
      ...baseRecord.excel,
      exportedWorkbookPath: "/tmp/monthly.xlsx"
    },
    backup: {
      ...baseRecord.backup,
      backupFile: "/tmp/aster-backup.zip",
      sourceHostName: "macos-host",
      sourceOs: "macos"
    },
    restore: {
      ...baseRecord.restore,
      beforeRestoreBackupFile: "/tmp/before-restore.zip",
      restoredFromPlatform: "windows",
      restoredBackupFile: "/tmp/windows-backup.zip",
      restoredBackupSourceHostName: "windows-host"
    },
    evidenceFiles: {
      installScreenshot: "/tmp/evidence/install.png",
      loginScreenshot: "/tmp/evidence/login.png",
      databaseLocationScreenshot: "/tmp/evidence/database-location.png",
      excelImportPreviewScreenshot: "/tmp/evidence/excel-import-preview.png",
      exportedWorkbookScreenshot: "/tmp/evidence/exported-workbook.png",
      backupRecordScreenshot: "/tmp/evidence/backup-record.png",
      restorePreviewScreenshot: "/tmp/evidence/restore-preview.png",
      restoreResultScreenshot: "/tmp/evidence/restore-result.png",
      hostModeScreenshot: "/tmp/evidence/host-mode.png",
      clientModeScreenshot: "/tmp/evidence/client-mode.png"
    },
    sampleData: {
      ...baseRecord.sampleData,
      asHostBusinessDocumentNo: "IN-20260630-002",
      asClientBusinessDocumentNo: "OUT-20260630-002"
    },
    hostClient: {
      asHost: {
        ...baseRecord.hostClient.asHost,
        peerPlatform: "windows"
      },
      asClient: {
        ...baseRecord.hostClient.asClient,
        peerPlatform: "windows"
      }
    },
    ...overrides,
  };
}

writeJson(join(acceptanceDir, "manual-acceptance-windows-older-empty.json"), {
  schemaVersion: 1,
  generatedAt: "2026-06-29T10:00:00.000Z",
  platform: "windows",
  machine: {
    operator: "",
    computerName: "",
    osVersion: "",
    cpuArch: "",
    appVersion: ""
  },
  releaseEvidence: {
    verifyReleaseReport: "",
    executionCoverageReport: "",
    installerPath: "",
    installerSha256: ""
  },
  localInstall: {},
  excel: {},
  backup: {},
  restore: {},
  evidenceFiles: {},
  sampleData: {},
  hostClient: {
    asHost: {},
    asClient: {}
  }
});

writeJson(join(acceptanceDir, "manual-acceptance-windows-test.json"), {
  ...baseRecord,
  generatedAt: "2026-06-30T11:00:00.000Z",
  platform: "windows",
});
writeJson(join(acceptanceDir, "manual-acceptance-macos-test.json"), macosRecord());

const result = spawnSync("node", [join(root, "scripts", "verify-manual-acceptance.mjs"), "--strict"], {
  cwd: tempRoot,
  encoding: "utf8",
});

if (result.status !== 0) {
  console.error(result.stdout);
  console.error(result.stderr);
  process.exit(result.status ?? 1);
}

writeJson(join(acceptanceDir, "manual-acceptance-windows-mismatch.json"), {
  ...baseRecord,
  platform: "macos",
});

const mismatchResult = spawnSync("node", [join(root, "scripts", "verify-manual-acceptance.mjs"), "--strict"], {
  cwd: tempRoot,
  encoding: "utf8",
});

if (mismatchResult.status === 0 || !mismatchResult.stdout.includes("platform-mismatch")) {
  console.error(mismatchResult.stdout);
  console.error(mismatchResult.stderr);
  throw new Error("Expected platform mismatch to fail strict manual acceptance verification.");
}

const missingCoverageRoot = mkdtempSync(join(tmpdir(), "aster-manual-acceptance-missing-coverage-"));
const missingCoverageAcceptanceDir = join(missingCoverageRoot, "docs", "manual-acceptance");
const missingCoverageEvidenceDir = join(missingCoverageRoot, "docs", "release-evidence");
mkdirSync(missingCoverageAcceptanceDir, { recursive: true });
mkdirSync(missingCoverageEvidenceDir, { recursive: true });
const releaseWithoutCoverage = releaseEvidence("win32");
releaseWithoutCoverage.commands = releaseWithoutCoverage.commands.filter(
  (command) => command !== "npm run verify:coverage",
);
releaseWithoutCoverage.commandResults = releaseWithoutCoverage.commandResults.filter(
  (result) => result.command !== "npm run verify:coverage",
);
writeJson(join(missingCoverageEvidenceDir, "verify-release-win32-placeholder.json"), releaseWithoutCoverage);
writeJson(join(missingCoverageEvidenceDir, "execution-coverage-win32-placeholder.json"), coverageEvidence("win32"));
writeJson(join(missingCoverageEvidenceDir, "verify-release-darwin-placeholder.json"), releaseEvidence("darwin"));
writeJson(join(missingCoverageEvidenceDir, "execution-coverage-darwin-placeholder.json"), coverageEvidence("darwin"));
writeJson(join(missingCoverageAcceptanceDir, "manual-acceptance-windows-test.json"), {
  ...baseRecord,
  generatedAt: "2026-06-30T11:00:00.000Z",
  platform: "windows",
});
writeJson(join(missingCoverageAcceptanceDir, "manual-acceptance-macos-test.json"), macosRecord());
const missingCoverageResult = spawnSync("node", [join(root, "scripts", "verify-manual-acceptance.mjs"), "--strict"], {
  cwd: missingCoverageRoot,
  encoding: "utf8",
});
if (missingCoverageResult.status === 0 || !missingCoverageResult.stdout.includes("npm run verify:coverage")) {
  console.error(missingCoverageResult.stdout);
  console.error(missingCoverageResult.stderr);
  throw new Error("Expected missing verify:coverage command to fail strict manual acceptance verification.");
}

const missingNoPlaceholderRoot = mkdtempSync(join(tmpdir(), "aster-manual-acceptance-missing-no-placeholder-"));
const missingNoPlaceholderAcceptanceDir = join(missingNoPlaceholderRoot, "docs", "manual-acceptance");
const missingNoPlaceholderEvidenceDir = join(missingNoPlaceholderRoot, "docs", "release-evidence");
mkdirSync(missingNoPlaceholderAcceptanceDir, { recursive: true });
mkdirSync(missingNoPlaceholderEvidenceDir, { recursive: true });
const releaseWithoutNoPlaceholder = releaseEvidence("win32");
releaseWithoutNoPlaceholder.commands = releaseWithoutNoPlaceholder.commands.filter(
  (command) => command !== "npm run verify:no-placeholders",
);
releaseWithoutNoPlaceholder.commandResults = releaseWithoutNoPlaceholder.commandResults.filter(
  (result) => result.command !== "npm run verify:no-placeholders",
);
writeJson(join(missingNoPlaceholderEvidenceDir, "verify-release-win32-placeholder.json"), releaseWithoutNoPlaceholder);
writeJson(join(missingNoPlaceholderEvidenceDir, "execution-coverage-win32-placeholder.json"), coverageEvidence("win32"));
writeJson(join(missingNoPlaceholderEvidenceDir, "verify-release-darwin-placeholder.json"), releaseEvidence("darwin"));
writeJson(join(missingNoPlaceholderEvidenceDir, "execution-coverage-darwin-placeholder.json"), coverageEvidence("darwin"));
writeJson(join(missingNoPlaceholderAcceptanceDir, "manual-acceptance-windows-test.json"), {
  ...baseRecord,
  generatedAt: "2026-06-30T11:00:00.000Z",
  platform: "windows",
});
writeJson(join(missingNoPlaceholderAcceptanceDir, "manual-acceptance-macos-test.json"), macosRecord());
const missingNoPlaceholderResult = spawnSync("node", [join(root, "scripts", "verify-manual-acceptance.mjs"), "--strict"], {
  cwd: missingNoPlaceholderRoot,
  encoding: "utf8",
});
if (missingNoPlaceholderResult.status === 0 || !missingNoPlaceholderResult.stdout.includes("npm run verify:no-placeholders")) {
  console.error(missingNoPlaceholderResult.stdout);
  console.error(missingNoPlaceholderResult.stderr);
  throw new Error("Expected missing verify:no-placeholders command to fail strict manual acceptance verification.");
}

const failedCommandRoot = mkdtempSync(join(tmpdir(), "aster-manual-acceptance-failed-command-"));
const failedCommandAcceptanceDir = join(failedCommandRoot, "docs", "manual-acceptance");
const failedCommandEvidenceDir = join(failedCommandRoot, "docs", "release-evidence");
mkdirSync(failedCommandAcceptanceDir, { recursive: true });
mkdirSync(failedCommandEvidenceDir, { recursive: true });
const releaseWithFailedCommand = releaseEvidence("win32");
releaseWithFailedCommand.commandResults = releaseWithFailedCommand.commandResults.map((result) =>
  result.command === "cargo test"
    ? {
        ...result,
        status: "failed",
        exitCode: 1,
      }
    : result,
);
writeJson(join(failedCommandEvidenceDir, "verify-release-win32-placeholder.json"), releaseWithFailedCommand);
writeJson(join(failedCommandEvidenceDir, "execution-coverage-win32-placeholder.json"), coverageEvidence("win32"));
writeJson(join(failedCommandEvidenceDir, "verify-release-darwin-placeholder.json"), releaseEvidence("darwin"));
writeJson(join(failedCommandEvidenceDir, "execution-coverage-darwin-placeholder.json"), coverageEvidence("darwin"));
writeJson(join(failedCommandAcceptanceDir, "manual-acceptance-windows-test.json"), {
  ...baseRecord,
  generatedAt: "2026-06-30T11:00:00.000Z",
  platform: "windows",
});
writeJson(join(failedCommandAcceptanceDir, "manual-acceptance-macos-test.json"), macosRecord());
const failedCommandResult = spawnSync("node", [join(root, "scripts", "verify-manual-acceptance.mjs"), "--strict"], {
  cwd: failedCommandRoot,
  encoding: "utf8",
});
if (failedCommandResult.status === 0 || !failedCommandResult.stdout.includes("verify-release 命令执行未成功：cargo test")) {
  console.error(failedCommandResult.stdout);
  console.error(failedCommandResult.stderr);
  throw new Error("Expected failed release command result to fail strict manual acceptance verification.");
}

const missingFieldEvidenceRoot = mkdtempSync(join(tmpdir(), "aster-manual-acceptance-missing-field-evidence-"));
const missingFieldEvidenceAcceptanceDir = join(missingFieldEvidenceRoot, "docs", "manual-acceptance");
const missingFieldEvidenceEvidenceDir = join(missingFieldEvidenceRoot, "docs", "release-evidence");
mkdirSync(missingFieldEvidenceAcceptanceDir, { recursive: true });
mkdirSync(missingFieldEvidenceEvidenceDir, { recursive: true });
writeJson(join(missingFieldEvidenceEvidenceDir, "verify-release-win32-placeholder.json"), releaseEvidence("win32"));
writeJson(join(missingFieldEvidenceEvidenceDir, "execution-coverage-win32-placeholder.json"), coverageEvidence("win32"));
writeJson(join(missingFieldEvidenceEvidenceDir, "verify-release-darwin-placeholder.json"), releaseEvidence("darwin"));
writeJson(join(missingFieldEvidenceEvidenceDir, "execution-coverage-darwin-placeholder.json"), coverageEvidence("darwin"));
writeJson(join(missingFieldEvidenceAcceptanceDir, "manual-acceptance-windows-test.json"), {
  ...baseRecord,
  generatedAt: "2026-06-30T11:00:00.000Z",
  platform: "windows",
  evidenceFiles: {
    ...baseRecord.evidenceFiles,
    loginScreenshot: ""
  },
  sampleData: {
    ...baseRecord.sampleData,
    restoredMovementCount: null
  }
});
writeJson(join(missingFieldEvidenceAcceptanceDir, "manual-acceptance-macos-test.json"), macosRecord());
const missingFieldEvidenceResult = spawnSync(
  "node",
  [join(root, "scripts", "verify-manual-acceptance.mjs"), "--strict"],
  {
    cwd: missingFieldEvidenceRoot,
    encoding: "utf8",
  },
);
if (
  missingFieldEvidenceResult.status === 0 ||
  !missingFieldEvidenceResult.stdout.includes("默认管理员登录截图不能为空") ||
  !missingFieldEvidenceResult.stdout.includes("恢复后库存流水数量必须为非负整数")
) {
  console.error(missingFieldEvidenceResult.stdout);
  console.error(missingFieldEvidenceResult.stderr);
  throw new Error("Expected missing manual evidence fields to fail strict manual acceptance verification.");
}
const missingFieldEvidenceSummary = latestSummary(missingFieldEvidenceEvidenceDir);
if (
  missingFieldEvidenceSummary?.remainingEvidence?.status !== "missing-evidence" ||
  !missingFieldEvidenceSummary.remainingEvidence.items.some((item) => item.id === "windows_screenshots") ||
  !missingFieldEvidenceSummary.remainingEvidence.items.some((item) => item.id === "windows_backup_restore")
) {
  console.error(JSON.stringify(missingFieldEvidenceSummary?.remainingEvidence, null, 2));
  throw new Error("Expected manual acceptance summary to include grouped remaining evidence.");
}

const missingExcelImportRunRoot = mkdtempSync(join(tmpdir(), "aster-manual-acceptance-missing-excel-import-run-"));
const missingExcelImportRunAcceptanceDir = join(missingExcelImportRunRoot, "docs", "manual-acceptance");
const missingExcelImportRunEvidenceDir = join(missingExcelImportRunRoot, "docs", "release-evidence");
mkdirSync(missingExcelImportRunAcceptanceDir, { recursive: true });
mkdirSync(missingExcelImportRunEvidenceDir, { recursive: true });
writeJson(join(missingExcelImportRunEvidenceDir, "verify-release-win32-placeholder.json"), releaseEvidence("win32"));
writeJson(join(missingExcelImportRunEvidenceDir, "execution-coverage-win32-placeholder.json"), coverageEvidence("win32"));
writeJson(join(missingExcelImportRunEvidenceDir, "verify-release-darwin-placeholder.json"), releaseEvidence("darwin"));
writeJson(join(missingExcelImportRunEvidenceDir, "execution-coverage-darwin-placeholder.json"), coverageEvidence("darwin"));
writeJson(join(missingExcelImportRunAcceptanceDir, "manual-acceptance-windows-test.json"), {
  ...baseRecord,
  generatedAt: "2026-06-30T11:00:00.000Z",
  platform: "windows",
  excel: {
    ...baseRecord.excel,
    importRunCreatedRecords: false
  }
});
writeJson(join(missingExcelImportRunAcceptanceDir, "manual-acceptance-macos-test.json"), macosRecord());
const missingExcelImportRunResult = spawnSync(
  "node",
  [join(root, "scripts", "verify-manual-acceptance.mjs"), "--strict"],
  {
    cwd: missingExcelImportRunRoot,
    encoding: "utf8",
  },
);
if (
  missingExcelImportRunResult.status === 0 ||
  !missingExcelImportRunResult.stdout.includes("Excel 正式导入已生成记录")
) {
  console.error(missingExcelImportRunResult.stdout);
  console.error(missingExcelImportRunResult.stderr);
  throw new Error("Expected missing Excel import run evidence to fail strict manual acceptance verification.");
}

const mismatchedInstallerRoot = mkdtempSync(join(tmpdir(), "aster-manual-acceptance-mismatched-installer-"));
const mismatchedInstallerAcceptanceDir = join(mismatchedInstallerRoot, "docs", "manual-acceptance");
const mismatchedInstallerEvidenceDir = join(mismatchedInstallerRoot, "docs", "release-evidence");
mkdirSync(mismatchedInstallerAcceptanceDir, { recursive: true });
mkdirSync(mismatchedInstallerEvidenceDir, { recursive: true });
writeJson(join(mismatchedInstallerEvidenceDir, "verify-release-win32-placeholder.json"), releaseEvidence("win32"));
writeJson(join(mismatchedInstallerEvidenceDir, "execution-coverage-win32-placeholder.json"), coverageEvidence("win32"));
writeJson(join(mismatchedInstallerEvidenceDir, "verify-release-darwin-placeholder.json"), releaseEvidence("darwin"));
writeJson(join(mismatchedInstallerEvidenceDir, "execution-coverage-darwin-placeholder.json"), coverageEvidence("darwin"));
writeJson(join(mismatchedInstallerAcceptanceDir, "manual-acceptance-windows-test.json"), {
  ...baseRecord,
  generatedAt: "2026-06-30T11:00:00.000Z",
  platform: "windows",
  releaseEvidence: {
    ...baseRecord.releaseEvidence,
    installerPath: "C:\\Aster\\WrongSetup.exe"
  }
});
writeJson(join(mismatchedInstallerAcceptanceDir, "manual-acceptance-macos-test.json"), macosRecord());
const mismatchedInstallerResult = spawnSync("node", [join(root, "scripts", "verify-manual-acceptance.mjs"), "--strict"], {
  cwd: mismatchedInstallerRoot,
  encoding: "utf8",
});
if (mismatchedInstallerResult.status === 0 || !mismatchedInstallerResult.stdout.includes("安装包路径必须匹配")) {
  console.error(mismatchedInstallerResult.stdout);
  console.error(mismatchedInstallerResult.stderr);
  throw new Error("Expected installer path mismatch to fail strict manual acceptance verification.");
}

const missingBeforeRestoreRoot = mkdtempSync(join(tmpdir(), "aster-manual-acceptance-missing-before-restore-"));
const missingBeforeRestoreAcceptanceDir = join(missingBeforeRestoreRoot, "docs", "manual-acceptance");
const missingBeforeRestoreEvidenceDir = join(missingBeforeRestoreRoot, "docs", "release-evidence");
mkdirSync(missingBeforeRestoreAcceptanceDir, { recursive: true });
mkdirSync(missingBeforeRestoreEvidenceDir, { recursive: true });
writeJson(join(missingBeforeRestoreEvidenceDir, "verify-release-win32-placeholder.json"), releaseEvidence("win32"));
writeJson(join(missingBeforeRestoreEvidenceDir, "execution-coverage-win32-placeholder.json"), coverageEvidence("win32"));
writeJson(join(missingBeforeRestoreEvidenceDir, "verify-release-darwin-placeholder.json"), releaseEvidence("darwin"));
writeJson(join(missingBeforeRestoreEvidenceDir, "execution-coverage-darwin-placeholder.json"), coverageEvidence("darwin"));
writeJson(join(missingBeforeRestoreAcceptanceDir, "manual-acceptance-windows-test.json"), {
  ...baseRecord,
  generatedAt: "2026-06-30T11:00:00.000Z",
  platform: "windows",
  restore: {
    ...baseRecord.restore,
    beforeRestoreBackupCreated: false,
    beforeRestoreBackupFile: ""
  }
});
writeJson(join(missingBeforeRestoreAcceptanceDir, "manual-acceptance-macos-test.json"), macosRecord());
const missingBeforeRestoreResult = spawnSync(
  "node",
  [join(root, "scripts", "verify-manual-acceptance.mjs"), "--strict"],
  {
    cwd: missingBeforeRestoreRoot,
    encoding: "utf8",
  },
);
if (
  missingBeforeRestoreResult.status === 0 ||
  !missingBeforeRestoreResult.stdout.includes("恢复前保护备份已生成") ||
  !missingBeforeRestoreResult.stdout.includes("恢复前保护备份路径不能为空")
) {
  console.error(missingBeforeRestoreResult.stdout);
  console.error(missingBeforeRestoreResult.stderr);
  throw new Error("Expected missing before_restore backup evidence to fail strict manual acceptance verification.");
}

function manualSummary(platformName) {
  return {
    generatedAt: "2026-06-30T12:00:00.000Z",
    platform: platformName,
    status: "incomplete",
    remainingEvidence: {
      status: "missing-evidence",
      remainingCount: 1,
      items: [
        {
          id: `${platformName}_fixture_evidence`,
          label: `${platformName} 夹具剩余证据`,
          owner: platformName,
          status: "missing-evidence",
          missing: ["夹具用于验证交接包 remainingEvidence 透传"],
        },
      ],
    },
    platformSummaries: [],
    hostClientSummaries: [],
    findings: [],
  };
}

function writeText(path, value) {
  writeFileSync(path, value);
}

function setupAcceptancePackageFixture(platformName) {
  const fixtureRoot = mkdtempSync(join(tmpdir(), `aster-acceptance-package-${platformName}-`));
  const fixtureScriptsDir = join(fixtureRoot, "scripts");
  const fixtureDocsDir = join(fixtureRoot, "docs");
  const fixtureEvidenceDir = join(fixtureDocsDir, "release-evidence");
  const fixtureManualDir = join(fixtureDocsDir, "manual-acceptance");
  mkdirSync(fixtureScriptsDir, { recursive: true });
  mkdirSync(fixtureEvidenceDir, { recursive: true });
  mkdirSync(fixtureManualDir, { recursive: true });

  for (const script of [
    "create-acceptance-package.mjs",
    "verify-acceptance-package.mjs",
    "create-acceptance-runners.mjs",
  ]) {
    copyFileSync(join(root, "scripts", script), join(fixtureScriptsDir, script));
  }
  for (const relativePath of [
    "README.md",
    "docs/ASTER_EXECUTION_PLAN.md",
    "docs/ASTER_CROSS_PLATFORM_ACCEPTANCE.md",
    "docs/ASTER_DISASTER_RECOVERY_RUNBOOK.md",
    "docs/manual-acceptance/README.md",
  ]) {
    const target = join(fixtureRoot, relativePath);
    mkdirSync(join(target, ".."), { recursive: true });
    writeText(target, readFileSync(join(root, relativePath), "utf8"));
  }

  for (const candidatePlatform of ["win32", "darwin"]) {
    evidence(`verify-all-local-${candidatePlatform}`, allLocalEvidence(candidatePlatform));
    evidence(`verify-release-${candidatePlatform}`, releaseEvidence(candidatePlatform));
    evidence(`execution-coverage-${candidatePlatform}`, coverageEvidence(candidatePlatform));
    evidence(`no-placeholder-scan-${candidatePlatform}`, noPlaceholderEvidence(candidatePlatform));
    evidence(`readiness-${candidatePlatform}`, readinessEvidence(candidatePlatform));
    evidence(`manual-acceptance-summary-${candidatePlatform}`, manualSummary(candidatePlatform));
  }
  for (const file of [
    "verify-release-win32-placeholder.json",
    "verify-all-local-win32-placeholder.json",
    "execution-coverage-win32-placeholder.json",
    "no-placeholder-scan-win32-placeholder.json",
    "readiness-win32-placeholder.json",
    "manual-acceptance-summary-win32-placeholder.json",
    "verify-release-darwin-placeholder.json",
    "verify-all-local-darwin-placeholder.json",
    "execution-coverage-darwin-placeholder.json",
    "no-placeholder-scan-darwin-placeholder.json",
    "readiness-darwin-placeholder.json",
    "manual-acceptance-summary-darwin-placeholder.json",
  ]) {
    copyFileSync(join(evidenceDir, file), join(fixtureEvidenceDir, file));
  }
  const expectedManualPlatform = platformName === "win32" ? "windows" : "macos";
  const fixtureManualDate = "2026-06-30";
  writeJson(join(fixtureManualDir, `manual-acceptance-${expectedManualPlatform}-${fixtureManualDate}.json`), {
    schemaVersion: 1,
    platform: expectedManualPlatform,
    generatedAt: "2026-06-30T12:00:00.000Z",
  });
  writeText(
    join(fixtureManualDir, `manual-acceptance-${expectedManualPlatform}-${fixtureManualDate}-checklist.md`),
    `# ${expectedManualPlatform} checklist\n`,
  );
  const fixtureAttachmentDir = join(fixtureManualDir, `evidence-${expectedManualPlatform}-${fixtureManualDate}`);
  mkdirSync(fixtureAttachmentDir, { recursive: true });
  writeText(join(fixtureAttachmentDir, "README.md"), `# ${expectedManualPlatform} evidence\n`);

  const result = spawnSync("node", [join(fixtureScriptsDir, "create-acceptance-package.mjs")], {
    cwd: fixtureRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      ASTER_ACCEPTANCE_PLATFORM: platformName,
    },
  });
  if (result.status !== 0) {
    console.error(result.stdout);
    console.error(result.stderr);
    throw new Error(`Expected acceptance package creation to pass for ${platformName}.`);
  }
  const verifyResult = spawnSync("node", [join(fixtureScriptsDir, "verify-acceptance-package.mjs")], {
    cwd: fixtureRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      ASTER_ACCEPTANCE_PLATFORM: platformName,
    },
  });
  if (verifyResult.status !== 0) {
    console.error(verifyResult.stdout);
    console.error(verifyResult.stderr);
    throw new Error(`Expected acceptance package verification to pass for ${platformName}.`);
  }

  const manifest = JSON.parse(readFileSync(join(fixtureRoot, "docs", "acceptance-package", "acceptance-package-manifest.json"), "utf8"));
  const remainingEvidenceReadme = readFileSync(join(fixtureRoot, "docs", "acceptance-package", "REMAINING_EVIDENCE.md"), "utf8");
  const expectedEvidencePlatform = platformName === "win32" ? "win32" : "darwin";
  const copiedEvidence = manifest.copiedEvidence.join("\n");
  if (manifest.generatedOn.evidencePlatform !== expectedEvidencePlatform) {
    throw new Error(`Expected evidencePlatform ${expectedEvidencePlatform}, got ${manifest.generatedOn.evidencePlatform}.`);
  }
  if (!manifest.fileInventorySha256 || !Array.isArray(manifest.fileInventory) || manifest.fileInventory.length === 0) {
    throw new Error("Expected acceptance package manifest to include file inventory with digest.");
  }
  if (!manifest.fileInventory.some((item) => item.path === "docs/acceptance-package/README.md" && item.sha256)) {
    throw new Error("Expected file inventory to include package README digest.");
  }
  if (!remainingEvidenceReadme.includes("剩余证据组数：1") || !remainingEvidenceReadme.includes(`${platformName} 夹具剩余证据`)) {
    throw new Error(`Expected ${platformName} package to include human-readable remaining evidence.`);
  }
  if (!copiedEvidence.includes(`verify-release-${expectedEvidencePlatform}-`)) {
    throw new Error(`Expected ${platformName} package to include ${expectedEvidencePlatform} verify-release evidence.`);
  }
  if (!copiedEvidence.includes(`verify-all-local-${expectedEvidencePlatform}-`)) {
    throw new Error(`Expected ${platformName} package to include ${expectedEvidencePlatform} verify-all-local evidence.`);
  }
  if (!copiedEvidence.includes(`no-placeholder-scan-${expectedEvidencePlatform}-`)) {
    throw new Error(`Expected ${platformName} package to include ${expectedEvidencePlatform} no-placeholder evidence.`);
  }
  if (!copiedEvidence.includes(`readiness-${expectedEvidencePlatform}-`)) {
    throw new Error(`Expected ${platformName} package to include ${expectedEvidencePlatform} readiness evidence.`);
  }
  if (expectedEvidencePlatform === "win32" && copiedEvidence.includes("verify-release-darwin-")) {
    throw new Error("Windows acceptance package should not copy darwin release evidence.");
  }
  if (expectedEvidencePlatform === "win32" && copiedEvidence.includes("verify-all-local-darwin-")) {
    throw new Error("Windows acceptance package should not copy darwin all-local evidence.");
  }
  if (expectedEvidencePlatform === "win32" && copiedEvidence.includes("readiness-darwin-")) {
    throw new Error("Windows acceptance package should not copy darwin readiness evidence.");
  }
  if (expectedEvidencePlatform === "darwin" && copiedEvidence.includes("verify-release-win32-")) {
    throw new Error("macOS acceptance package should not copy win32 release evidence.");
  }
  if (expectedEvidencePlatform === "darwin" && copiedEvidence.includes("verify-all-local-win32-")) {
    throw new Error("macOS acceptance package should not copy win32 all-local evidence.");
  }
  if (expectedEvidencePlatform === "darwin" && copiedEvidence.includes("readiness-win32-")) {
    throw new Error("macOS acceptance package should not copy win32 readiness evidence.");
  }
  const copiedManualAcceptance = (manifest.copiedManualAcceptance ?? []).join("\n");
  for (const expectedManualFile of [
    `manual-acceptance-${expectedManualPlatform}-${fixtureManualDate}.json`,
    `manual-acceptance-${expectedManualPlatform}-${fixtureManualDate}-checklist.md`,
    `evidence-${expectedManualPlatform}-${fixtureManualDate}/README.md`,
  ]) {
    if (!copiedManualAcceptance.includes(expectedManualFile)) {
      throw new Error(`Expected ${platformName} package to include ${expectedManualFile}.`);
    }
  }

  rmSync(fixtureRoot, { recursive: true, force: true });
}

function setupRejectedAcceptancePackageFixture() {
  const fixtureRoot = mkdtempSync(join(tmpdir(), "aster-acceptance-package-rejected-"));
  const fixtureScriptsDir = join(fixtureRoot, "scripts");
  const fixtureDocsDir = join(fixtureRoot, "docs");
  const fixtureEvidenceDir = join(fixtureDocsDir, "release-evidence");
  const fixtureManualDir = join(fixtureDocsDir, "manual-acceptance");
  mkdirSync(fixtureScriptsDir, { recursive: true });
  mkdirSync(fixtureEvidenceDir, { recursive: true });
  mkdirSync(fixtureManualDir, { recursive: true });
  for (const script of [
    "create-acceptance-package.mjs",
    "verify-acceptance-package.mjs",
    "create-acceptance-runners.mjs",
  ]) {
    copyFileSync(join(root, "scripts", script), join(fixtureScriptsDir, script));
  }
  for (const relativePath of [
    "README.md",
    "docs/ASTER_EXECUTION_PLAN.md",
    "docs/ASTER_CROSS_PLATFORM_ACCEPTANCE.md",
    "docs/ASTER_DISASTER_RECOVERY_RUNBOOK.md",
    "docs/manual-acceptance/README.md",
  ]) {
    const target = join(fixtureRoot, relativePath);
    mkdirSync(join(target, ".."), { recursive: true });
    writeText(target, readFileSync(join(root, relativePath), "utf8"));
  }
  evidence("verify-all-local-darwin", allLocalEvidence("darwin"));
  evidence("verify-release-darwin", releaseEvidence("darwin"));
  evidence("execution-coverage-darwin", coverageEvidence("darwin"));
  evidence("readiness-darwin", readinessEvidence("darwin"));
  evidence("manual-acceptance-summary-darwin", manualSummary("darwin"));
  const failedNoPlaceholder = {
    ...noPlaceholderEvidence("darwin"),
    status: "failed",
    findings: [{ path: "src/App.tsx", line: 1, text: "TODO" }],
  };
  evidence("no-placeholder-scan-darwin", failedNoPlaceholder);
  for (const file of [
    "verify-release-darwin-placeholder.json",
    "verify-all-local-darwin-placeholder.json",
    "execution-coverage-darwin-placeholder.json",
    "readiness-darwin-placeholder.json",
    "no-placeholder-scan-darwin-placeholder.json",
    "manual-acceptance-summary-darwin-placeholder.json",
  ]) {
    copyFileSync(join(evidenceDir, file), join(fixtureEvidenceDir, file));
  }
  const fixtureManualDate = "2026-06-30";
  writeJson(join(fixtureManualDir, `manual-acceptance-macos-${fixtureManualDate}.json`), {
    schemaVersion: 1,
    platform: "macos",
    generatedAt: "2026-06-30T12:00:00.000Z",
  });
  writeText(join(fixtureManualDir, `manual-acceptance-macos-${fixtureManualDate}-checklist.md`), "# macos checklist\n");
  mkdirSync(join(fixtureManualDir, `evidence-macos-${fixtureManualDate}`), { recursive: true });
  writeText(join(fixtureManualDir, `evidence-macos-${fixtureManualDate}`, "README.md"), "# macos evidence\n");
  const createResult = spawnSync("node", [join(fixtureScriptsDir, "create-acceptance-package.mjs")], {
    cwd: fixtureRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      ASTER_ACCEPTANCE_PLATFORM: "darwin",
    },
  });
  if (createResult.status !== 0) {
    console.error(createResult.stdout);
    console.error(createResult.stderr);
    throw new Error("Expected rejected acceptance package creation fixture to build.");
  }
  const verifyResult = spawnSync("node", [join(fixtureScriptsDir, "verify-acceptance-package.mjs")], {
    cwd: fixtureRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      ASTER_ACCEPTANCE_PLATFORM: "darwin",
    },
  });
  if (verifyResult.status === 0 || !verifyResult.stderr.includes("no-placeholder-scan 最新证据状态必须为 passed")) {
    console.error(verifyResult.stdout);
    console.error(verifyResult.stderr);
    throw new Error("Expected acceptance package verification to reject failed no-placeholder evidence.");
  }
  rmSync(fixtureRoot, { recursive: true, force: true });
}

function setupFinalizeRejectsManualEvidenceFixture() {
  const fixtureRoot = mkdtempSync(join(tmpdir(), "aster-finalize-rejects-manual-evidence-"));
  const fixtureScriptsDir = join(fixtureRoot, "scripts");
  const fixtureEvidenceDir = join(fixtureRoot, "docs", "release-evidence");
  mkdirSync(fixtureScriptsDir, { recursive: true });
  mkdirSync(fixtureEvidenceDir, { recursive: true });
  copyFileSync(join(root, "scripts", "finalize-acceptance.mjs"), join(fixtureScriptsDir, "finalize-acceptance.mjs"));
  writeJson(join(fixtureEvidenceDir, "manual-acceptance-summary-darwin-placeholder.json"), manualSummary("darwin"));
  writeJson(join(fixtureEvidenceDir, "readiness-darwin-placeholder.json"), readinessEvidence("darwin"));
  writeJson(join(fixtureEvidenceDir, "acceptance-archive-darwin-placeholder.json"), {
    generatedAt: "2026-06-30T12:00:00.000Z",
    status: "passed",
    archive: {
      sha256: sha,
    },
  });

  const noopCommand = "process.exit(0)";
  const result = spawnSync("node", [join(fixtureScriptsDir, "finalize-acceptance.mjs")], {
    cwd: fixtureRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      ASTER_FINALIZE_VERIFY_STRICT_COMMAND: noopCommand,
      ASTER_FINALIZE_PACKAGE_COMMAND: noopCommand,
      ASTER_FINALIZE_READINESS_COMMAND: noopCommand,
      ASTER_FINALIZE_VERIFY_PACKAGE_COMMAND: noopCommand,
      ASTER_FINALIZE_ARCHIVE_COMMAND: noopCommand,
    },
  });
  if (
    result.status === 0 ||
    !result.stderr.includes("readiness 状态必须为 ready-for-final-archive")
  ) {
    console.error(result.stdout);
    console.error(result.stderr);
    throw new Error("Expected acceptance finalize to reject ready-for-manual-evidence readiness.");
  }
  const finalizeReports = readdirSync(fixtureEvidenceDir).filter((name) =>
    name.startsWith("acceptance-finalize-") && name.endsWith(".json"),
  );
  if (finalizeReports.length !== 1) {
    throw new Error("Expected acceptance finalize to write one failure report.");
  }
  const report = JSON.parse(readFileSync(join(fixtureEvidenceDir, finalizeReports[0]), "utf8"));
  if (
    report.status !== "failed" ||
    report.readinessStatus !== "ready-for-manual-evidence" ||
    report.remainingEvidenceCount !== 1
  ) {
    console.error(JSON.stringify(report, null, 2));
    throw new Error("Expected acceptance finalize failure report to include readiness and remaining evidence.");
  }
  rmSync(fixtureRoot, { recursive: true, force: true });
}

function setupReadinessRejectsStaleAllLocalFixture() {
  const fixtureRoot = mkdtempSync(join(tmpdir(), "aster-readiness-stale-all-local-"));
  const fixtureScriptsDir = join(fixtureRoot, "scripts");
  const fixturePackageDir = join(fixtureRoot, "docs", "acceptance-package");
  const fixturePackageEvidenceDir = join(fixturePackageDir, "release-evidence");
  const fixtureEvidenceDir = join(fixtureRoot, "docs", "release-evidence");
  mkdirSync(fixtureScriptsDir, { recursive: true });
  mkdirSync(fixturePackageEvidenceDir, { recursive: true });
  mkdirSync(fixtureEvidenceDir, { recursive: true });
  copyFileSync(join(root, "scripts", "verify-readiness.mjs"), join(fixtureScriptsDir, "verify-readiness.mjs"));

  const evidenceFiles = {
    "verify-all-local-darwin-placeholder.json": allLocalEvidence("darwin"),
    "verify-release-darwin-placeholder.json": releaseEvidence("darwin"),
    "execution-coverage-darwin-newer.json": {
      ...coverageEvidence("darwin"),
      generatedAt: "2026-06-30T12:05:00.000Z",
    },
    "no-placeholder-scan-darwin-placeholder.json": noPlaceholderEvidence("darwin"),
    "manual-acceptance-summary-darwin-placeholder.json": manualSummary("darwin"),
  };
  for (const [name, data] of Object.entries(evidenceFiles)) {
    writeJson(join(fixtureEvidenceDir, name), data);
  }
  for (const name of [
    "verify-release-darwin-placeholder.json",
    "execution-coverage-darwin-newer.json",
    "no-placeholder-scan-darwin-placeholder.json",
  ]) {
    copyFileSync(join(fixtureEvidenceDir, name), join(fixturePackageEvidenceDir, name));
  }
  writeJson(join(fixturePackageDir, "acceptance-package-manifest.json"), {
    generatedAt: "2026-06-30T12:00:00.000Z",
    generatedOn: {
      evidencePlatform: "darwin",
    },
    copiedEvidence: [
      "docs/acceptance-package/release-evidence/verify-release-darwin-placeholder.json",
      "docs/acceptance-package/release-evidence/execution-coverage-darwin-newer.json",
      "docs/acceptance-package/release-evidence/no-placeholder-scan-darwin-placeholder.json",
    ],
    remainingEvidence: manualSummary("darwin").remainingEvidence,
  });

  const result = spawnSync("node", [join(fixtureScriptsDir, "verify-readiness.mjs")], {
    cwd: fixtureRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      ASTER_ACCEPTANCE_PLATFORM: "darwin",
    },
  });
  if (
    result.status === 0 ||
    !result.stderr.includes("verify-all-local 早于最新 execution-coverage 证据")
  ) {
    console.error(result.stdout);
    console.error(result.stderr);
    throw new Error("Expected readiness to reject automatic evidence newer than verify-all-local.");
  }
  const readinessReports = readdirSync(fixtureEvidenceDir).filter((name) =>
    name.startsWith("readiness-darwin-") && name.endsWith(".json"),
  );
  if (readinessReports.length !== 1) {
    throw new Error("Expected readiness to write one blocked report.");
  }
  const report = JSON.parse(readFileSync(join(fixtureEvidenceDir, readinessReports[0]), "utf8"));
  if (
    report.status !== "blocked" ||
    !report.blockers.some((item) => item.includes("verify-all-local 早于最新 execution-coverage 证据"))
  ) {
    console.error(JSON.stringify(report, null, 2));
    throw new Error("Expected readiness report to include stale verify-all-local blocker.");
  }
  rmSync(fixtureRoot, { recursive: true, force: true });
}

function setupAcceptanceStatusFixture() {
  const fixtureRoot = mkdtempSync(join(tmpdir(), "aster-acceptance-status-"));
  const fixtureScriptsDir = join(fixtureRoot, "scripts");
  const fixturePackageDir = join(fixtureRoot, "docs", "acceptance-package");
  const fixtureEvidenceDir = join(fixtureRoot, "docs", "release-evidence");
  mkdirSync(fixtureScriptsDir, { recursive: true });
  mkdirSync(fixturePackageDir, { recursive: true });
  mkdirSync(fixtureEvidenceDir, { recursive: true });
  copyFileSync(join(root, "scripts", "acceptance-status.mjs"), join(fixtureScriptsDir, "acceptance-status.mjs"));
  writeJson(join(fixtureEvidenceDir, "verify-all-local-darwin-placeholder.json"), allLocalEvidence("darwin"));
  writeJson(join(fixtureEvidenceDir, "verify-release-darwin-placeholder.json"), releaseEvidence("darwin"));
  writeJson(join(fixtureEvidenceDir, "execution-coverage-darwin-placeholder.json"), coverageEvidence("darwin"));
  writeJson(join(fixtureEvidenceDir, "no-placeholder-scan-darwin-placeholder.json"), noPlaceholderEvidence("darwin"));
  writeJson(join(fixtureEvidenceDir, "readiness-darwin-placeholder.json"), readinessEvidence("darwin"));
  writeJson(join(fixtureEvidenceDir, "manual-acceptance-summary-darwin-placeholder.json"), manualSummary("darwin"));
  writeJson(join(fixtureEvidenceDir, "acceptance-archive-darwin-placeholder.json"), {
    generatedAt: "2026-06-30T12:00:00.000Z",
    status: "passed",
    archive: {
      path: "/tmp/aster-acceptance-package-darwin.zip",
      name: "aster-acceptance-package-darwin.zip",
      sha256: sha,
    },
  });
  writeJson(join(fixtureEvidenceDir, "acceptance-finalize-darwin-placeholder.json"), {
    generatedAt: "2026-06-30T12:00:00.000Z",
    status: "failed",
    readinessStatus: "ready-for-manual-evidence",
    remainingEvidenceCount: 1,
  });
  writeJson(join(fixturePackageDir, "acceptance-package-manifest.json"), {
    generatedAt: "2026-06-30T12:00:00.000Z",
    generatedOn: {
      evidencePlatform: "darwin",
    },
    copiedEvidence: [
      "docs/acceptance-package/release-evidence/verify-release-darwin-placeholder.json",
      "docs/acceptance-package/release-evidence/execution-coverage-darwin-placeholder.json",
      "docs/acceptance-package/release-evidence/no-placeholder-scan-darwin-placeholder.json",
      "docs/acceptance-package/release-evidence/readiness-darwin-placeholder.json",
    ],
    remainingEvidence: readinessEvidence("darwin").remainingEvidence,
  });
  mkdirSync(join(fixturePackageDir, "release-evidence"), { recursive: true });
  for (const name of [
    "verify-release-darwin-placeholder.json",
    "execution-coverage-darwin-placeholder.json",
    "no-placeholder-scan-darwin-placeholder.json",
    "readiness-darwin-placeholder.json",
  ]) {
    copyFileSync(join(fixtureEvidenceDir, name), join(fixturePackageDir, "release-evidence", name));
  }

  const result = spawnSync("node", [join(fixtureScriptsDir, "acceptance-status.mjs")], {
    cwd: fixtureRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      ASTER_ACCEPTANCE_PLATFORM: "darwin",
    },
  });
  if (result.status !== 0) {
    console.error(result.stdout);
    console.error(result.stderr);
    throw new Error("Expected acceptance status to pass for ready-for-manual-evidence fixture.");
  }
  const report = JSON.parse(result.stdout);
  if (
    report.status !== "ready-for-manual-evidence" ||
    report.remainingEvidence?.remainingCount !== 1 ||
    report.archive?.sha256 !== sha ||
    !report.nextCommand.includes("npm run acceptance:finalize")
  ) {
    console.error(JSON.stringify(report, null, 2));
    throw new Error("Expected acceptance status to summarize readiness, archive, and next command.");
  }
  const summaryResult = spawnSync("node", [join(fixtureScriptsDir, "acceptance-status.mjs"), "--summary"], {
    cwd: fixtureRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      ASTER_ACCEPTANCE_PLATFORM: "darwin",
    },
  });
  if (
    summaryResult.status !== 0 ||
    !summaryResult.stdout.includes("Status: ready-for-manual-evidence") ||
    !summaryResult.stdout.includes(`Archive SHA256: ${sha}`) ||
    !summaryResult.stdout.includes("Remaining evidence groups: 1") ||
    !summaryResult.stdout.includes("Next: 补齐 REMAINING_EVIDENCE.md") ||
    !summaryResult.stdout.includes("- WAIT acceptance-finalize: failed") ||
    summaryResult.stdout.includes("- OK acceptance-finalize: failed")
  ) {
    console.error(summaryResult.stdout);
    console.error(summaryResult.stderr);
    throw new Error("Expected acceptance status summary to include status, archive SHA256, remaining evidence, and next command.");
  }
  writeJson(join(fixtureEvidenceDir, "execution-coverage-darwin-zzzz-newer.json"), {
    ...coverageEvidence("darwin"),
    generatedAt: "2026-06-30T12:05:00.000Z",
  });
  const stalePackageResult = spawnSync("node", [join(fixtureScriptsDir, "acceptance-status.mjs")], {
    cwd: fixtureRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      ASTER_ACCEPTANCE_PLATFORM: "darwin",
    },
  });
  if (stalePackageResult.status === 0) {
    console.error(stalePackageResult.stdout);
    console.error(stalePackageResult.stderr);
    throw new Error("Expected acceptance status to fail when acceptance package is missing latest coverage evidence.");
  }
  const stalePackageReport = JSON.parse(stalePackageResult.stdout);
  if (
    stalePackageReport.status !== "blocked" ||
    !stalePackageReport.automaticBlockers.some((item) =>
      item.includes("验收交接包未记录最新 execution-coverage 证据"),
    )
  ) {
    console.error(JSON.stringify(stalePackageReport, null, 2));
    throw new Error("Expected acceptance status to report stale acceptance package evidence.");
  }
  writeJson(join(fixturePackageDir, "acceptance-package-manifest.json"), {
    generatedAt: "2026-06-30T12:00:00.000Z",
    generatedOn: {
      evidencePlatform: "darwin",
    },
    copiedEvidence: [
      "docs/acceptance-package/release-evidence/verify-release-darwin-placeholder.json",
      "docs/acceptance-package/release-evidence/execution-coverage-darwin-zzzz-newer.json",
      "docs/acceptance-package/release-evidence/no-placeholder-scan-darwin-placeholder.json",
      "docs/acceptance-package/release-evidence/readiness-darwin-placeholder.json",
    ],
    remainingEvidence: readinessEvidence("darwin").remainingEvidence,
  });
  copyFileSync(
    join(fixtureEvidenceDir, "execution-coverage-darwin-zzzz-newer.json"),
    join(fixturePackageDir, "release-evidence", "execution-coverage-darwin-zzzz-newer.json"),
  );
  const staleAllLocalSummary = spawnSync("node", [join(fixtureScriptsDir, "acceptance-status.mjs"), "--summary"], {
    cwd: fixtureRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      ASTER_ACCEPTANCE_PLATFORM: "darwin",
    },
  });
  if (
    staleAllLocalSummary.status === 0 ||
    !staleAllLocalSummary.stdout.includes("Automatic blockers:") ||
    !staleAllLocalSummary.stdout.includes("verify-all-local 早于最新 execution-coverage 证据") ||
    !staleAllLocalSummary.stdout.includes("Next: 最新自动证据晚于 verify-all-local，请执行 npm run verify:all-local")
  ) {
    console.error(staleAllLocalSummary.stdout);
    console.error(staleAllLocalSummary.stderr);
    throw new Error("Expected acceptance status summary to explain stale verify-all-local evidence.");
  }
  rmSync(fixtureRoot, { recursive: true, force: true });
}

function setupWindowsArtifactsImportFixture() {
  const fixtureRoot = mkdtempSync(join(tmpdir(), "aster-windows-artifacts-import-"));
  const fixtureScriptsDir = join(fixtureRoot, "scripts");
  const fixtureArtifactDir = join(fixtureRoot, "downloaded-artifacts");
  const fixtureInstallerDir = join(fixtureArtifactDir, "aster-windows-x64");
  const fixtureEvidenceDir = join(fixtureArtifactDir, "aster-windows-x64-release-evidence");
  mkdirSync(fixtureScriptsDir, { recursive: true });
  mkdirSync(fixtureInstallerDir, { recursive: true });
  mkdirSync(fixtureEvidenceDir, { recursive: true });
  copyFileSync(join(root, "scripts", "import-windows-artifacts.mjs"), join(fixtureScriptsDir, "import-windows-artifacts.mjs"));

  const installerPath = join(fixtureInstallerDir, "AsterSetup.exe");
  writeFileSync(installerPath, "windows installer fixture");
  const installerSha = createHash("sha256").update(readFileSync(installerPath)).digest("hex");
  const release = releaseEvidence("win32");
  release.artifacts = [
    {
      path: "D:\\a\\Aster\\src-tauri\\target\\x86_64-pc-windows-msvc\\release\\bundle\\nsis\\AsterSetup.exe",
      type: "file",
      bytes: statSync(installerPath).size,
      sha256: installerSha,
    },
  ];
  writeJson(join(fixtureEvidenceDir, "verify-release-win32-fixture.json"), release);
  writeJson(join(fixtureEvidenceDir, "execution-coverage-win32-fixture.json"), coverageEvidence("win32"));
  writeJson(join(fixtureEvidenceDir, "no-placeholder-scan-win32-fixture.json"), noPlaceholderEvidence("win32"));

  const result = spawnSync("node", [join(fixtureScriptsDir, "import-windows-artifacts.mjs"), fixtureArtifactDir], {
    cwd: fixtureRoot,
    encoding: "utf8",
  });
  if (result.status !== 0) {
    console.error(result.stdout);
    console.error(result.stderr);
    throw new Error("Expected Windows artifacts import to pass.");
  }
  for (const expected of [
    "docs/release-evidence/verify-release-win32-fixture.json",
    "docs/release-evidence/execution-coverage-win32-fixture.json",
    "docs/release-evidence/no-placeholder-scan-win32-fixture.json",
    "docs/manual-acceptance/windows-installers/AsterSetup.exe",
  ]) {
    if (!existsSync(join(fixtureRoot, expected))) {
      throw new Error(`Expected imported file: ${expected}`);
    }
  }
  const reports = readdirSync(join(fixtureRoot, "docs", "release-evidence")).filter((name) =>
    name.startsWith("windows-artifacts-import-"),
  );
  if (reports.length !== 1) {
    throw new Error("Expected Windows artifacts import report.");
  }

  const badRoot = mkdtempSync(join(tmpdir(), "aster-windows-artifacts-import-bad-"));
  const badScriptsDir = join(badRoot, "scripts");
  const badArtifactDir = join(badRoot, "downloaded-artifacts");
  const badInstallerDir = join(badArtifactDir, "aster-windows-x64");
  const badEvidenceDir = join(badArtifactDir, "aster-windows-x64-release-evidence");
  mkdirSync(badScriptsDir, { recursive: true });
  mkdirSync(badInstallerDir, { recursive: true });
  mkdirSync(badEvidenceDir, { recursive: true });
  copyFileSync(join(root, "scripts", "import-windows-artifacts.mjs"), join(badScriptsDir, "import-windows-artifacts.mjs"));
  writeFileSync(join(badInstallerDir, "AsterSetup.exe"), "tampered installer");
  writeJson(join(badEvidenceDir, "verify-release-win32-fixture.json"), release);
  const badResult = spawnSync("node", [join(badScriptsDir, "import-windows-artifacts.mjs"), badArtifactDir], {
    cwd: badRoot,
    encoding: "utf8",
  });
  if (badResult.status === 0 || !badResult.stderr.includes("No copied Windows installer SHA256 matches")) {
    console.error(badResult.stdout);
    console.error(badResult.stderr);
    throw new Error("Expected Windows artifacts import to reject mismatched installer SHA256.");
  }

  rmSync(fixtureRoot, { recursive: true, force: true });
  rmSync(badRoot, { recursive: true, force: true });
}

function setupWindowsArtifactsDownloadFixture() {
  const fixtureRoot = mkdtempSync(join(tmpdir(), "aster-windows-artifacts-download-"));
  const fixtureScriptsDir = join(fixtureRoot, "scripts");
  const fixtureArtifactRoot = join(fixtureRoot, "artifact-source");
  const fixtureInstallerDir = join(fixtureArtifactRoot, "aster-windows-x64");
  const fixtureEvidenceDir = join(fixtureArtifactRoot, "aster-windows-x64-release-evidence");
  mkdirSync(fixtureScriptsDir, { recursive: true });
  mkdirSync(fixtureInstallerDir, { recursive: true });
  mkdirSync(fixtureEvidenceDir, { recursive: true });
  copyFileSync(join(root, "scripts", "download-github-windows-artifacts.mjs"), join(fixtureScriptsDir, "download-github-windows-artifacts.mjs"));
  copyFileSync(join(root, "scripts", "import-windows-artifacts.mjs"), join(fixtureScriptsDir, "import-windows-artifacts.mjs"));
  writeJson(join(fixtureRoot, "package.json"), {
    type: "module",
    scripts: {
      "acceptance:import-windows-artifacts": "node scripts/import-windows-artifacts.mjs",
    },
  });

  const installerPath = join(fixtureInstallerDir, "AsterSetup.exe");
  writeFileSync(installerPath, "downloaded windows installer fixture");
  const installerSha = createHash("sha256").update(readFileSync(installerPath)).digest("hex");
  const release = releaseEvidence("win32");
  release.artifacts = [
    {
      path: "D:\\a\\Aster\\src-tauri\\target\\x86_64-pc-windows-msvc\\release\\bundle\\nsis\\AsterSetup.exe",
      type: "file",
      bytes: statSync(installerPath).size,
      sha256: installerSha,
    },
  ];
  writeJson(join(fixtureEvidenceDir, "verify-release-win32-fixture.json"), release);
  writeJson(join(fixtureEvidenceDir, "execution-coverage-win32-fixture.json"), coverageEvidence("win32"));
  writeJson(join(fixtureEvidenceDir, "no-placeholder-scan-win32-fixture.json"), noPlaceholderEvidence("win32"));

  const installerZip = join(fixtureRoot, "aster-windows-x64.zip");
  const evidenceZip = join(fixtureRoot, "aster-windows-x64-release-evidence.zip");
  for (const [zipPath, sourceDir] of [
    [installerZip, fixtureInstallerDir],
    [evidenceZip, fixtureEvidenceDir],
  ]) {
    const zipResult = spawnSync("zip", ["-qr", zipPath, "."], {
      cwd: sourceDir,
      encoding: "utf8",
    });
    if (zipResult.status !== 0) {
      console.error(zipResult.stdout);
      console.error(zipResult.stderr);
      throw new Error("Expected fixture zip creation to pass.");
    }
  }

  let server;
  const serverReady = new Promise((resolveServer) => {
    server = http.createServer((request, response) => {
      const url = new URL(request.url, "http://127.0.0.1");
      if (url.pathname.endsWith("/actions/workflows/build-desktop.yml/runs")) {
        response.setHeader("content-type", "application/json");
        response.end(JSON.stringify({
          workflow_runs: [
            {
              id: 123,
              conclusion: "success",
              html_url: "https://github.example/actions/runs/123",
            },
          ],
        }));
        return;
      }
      if (url.pathname.endsWith("/actions/runs/123/artifacts")) {
        response.setHeader("content-type", "application/json");
        response.end(JSON.stringify({
          artifacts: [
            {
              name: "aster-windows-x64",
              expired: false,
              archive_download_url: `http://127.0.0.1:${server.address().port}/download/installer`,
            },
            {
              name: "aster-windows-x64-release-evidence",
              expired: false,
              archive_download_url: `http://127.0.0.1:${server.address().port}/download/evidence`,
            },
          ],
        }));
        return;
      }
      const file = url.pathname.endsWith("/download/installer")
        ? installerZip
        : url.pathname.endsWith("/download/evidence")
          ? evidenceZip
          : null;
      if (!file) {
        response.statusCode = 404;
        response.end("not found");
        return;
      }
      response.setHeader("content-type", "application/zip");
      response.end(readFileSync(file));
    });
    server.listen(0, "127.0.0.1", () => resolveServer(server.address().port));
  });

  return serverReady.then(async (port) => {
    const missingToken = await spawnNodeAsync([join(fixtureScriptsDir, "download-github-windows-artifacts.mjs")], {
      cwd: fixtureRoot,
      env: {
        ...process.env,
        GITHUB_TOKEN: "",
        GH_TOKEN: "",
      },
    });
    if (missingToken.status === 0 || !missingToken.stderr.includes("Missing GITHUB_TOKEN")) {
      console.error(missingToken.stdout);
      console.error(missingToken.stderr);
      throw new Error("Expected Windows artifact downloader to require a token.");
    }

    const result = await spawnNodeAsync([join(fixtureScriptsDir, "download-github-windows-artifacts.mjs")], {
      cwd: fixtureRoot,
      env: {
        ...process.env,
        GITHUB_TOKEN: "fixture-token",
        ASTER_GITHUB_REPOSITORY: "fixture/Aster",
        ASTER_GITHUB_API_BASE_URL: `http://127.0.0.1:${port}`,
      },
    });
    server.close();
    if (result.status !== 0) {
      console.error(result.stdout);
      console.error(result.stderr);
      throw new Error("Expected Windows artifact downloader to download and import fixture artifacts.");
    }
    for (const expected of [
      "docs/release-evidence/verify-release-win32-fixture.json",
      "docs/release-evidence/execution-coverage-win32-fixture.json",
      "docs/release-evidence/no-placeholder-scan-win32-fixture.json",
      "docs/manual-acceptance/windows-installers/AsterSetup.exe",
    ]) {
      if (!existsSync(join(fixtureRoot, expected))) {
        throw new Error(`Expected downloaded and imported file: ${expected}`);
      }
    }
    rmSync(fixtureRoot, { recursive: true, force: true });
  }).finally(() => {
    server?.close();
  });
}

function setupGithubDesktopBuildRunnerFixture() {
  const fixtureRoot = mkdtempSync(join(tmpdir(), "aster-github-build-runner-"));
  const fixtureScriptsDir = join(fixtureRoot, "scripts");
  mkdirSync(fixtureScriptsDir, { recursive: true });
  copyFileSync(join(root, "scripts", "run-github-desktop-build.mjs"), join(fixtureScriptsDir, "run-github-desktop-build.mjs"));

  const npmFixture = join(fixtureRoot, process.platform === "win32" ? "npm.cmd" : "npm");
  const npmLog = join(fixtureRoot, "npm-command.log");
  writeFileSync(
    npmFixture,
    process.platform === "win32"
      ? `@echo off\r\necho %* > "${npmLog}"\r\nexit /b 0\r\n`
      : `#!/usr/bin/env bash\necho "$*" > "${npmLog}"\n`,
    { mode: 0o755 },
  );

  let dispatched = false;
  let server;
  const serverReady = new Promise((resolveServer) => {
    server = http.createServer((request, response) => {
      const url = new URL(request.url, "http://127.0.0.1");
      if (url.pathname.endsWith("/actions/workflows/build-desktop.yml/dispatches")) {
        dispatched = true;
        response.statusCode = 204;
        response.end();
        return;
      }
      if (url.pathname.endsWith("/actions/workflows/build-desktop.yml/runs")) {
        response.setHeader("content-type", "application/json");
        response.end(JSON.stringify({
          workflow_runs: [
            {
              id: 456,
              status: "completed",
              conclusion: "success",
              created_at: new Date().toISOString(),
              html_url: "https://github.example/actions/runs/456",
            },
          ],
        }));
        return;
      }
      response.statusCode = 404;
      response.end("not found");
    });
    server.listen(0, "127.0.0.1", () => resolveServer(server.address().port));
  });

  return serverReady.then(async (port) => {
    const missingToken = await spawnNodeAsync([join(fixtureScriptsDir, "run-github-desktop-build.mjs")], {
      cwd: fixtureRoot,
      env: {
        ...process.env,
        GITHUB_TOKEN: "",
        GH_TOKEN: "",
      },
    });
    if (missingToken.status === 0 || !missingToken.stderr.includes("Missing GITHUB_TOKEN")) {
      console.error(missingToken.stdout);
      console.error(missingToken.stderr);
      throw new Error("Expected GitHub build runner to require a token.");
    }

    const result = await spawnNodeAsync([join(fixtureScriptsDir, "run-github-desktop-build.mjs")], {
      cwd: fixtureRoot,
      env: {
        ...process.env,
        GITHUB_TOKEN: "fixture-token",
        ASTER_GITHUB_REPOSITORY: "fixture/Aster",
        ASTER_GITHUB_BRANCH: "codex/aster-full-execution",
        ASTER_GITHUB_API_BASE_URL: `http://127.0.0.1:${port}`,
        ASTER_GITHUB_POLL_INTERVAL_MS: "10",
        ASTER_GITHUB_BUILD_TIMEOUT_MS: "1000",
        ASTER_NPM_COMMAND: npmFixture,
      },
    });
    server.close();
    if (result.status !== 0) {
      console.error(result.stdout);
      console.error(result.stderr);
      throw new Error("Expected GitHub build runner to dispatch, poll, and invoke artifact download.");
    }
    if (!dispatched) {
      throw new Error("Expected GitHub build runner to call workflow_dispatch.");
    }
    if (!existsSync(npmLog) || !readFileSync(npmLog, "utf8").includes("acceptance:download-windows-artifacts")) {
      throw new Error("Expected GitHub build runner to invoke artifact download command.");
    }
    rmSync(fixtureRoot, { recursive: true, force: true });
  }).finally(() => {
    server?.close();
  });
}

setupAcceptancePackageFixture("win32");
setupAcceptancePackageFixture("darwin");
setupRejectedAcceptancePackageFixture();
setupFinalizeRejectsManualEvidenceFixture();
setupReadinessRejectsStaleAllLocalFixture();
setupAcceptanceStatusFixture();
setupWindowsArtifactsImportFixture();
await setupWindowsArtifactsDownloadFixture();
await setupGithubDesktopBuildRunnerFixture();

console.log("[test-manual-acceptance-paths] Cross-platform paths and platform mismatch checks passed.");
