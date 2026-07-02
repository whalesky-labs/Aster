import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { arch, hostname, platform as osPlatform, release } from "node:os";

const root = process.cwd();
const targetDir = join(root, "docs", "manual-acceptance");
const evidenceDir = join(root, "docs", "release-evidence");
mkdirSync(targetDir, { recursive: true });

const platform = (process.argv[2] ?? process.platform).toLowerCase();
const normalizedPlatform = platform.startsWith("win")
  ? "windows"
  : platform.startsWith("darwin") || platform.startsWith("mac")
    ? "macos"
    : platform;

const date = new Date().toISOString().slice(0, 10);
const target = join(targetDir, `manual-acceptance-${normalizedPlatform}-${date}.json`);
const evidencePlatform = normalizedPlatform === "macos" ? "darwin" : normalizedPlatform === "windows" ? "win32" : normalizedPlatform;
const evidenceAttachmentDir = `docs/manual-acceptance/evidence-${normalizedPlatform}-${date}`;

function listEvidence(prefix) {
  if (!existsSync(evidenceDir)) return [];
  return readdirSync(evidenceDir)
    .filter((name) => name.startsWith(`${prefix}-${evidencePlatform}-`) && name.endsWith(".json"))
    .map((name) => join(evidenceDir, name))
    .sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs);
}

function readJson(path) {
  if (!path || !existsSync(path)) return null;
  return JSON.parse(readFileSync(path, "utf8"));
}

function relativeToRoot(path) {
  if (!path) return "";
  return path.startsWith(root) ? path.slice(root.length + 1) : path;
}

const latestReleaseReport = listEvidence("verify-release")[0] ?? "";
const latestCoverageReport = listEvidence("execution-coverage")[0] ?? "";
const releaseReport = readJson(latestReleaseReport);
const appVersion = releaseReport?.tauriConfig?.version ?? "0.1.0";
const installerArtifact = releaseReport?.artifacts?.find((artifact) =>
  artifact.type === "file" && /\.(dmg|exe|msi)$/i.test(basename(artifact.path)),
);

const template = {
  schemaVersion: 1,
  generatedAt: new Date().toISOString(),
  platform: normalizedPlatform,
  machine: {
    operator: "",
    computerName: hostname(),
    osVersion: `${osPlatform()} ${release()}`,
    cpuArch: arch(),
    appVersion
  },
  releaseEvidence: {
    verifyReleaseReport: relativeToRoot(latestReleaseReport),
    executionCoverageReport: relativeToRoot(latestCoverageReport),
    installerPath: installerArtifact?.path ?? "",
    installerSha256: installerArtifact?.sha256 ?? ""
  },
  localInstall: {
    installerGenerated: false,
    installerOpenedOrInstalled: false,
    firstLaunchOk: false,
    defaultAdminLoginOk: false,
    databaseCreatedInSystemAppDataDir: false,
    appDataDir: ""
  },
  excel: {
    importPreviewReadLegacyWorkbook: false,
    importRunCreatedRecords: false,
    monthlyReportExported: false,
    exportedWorkbookOpenedWithoutOfficeOrWpsDependency: false,
    exportedWorkbookPath: ""
  },
  backup: {
    manualBackupCreated: false,
    secondBackupDirWritable: false,
    backupZipContainsRequiredEntries: false,
    backupFile: "",
    sourceHostName: "",
    sourceOs: "",
    backupSha256: ""
  },
  restore: {
    beforeRestoreBackupCreated: false,
    beforeRestoreBackupFile: "",
    restoredBackupFromOtherPlatform: false,
    restoredFromPlatform: "",
    restoredBackupFile: "",
    restoredBackupSourceHostName: "",
    healthCheckOkAfterRestore: false,
    dataConsistentAfterRestore: false
  },
  evidenceFiles: {
    installScreenshot: `${evidenceAttachmentDir}/01-install-or-launch.png`,
    loginScreenshot: `${evidenceAttachmentDir}/02-admin-login.png`,
    databaseLocationScreenshot: `${evidenceAttachmentDir}/03-database-location.png`,
    excelImportPreviewScreenshot: `${evidenceAttachmentDir}/04-excel-import-preview.png`,
    exportedWorkbookScreenshot: `${evidenceAttachmentDir}/05-exported-workbook-opened.png`,
    backupRecordScreenshot: `${evidenceAttachmentDir}/06-backup-record.png`,
    restorePreviewScreenshot: `${evidenceAttachmentDir}/07-restore-preview.png`,
    restoreResultScreenshot: `${evidenceAttachmentDir}/08-restore-result.png`,
    hostModeScreenshot: `${evidenceAttachmentDir}/09-host-mode.png`,
    clientModeScreenshot: `${evidenceAttachmentDir}/10-client-mode.png`
  },
  sampleData: {
    importedWorkbookName: "",
    exportedReportMonth: "",
    backupRecordId: "",
    restoredItemCount: null,
    restoredMovementCount: null,
    asHostBusinessDocumentNo: "",
    asClientBusinessDocumentNo: ""
  },
  hostClient: {
    asHost: {
      tested: false,
      peerPlatform: "",
      hostIp: "",
      hostPort: 17871,
      pairCodeDisplayed: false,
      peerPaired: false,
      disconnectDetectedByPeer: false,
      reconnectDetectedByPeer: false,
      peerBusinessWriteSucceeded: false,
      hostInventoryAndReportsConsistent: false
    },
    asClient: {
      tested: false,
      peerPlatform: "",
      connectedByDiscoveryOrManualAddress: false,
      paired: false,
      disconnectDetected: false,
      reconnectDetected: false,
      clientBusinessWriteSucceeded: false,
      hostInventoryAndReportsConsistent: false
    }
  },
  notes: ""
};

if (existsSync(target) && !process.argv.includes("--force")) {
  console.error(`[manual-acceptance-template] File already exists: ${target}`);
  console.error("[manual-acceptance-template] Re-run with --force to overwrite.");
  process.exit(1);
}

writeFileSync(target, `${JSON.stringify(template, null, 2)}\n`);
console.log(`[manual-acceptance-template] Created: ${target}`);
