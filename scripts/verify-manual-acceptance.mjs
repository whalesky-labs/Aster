import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { arch, platform, release } from "node:os";

const root = process.cwd();
const acceptanceDir = join(root, "docs", "manual-acceptance");
const evidenceDir = join(root, "docs", "release-evidence");
const strict = process.argv.includes("--strict");

function listJsonFiles(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((name) => name.endsWith(".json"))
    .map((name) => join(dir, name))
    .sort();
}

function loadJson(path) {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch (error) {
    return { __invalidJson: String(error) };
  }
}

function loadEvidenceJson(path, missing, label) {
  if (!path) return null;
  if (!existsSync(path)) {
    missing.push(`${label} 路径不存在`);
    return null;
  }
  const data = loadJson(path);
  if (data.__invalidJson) {
    missing.push(`${label} JSON 无法解析`);
    return null;
  }
  return data;
}

function existsNonEmpty(path) {
  return typeof path === "string" && path.trim().length > 0 && existsSync(resolveEvidencePath(path));
}

function resolveEvidencePath(value) {
  if (value.startsWith("/") || /^[A-Za-z]:[\\/]/.test(value)) {
    return value;
  }
  return join(root, value);
}

function isAbsolutePath(value) {
  return typeof value === "string" && (value.startsWith("/") || /^[A-Za-z]:[\\/]/.test(value));
}

function boolAt(record, path) {
  return path.split(".").reduce((value, key) => value?.[key], record) === true;
}

function valueAt(record, path) {
  return path.split(".").reduce((value, key) => value?.[key], record);
}

function isNonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function isSha256(value) {
  return typeof value === "string" && /^[a-fA-F0-9]{64}$/.test(value.trim());
}

function isValidPort(value) {
  return Number.isInteger(value) && value >= 1024 && value <= 65535;
}

function isNonNegativeInteger(value) {
  return Number.isInteger(value) && value >= 0;
}

function requireNonEmpty(record, path, label, missing) {
  if (!isNonEmptyString(valueAt(record, path))) {
    missing.push(label);
  }
}

function requireSha256(record, path, label, missing) {
  if (!isSha256(valueAt(record, path))) {
    missing.push(label);
  }
}

function requireExistingPathIfProvided(record, path, label, missing) {
  const value = valueAt(record, path);
  if (isNonEmptyString(value) && !isAbsolutePath(value) && !existsSync(resolveEvidencePath(value))) {
    missing.push(label);
  }
}

function requireEvidenceFile(record, path, label, missing) {
  requireNonEmpty(record, path, `${label}不能为空`, missing);
  requireExistingPathIfProvided(record, path, `${label}路径不存在`, missing);
}

function requireNonNegativeInteger(record, path, label, missing) {
  if (!isNonNegativeInteger(valueAt(record, path))) {
    missing.push(label);
  }
}

function latestMatchingEvidence(prefix, platformName) {
  const normalized =
    platformName === "macos" ? "darwin" : platformName === "windows" ? "win32" : platformName;
  const files = listJsonFiles(evidenceDir)
    .filter((file) => basename(file).startsWith(`${prefix}-${normalized}-`))
    .sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs);
  return files[0] ?? null;
}

function evidencePlatform(platformName) {
  return platformName === "macos" ? "darwin" : platformName === "windows" ? "win32" : platformName;
}

function makeEvidenceValidation() {
  return {
    verifyRelease: {
      checked: false,
      path: null,
      platformMatches: false,
      versionMatches: false,
      hasArtifacts: false,
      installerPathMatches: false,
      installerShaMatches: false,
      requiredCommandsPresent: false,
      requiredCommandResultsSuccessful: false,
    },
    executionCoverage: {
      checked: false,
      path: null,
      platformMatches: false,
      statusCovered: false,
      allFeaturesCovered: false,
      requiredFeatureGroupsPresent: false,
    },
  };
}

function validateReleaseReport(reportPath, record, platformName, missing) {
  const validation = makeEvidenceValidation().verifyRelease;
  validation.path = reportPath ?? null;
  const report = loadEvidenceJson(reportPath, missing, "verify-release 证据报告");
  if (!report) return validation;
  validation.checked = true;

  const expectedPlatform = evidencePlatform(platformName);
  if (report.platform !== expectedPlatform) {
    missing.push(`verify-release 平台必须为 ${expectedPlatform}`);
  } else {
    validation.platformMatches = true;
  }

  const appVersion = valueAt(record, "machine.appVersion");
  if (isNonEmptyString(appVersion) && report.tauriConfig?.version !== appVersion) {
    missing.push("verify-release 应用版本必须与手工记录一致");
  } else if (isNonEmptyString(appVersion)) {
    validation.versionMatches = true;
  }

  const artifacts = Array.isArray(report.artifacts) ? report.artifacts : [];
  if (artifacts.length === 0) {
    missing.push("verify-release 证据报告必须包含打包产物");
  } else {
    validation.hasArtifacts = true;
  }

  const installerSha = valueAt(record, "releaseEvidence.installerSha256");
  const installerPath = valueAt(record, "releaseEvidence.installerPath");
  if (
    isNonEmptyString(installerPath) &&
    artifacts.length > 0 &&
    !artifacts.some((artifact) => artifact.path === installerPath)
  ) {
    missing.push("安装包路径必须匹配 verify-release 证据报告中的产物路径");
  } else if (isNonEmptyString(installerPath) && artifacts.length > 0) {
    validation.installerPathMatches = true;
  }
  if (isSha256(installerSha) && artifacts.length > 0 && !artifacts.some((artifact) => artifact.sha256 === installerSha)) {
    missing.push("安装包 SHA256 必须匹配 verify-release 证据报告中的产物 SHA256");
  } else if (isSha256(installerSha) && artifacts.length > 0) {
    validation.installerShaMatches = true;
  }

  const commands = Array.isArray(report.commands) ? report.commands : [];
  const commandResults = Array.isArray(report.commandResults) ? report.commandResults : [];
  let commandsOk = true;
  let commandResultsOk = true;
  const requiredCommands = [
    "npm run build",
    "npm run verify:coverage",
    "npm run verify:no-placeholders",
    "cargo fmt --check",
    "cargo test",
    "npm run tauri -- build",
  ];
  const commandMatchers = {
    "npm run tauri -- build": (command) =>
      command === "npm run tauri -- build" || command?.startsWith("npm run tauri -- build "),
  };
  for (const command of requiredCommands) {
    const matchesCommand = commandMatchers[command] ?? ((candidate) => candidate === command);
    const recordedCommand = commands.find(matchesCommand);
    if (!recordedCommand) {
      missing.push(`verify-release 证据报告缺少命令记录：${command}`);
      commandsOk = false;
    }
    const result = commandResults.find((item) => matchesCommand(item?.command));
    if (!result) {
      missing.push(`verify-release 证据报告缺少命令执行结果：${command}`);
      commandResultsOk = false;
    } else if (result.status !== "passed" || result.exitCode !== 0) {
      missing.push(`verify-release 命令执行未成功：${command}`);
      commandResultsOk = false;
    }
  }
  validation.requiredCommandsPresent = commandsOk;
  validation.requiredCommandResultsSuccessful = commandResultsOk;
  return validation;
}

function validateCoverageReport(reportPath, platformName, missing) {
  const validation = makeEvidenceValidation().executionCoverage;
  validation.path = reportPath ?? null;
  const report = loadEvidenceJson(reportPath, missing, "execution-coverage 证据报告");
  if (!report) return validation;
  validation.checked = true;

  const expectedPlatform = evidencePlatform(platformName);
  if (report.platform !== expectedPlatform) {
    missing.push(`execution-coverage 平台必须为 ${expectedPlatform}`);
  } else {
    validation.platformMatches = true;
  }
  if (report.coverageStatus !== "covered") {
    missing.push("execution-coverage 状态必须为 covered");
  } else {
    validation.statusCovered = true;
  }

  const features = Array.isArray(report.features) ? report.features : [];
  if (features.length === 0) {
    missing.push("execution-coverage 证据报告必须包含功能组");
  }
  const uncovered = features.filter((feature) => feature.status !== "covered");
  if (uncovered.length > 0) {
    missing.push(`execution-coverage 存在未覆盖功能组：${uncovered.map((feature) => feature.id ?? feature.name).join(", ")}`);
  } else if (features.length > 0) {
    validation.allFeaturesCovered = true;
  }

  const expectedFeatureIds = [
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
  const featureIds = new Set(features.map((feature) => feature.id));
  let requiredFeatureGroupsPresent = true;
  for (const id of expectedFeatureIds) {
    if (!featureIds.has(id)) {
      missing.push(`execution-coverage 缺少功能组：${id}`);
      requiredFeatureGroupsPresent = false;
    }
  }
  validation.requiredFeatureGroupsPresent = requiredFeatureGroupsPresent;
  return validation;
}

const records = listJsonFiles(acceptanceDir).map((path) => ({
  path,
  data: loadJson(path),
}));

function recordTimestamp(record) {
  const generatedAt = Date.parse(record.data?.generatedAt ?? "");
  if (Number.isFinite(generatedAt)) return generatedAt;
  try {
    return statSync(record.path).mtimeMs;
  } catch {
    return 0;
  }
}

function latestRecordForPlatform(platformName) {
  return records
    .filter((item) => item.data.platform === platformName && !item.data.__invalidJson)
    .sort((a, b) => recordTimestamp(b) - recordTimestamp(a) || b.path.localeCompare(a.path))[0] ?? null;
}

const requiredPlatforms = ["windows", "macos"];
const platformRequirements = [
  ["localInstall.installerGenerated", "安装包已生成"],
  ["localInstall.installerOpenedOrInstalled", "安装器可打开或已安装"],
  ["localInstall.firstLaunchOk", "首次启动成功"],
  ["localInstall.defaultAdminLoginOk", "默认管理员可登录"],
  ["localInstall.databaseCreatedInSystemAppDataDir", "数据库创建在系统应用数据目录"],
  ["excel.importPreviewReadLegacyWorkbook", "旧 Excel 可预览"],
  ["excel.importRunCreatedRecords", "Excel 正式导入已生成记录"],
  ["excel.monthlyReportExported", "月报 Excel 已导出"],
  ["excel.exportedWorkbookOpenedWithoutOfficeOrWpsDependency", "导出工作簿可打开且不依赖 Office/WPS"],
  ["backup.manualBackupCreated", "手动备份已生成"],
  ["backup.secondBackupDirWritable", "第二备份目录可写"],
  ["backup.backupZipContainsRequiredEntries", "备份包必需条目齐全"],
  ["restore.beforeRestoreBackupCreated", "恢复前保护备份已生成"],
  ["restore.restoredBackupFromOtherPlatform", "已恢复另一平台备份"],
  ["restore.healthCheckOkAfterRestore", "恢复后健康检查通过"],
  ["restore.dataConsistentAfterRestore", "恢复后数据一致"],
];

const findings = [];
const platformSummaries = requiredPlatforms.map((platformName) => {
  const record = latestRecordForPlatform(platformName);
  const missing = [];
  const evidenceValidation = makeEvidenceValidation();

  if (!record) {
    return {
      platform: platformName,
      status: "missing-record",
      recordPath: null,
      missing: ["缺少手工验收 JSON 记录"],
    };
  }

  if (record.data.schemaVersion !== 1) {
    missing.push("schemaVersion 必须为 1");
  }
  requireNonEmpty(record.data, "machine.operator", "验收操作人不能为空", missing);
  requireNonEmpty(record.data, "machine.computerName", "电脑名称不能为空", missing);
  requireNonEmpty(record.data, "machine.osVersion", "系统版本不能为空", missing);
  requireNonEmpty(record.data, "machine.cpuArch", "CPU 架构不能为空", missing);
  requireNonEmpty(record.data, "machine.appVersion", "应用版本不能为空", missing);

  const releaseReport = valueAt(record.data, "releaseEvidence.verifyReleaseReport");
  const coverageReport = valueAt(record.data, "releaseEvidence.executionCoverageReport");
  const resolvedReleaseReport = existsNonEmpty(releaseReport)
    ? resolveEvidencePath(releaseReport)
    : latestMatchingEvidence("verify-release", platformName);
  const resolvedCoverageReport = existsNonEmpty(coverageReport)
    ? resolveEvidencePath(coverageReport)
    : latestMatchingEvidence("execution-coverage", platformName);
  if (!resolvedReleaseReport) {
    missing.push("缺少 verify-release 证据报告");
  }
  if (!resolvedCoverageReport) {
    missing.push("缺少 execution-coverage 证据报告");
  }
  if (isNonEmptyString(releaseReport) && !existsNonEmpty(releaseReport)) {
    missing.push("verify-release 证据报告路径不存在");
  }
  if (isNonEmptyString(coverageReport) && !existsNonEmpty(coverageReport)) {
    missing.push("execution-coverage 证据报告路径不存在");
  }
  evidenceValidation.verifyRelease = validateReleaseReport(resolvedReleaseReport, record.data, platformName, missing);
  evidenceValidation.executionCoverage = validateCoverageReport(resolvedCoverageReport, platformName, missing);
  requireNonEmpty(record.data, "releaseEvidence.installerPath", "安装包路径不能为空", missing);
  requireSha256(record.data, "releaseEvidence.installerSha256", "安装包 SHA256 必须为 64 位十六进制", missing);
  requireExistingPathIfProvided(record.data, "releaseEvidence.installerPath", "安装包路径不存在", missing);

  for (const [path, label] of platformRequirements) {
    if (!boolAt(record.data, path)) {
      missing.push(label);
    }
  }

  requireNonEmpty(record.data, "localInstall.appDataDir", "系统应用数据目录不能为空", missing);
  requireNonEmpty(record.data, "excel.exportedWorkbookPath", "导出的 Excel 文件路径不能为空", missing);
  requireExistingPathIfProvided(record.data, "excel.exportedWorkbookPath", "导出的 Excel 文件路径不存在", missing);
  requireNonEmpty(record.data, "backup.backupFile", "备份文件路径不能为空", missing);
  requireNonEmpty(record.data, "backup.sourceHostName", "备份来源主机名不能为空", missing);
  requireNonEmpty(record.data, "backup.sourceOs", "备份来源系统不能为空", missing);
  requireSha256(record.data, "backup.backupSha256", "备份文件 SHA256 必须为 64 位十六进制", missing);
  requireExistingPathIfProvided(record.data, "backup.backupFile", "备份文件路径不存在", missing);
  requireNonEmpty(record.data, "restore.beforeRestoreBackupFile", "恢复前保护备份路径不能为空", missing);
  requireExistingPathIfProvided(record.data, "restore.beforeRestoreBackupFile", "恢复前保护备份路径不存在", missing);
  requireNonEmpty(record.data, "restore.restoredBackupFile", "跨平台恢复使用的备份文件路径不能为空", missing);
  requireNonEmpty(record.data, "restore.restoredBackupSourceHostName", "跨平台恢复备份来源主机名不能为空", missing);
  requireExistingPathIfProvided(record.data, "restore.restoredBackupFile", "跨平台恢复备份文件路径不存在", missing);

  for (const [path, label] of [
    ["evidenceFiles.installScreenshot", "安装或应用启动截图"],
    ["evidenceFiles.loginScreenshot", "默认管理员登录截图"],
    ["evidenceFiles.databaseLocationScreenshot", "系统应用数据目录建库截图"],
    ["evidenceFiles.excelImportPreviewScreenshot", "Excel 导入预览截图"],
    ["evidenceFiles.exportedWorkbookScreenshot", "导出工作簿打开截图"],
    ["evidenceFiles.backupRecordScreenshot", "备份记录截图"],
    ["evidenceFiles.restorePreviewScreenshot", "恢复预览截图"],
    ["evidenceFiles.restoreResultScreenshot", "恢复结果截图"],
    ["evidenceFiles.hostModeScreenshot", "作为主机连接状态截图"],
    ["evidenceFiles.clientModeScreenshot", "作为客户端连接状态截图"],
  ]) {
    requireEvidenceFile(record.data, path, label, missing);
  }

  requireNonEmpty(record.data, "sampleData.importedWorkbookName", "导入样本工作簿名不能为空", missing);
  requireNonEmpty(record.data, "sampleData.exportedReportMonth", "导出报表月份不能为空", missing);
  requireNonEmpty(record.data, "sampleData.backupRecordId", "备份记录编号不能为空", missing);
  requireNonNegativeInteger(record.data, "sampleData.restoredItemCount", "恢复后物品数量必须为非负整数", missing);
  requireNonNegativeInteger(record.data, "sampleData.restoredMovementCount", "恢复后库存流水数量必须为非负整数", missing);
  requireNonEmpty(record.data, "sampleData.asHostBusinessDocumentNo", "作为主机验收的客户端写入单据号不能为空", missing);
  requireNonEmpty(record.data, "sampleData.asClientBusinessDocumentNo", "作为客户端验收的本机写入单据号不能为空", missing);

  const expectedRestoreSource = platformName === "windows" ? "macos" : "windows";
  if (valueAt(record.data, "restore.restoredFromPlatform") !== expectedRestoreSource) {
    missing.push(`恢复来源平台必须为 ${expectedRestoreSource}`);
  }

  return {
    platform: platformName,
    status: missing.length === 0 ? "accepted" : "incomplete",
    recordPath: record.path,
    evidence: {
      verifyReleaseReport: resolvedReleaseReport,
      executionCoverageReport: resolvedCoverageReport,
      validation: evidenceValidation,
    },
    missing,
  };
});

for (const record of records) {
  if (record.data.__invalidJson) {
    findings.push({
      status: "invalid-json",
      path: record.path,
      message: record.data.__invalidJson,
    });
    continue;
  }
  const fileName = basename(record.path);
  if (fileName.includes("-windows-") && record.data.platform !== "windows") {
    findings.push({
      status: "platform-mismatch",
      path: record.path,
      message: "文件名是 windows 验收记录，但 JSON platform 不是 windows",
    });
  }
  if (fileName.includes("-macos-") && record.data.platform !== "macos") {
    findings.push({
      status: "platform-mismatch",
      path: record.path,
      message: "文件名是 macOS 验收记录，但 JSON platform 不是 macos",
    });
  }
}

function hostClientStatus(hostPlatform, clientPlatform) {
  const hostRecord = latestRecordForPlatform(hostPlatform);
  const clientRecord = latestRecordForPlatform(clientPlatform);
  const missing = [];
  if (!hostRecord) missing.push(`缺少 ${hostPlatform} 主机记录`);
  if (!clientRecord) missing.push(`缺少 ${clientPlatform} 客户端记录`);
  if (hostRecord) {
    const prefix = "hostClient.asHost";
    const checks = [
      ["tested", "主机侧未标记已测试"],
      ["pairCodeDisplayed", "主机侧未记录配对码显示"],
      ["peerPaired", "主机侧未记录客户端配对成功"],
      ["disconnectDetectedByPeer", "主机侧未记录客户端断线检测"],
      ["reconnectDetectedByPeer", "主机侧未记录客户端重连检测"],
      ["peerBusinessWriteSucceeded", "主机侧未记录客户端业务写入成功"],
      ["hostInventoryAndReportsConsistent", "主机侧未记录库存和报表一致"],
    ];
    for (const [key, label] of checks) {
      if (!boolAt(hostRecord.data, `${prefix}.${key}`)) missing.push(label);
    }
    if (valueAt(hostRecord.data, `${prefix}.peerPlatform`) !== clientPlatform) {
      missing.push(`主机侧 peerPlatform 必须为 ${clientPlatform}`);
    }
    if (!isNonEmptyString(valueAt(hostRecord.data, `${prefix}.hostIp`))) {
      missing.push("主机侧 hostIp 不能为空");
    }
    if (!isValidPort(valueAt(hostRecord.data, `${prefix}.hostPort`))) {
      missing.push("主机侧 hostPort 必须在 1024-65535 范围内");
    }
  }
  if (clientRecord) {
    const prefix = "hostClient.asClient";
    const checks = [
      ["tested", "客户端侧未标记已测试"],
      ["connectedByDiscoveryOrManualAddress", "客户端侧未记录发现或手动连接成功"],
      ["paired", "客户端侧未记录配对成功"],
      ["disconnectDetected", "客户端侧未记录断线检测"],
      ["reconnectDetected", "客户端侧未记录重连检测"],
      ["clientBusinessWriteSucceeded", "客户端侧未记录业务写入成功"],
      ["hostInventoryAndReportsConsistent", "客户端侧未记录主机库存和报表一致"],
    ];
    for (const [key, label] of checks) {
      if (!boolAt(clientRecord.data, `${prefix}.${key}`)) missing.push(label);
    }
    if (valueAt(clientRecord.data, `${prefix}.peerPlatform`) !== hostPlatform) {
      missing.push(`客户端侧 peerPlatform 必须为 ${hostPlatform}`);
    }
  }
  return {
    hostPlatform,
    clientPlatform,
    status: missing.length === 0 ? "accepted" : "incomplete",
    missing,
  };
}

const hostClientSummaries = [
  hostClientStatus("windows", "macos"),
  hostClientStatus("macos", "windows"),
];

const complete =
  findings.length === 0 &&
  platformSummaries.every((item) => item.status === "accepted") &&
  hostClientSummaries.every((item) => item.status === "accepted");

function buildRemainingEvidence() {
  const items = [];
  const add = (id, label, owner, missing) => {
    const normalizedMissing = [...new Set(missing.filter(Boolean))];
    if (normalizedMissing.length === 0) return;
    items.push({
      id,
      label,
      owner,
      status: "missing-evidence",
      missing: normalizedMissing,
    });
  };

  for (const summary of platformSummaries) {
    if (summary.status === "accepted") continue;
    const missing = summary.missing ?? [];
    add(`${summary.platform}_record`, `${summary.platform} 手工验收记录`, summary.platform, missing);
    add(
      `${summary.platform}_install`,
      `${summary.platform} 安装、首次启动、登录和建库证据`,
      summary.platform,
      missing.filter((item) =>
        item.includes("安装") ||
        item.includes("启动") ||
        item.includes("登录") ||
        item.includes("数据库") ||
        item.includes("系统应用数据目录"),
      ),
    );
    add(
      `${summary.platform}_excel`,
      `${summary.platform} Excel 导入导出证据`,
      summary.platform,
      missing.filter((item) => item.includes("Excel") || item.includes("工作簿") || item.includes("报表月份")),
    );
    add(
      `${summary.platform}_backup_restore`,
      `${summary.platform} 备份、恢复和冗灾证据`,
      summary.platform,
      missing.filter((item) =>
        item.includes("备份") ||
        item.includes("恢复") ||
        item.includes("健康检查") ||
        item.includes("数据一致") ||
        item.includes("SHA256"),
      ),
    );
    add(
      `${summary.platform}_screenshots`,
      `${summary.platform} 截图或附件归档`,
      summary.platform,
      missing.filter((item) => item.includes("截图")),
    );
  }

  for (const summary of hostClientSummaries) {
    if (summary.status === "accepted") continue;
    add(
      `${summary.hostPlatform}_host_${summary.clientPlatform}_client`,
      `${summary.hostPlatform} 主机 + ${summary.clientPlatform} 客户端互联证据`,
      `${summary.hostPlatform}/${summary.clientPlatform}`,
      summary.missing ?? [],
    );
  }

  return {
    status: items.length === 0 ? "complete" : "missing-evidence",
    remainingCount: items.length,
    items,
  };
}

const remainingEvidence = buildRemainingEvidence();

mkdirSync(evidenceDir, { recursive: true });
const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
const reportPath = join(evidenceDir, `manual-acceptance-summary-${process.platform}-${timestamp}.json`);
const report = {
  generatedAt: new Date().toISOString(),
  platform: platform(),
  platformRelease: release(),
  arch: arch(),
  strict,
  acceptanceDir,
  status: complete ? "accepted" : "incomplete",
  remainingEvidence,
  platformSummaries,
  hostClientSummaries,
  findings,
};
writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);

if (!complete) {
  console.log("[verify-manual-acceptance] Manual acceptance is incomplete.");
  for (const finding of findings) {
    console.log(`- ${finding.status}: ${finding.path}；${finding.message}`);
  }
  for (const item of platformSummaries) {
    if (item.status !== "accepted") {
      console.log(`- ${item.platform}: ${item.missing.join("；")}`);
    }
  }
  for (const item of hostClientSummaries) {
    if (item.status !== "accepted") {
      console.log(`- ${item.hostPlatform} host + ${item.clientPlatform} client: ${item.missing.join("；")}`);
    }
  }
  if (remainingEvidence.items.length > 0) {
    console.log("[verify-manual-acceptance] Remaining evidence groups:");
    for (const item of remainingEvidence.items) {
      console.log(`- ${item.label}: ${item.missing.length} 项`);
    }
  }
  console.log(`[verify-manual-acceptance] Summary report: ${reportPath}`);
  if (strict) process.exit(1);
} else {
  console.log("[verify-manual-acceptance] Manual acceptance is complete.");
  console.log(`[verify-manual-acceptance] Summary report: ${reportPath}`);
}
