import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { arch, hostname, platform as osPlatform, release } from "node:os";
import { spawnSync } from "node:child_process";

const root = process.cwd();
const acceptanceDir = join(root, "docs", "manual-acceptance");
const evidenceDir = join(root, "docs", "release-evidence");
mkdirSync(acceptanceDir, { recursive: true });

const requestedPlatform = (process.argv[2] ?? process.platform).toLowerCase();
const normalizedPlatform = requestedPlatform.startsWith("win")
  ? "windows"
  : requestedPlatform.startsWith("darwin") || requestedPlatform.startsWith("mac")
    ? "macos"
    : requestedPlatform;
const evidencePlatform = normalizedPlatform === "macos" ? "darwin" : normalizedPlatform === "windows" ? "win32" : normalizedPlatform;
const date = new Date().toISOString().slice(0, 10);
const jsonPath = join(acceptanceDir, `manual-acceptance-${normalizedPlatform}-${date}.json`);
const jsonPathRelative = `docs/manual-acceptance/manual-acceptance-${normalizedPlatform}-${date}.json`;
const checklistPath = join(acceptanceDir, `manual-acceptance-${normalizedPlatform}-${date}-checklist.md`);
const attachmentDirName = `evidence-${normalizedPlatform}-${date}`;
const attachmentDir = join(acceptanceDir, attachmentDirName);
const attachmentDirRelative = `docs/manual-acceptance/${attachmentDirName}`;

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

function runTemplate() {
  if (existsSync(jsonPath) && !process.argv.includes("--force")) {
    return { status: 0, skipped: true };
  }
  const args = ["scripts/create-manual-acceptance-template.mjs", normalizedPlatform];
  if (process.argv.includes("--force")) args.push("--force");
  const result = spawnSync(process.execPath, args, {
    cwd: root,
    stdio: "inherit",
  });
  return { status: result.status ?? 1, skipped: false };
}

const templateResult = runTemplate();
if (templateResult.status !== 0) {
  process.exit(templateResult.status);
}

const releaseReportPath = listEvidence("verify-release")[0] ?? "";
const coverageReportPath = listEvidence("execution-coverage")[0] ?? "";
const releaseReport = readJson(releaseReportPath);
const coverageReport = readJson(coverageReportPath);
const artifact = releaseReport?.artifacts?.find((item) => item.type === "file") ?? releaseReport?.artifacts?.[0];
const peerPlatform = normalizedPlatform === "windows" ? "macos" : "windows";
const asHostPeer = peerPlatform;
const asClientHost = peerPlatform;
mkdirSync(attachmentDir, { recursive: true });

const attachmentReadmePath = join(attachmentDir, "README.md");
const attachmentReadme = `# Aster ${normalizedPlatform} 实机验收附件

把本机验收截图或附件放在本目录，默认文件名如下。若现场使用其他文件名，请同步更新 \`${jsonPathRelative}\` 的 \`evidenceFiles\` 字段。

- \`01-install-or-launch.png\`：安装器、安装结果或应用首次启动
- \`02-admin-login.png\`：默认管理员登录成功
- \`03-database-location.png\`：SQLite 创建在系统应用数据目录
- \`04-excel-import-preview.png\`：旧 Excel 导入预览
- \`05-exported-workbook-opened.png\`：导出的 Excel 工作簿已打开
- \`06-backup-record.png\`：备份记录、备份路径、来源主机和 SHA256
- \`07-restore-preview.png\`：恢复预览校验通过
- \`08-restore-result.png\`：恢复完成、健康检查和数据一致
- \`09-host-mode.png\`：本机作为主机的连接状态
- \`10-client-mode.png\`：本机作为客户端的连接状态
`;
writeFileSync(attachmentReadmePath, attachmentReadme);

const checklist = `# Aster ${normalizedPlatform} 实机验收采集清单

生成时间：${new Date().toISOString()}

## 机器信息

- 电脑名称：${hostname()}
- 系统：${osPlatform()} ${release()}
- CPU：${arch()}
- 平台：${normalizedPlatform}
- 对端平台：${peerPlatform}

## 自动证据

- 手工验收 JSON：\`${jsonPath}\`
- verify-release：\`${releaseReportPath || (normalizedPlatform === "windows" ? "未生成，请先从 GitHub Actions 下载 aster-windows-x64-release-evidence" : "未生成，先运行 npm run verify:release")}\`
- execution-coverage：\`${coverageReportPath || "未生成，先运行 npm run verify:coverage"}\`
- 截图/附件目录：\`${attachmentDirRelative}\`
- 应用版本：${releaseReport?.tauriConfig?.version ?? "未读取"}
- 打包产物：${artifact?.path ?? "未读取"}
- 打包产物 SHA256：${artifact?.sha256 ?? "未读取"}
- 功能覆盖状态：${coverageReport?.coverageStatus ?? "未读取"}

## 必须先执行

\`\`\`${normalizedPlatform === "windows" ? "powershell" : "bash"}
npm install
${normalizedPlatform === "windows" ? "# 从 GitHub Actions 的 Build Desktop Bundles 下载 aster-windows-x64 和 aster-windows-x64-release-evidence" : "npm run verify:release"}
${normalizedPlatform === "windows" ? "npm run acceptance:import-windows-artifacts -- C:\\path\\to\\downloaded-artifacts" : ""}
npm run acceptance:collect -- ${normalizedPlatform} --force
\`\`\`

## 本机安装与启动

- [ ] 安装包已生成，并与 verify-release 报告中的路径和 SHA256 一致。
- [ ] 安装器可打开或应用已安装。
- [ ] 首次启动成功。
- [ ] 默认管理员 \`admin / admin123\` 可登录。
- [ ] SQLite 数据库创建在系统应用数据目录，不在项目源码目录内。
- [ ] 把上述结果填写到 \`localInstall\`。
- [ ] 截图或附件保存到 \`${attachmentDirRelative}\`，并确认 \`evidenceFiles.installScreenshot\`、\`evidenceFiles.loginScreenshot\` 和 \`evidenceFiles.databaseLocationScreenshot\` 指向真实文件。

## Excel 导入导出

- [ ] 旧酒店 Excel 可进入导入预览，不直接写入正式库。
- [ ] 执行导入后生成物品、流水或导入报告。
- [ ] 月报 Excel 可导出。
- [ ] 导出的 Excel 文件可直接打开查看，不依赖本机安装 Microsoft Office 或 WPS。
- [ ] 把导出文件路径填写到 \`excel.exportedWorkbookPath\`。
- [ ] 把导入样本工作簿名、导出报表月份填写到 \`sampleData\`。
- [ ] 把导入预览截图和导出工作簿打开截图保存到 \`${attachmentDirRelative}\`，并确认 \`evidenceFiles.excelImportPreviewScreenshot\` 和 \`evidenceFiles.exportedWorkbookScreenshot\` 指向真实文件。

## 备份与恢复

- [ ] 手动备份生成 zip 包。
- [ ] 第二备份目录可写。
- [ ] 备份包包含 \`aster.sqlite\`、\`backup-meta.json\`、\`app-settings.json\` 和 \`import-reports/\`。
- [ ] 备份记录或恢复预览中已记录来源主机名和来源系统。
- [ ] 备份文件 SHA256 已记录。
- [ ] 执行恢复前系统已生成 \`before_restore\` 保护备份，并记录保护备份路径。
- [ ] 从 ${peerPlatform} 备份包恢复成功。
- [ ] 跨平台恢复使用的备份来源主机名已记录。
- [ ] 恢复后健康检查通过。
- [ ] 恢复后物品、库存、流水、报表数据一致。
- [ ] 把恢复前保护备份路径填写到 \`restore.beforeRestoreBackupFile\`，并勾选 \`restore.beforeRestoreBackupCreated\`。
- [ ] 把备份记录编号、恢复后物品数量、恢复后库存流水数量填写到 \`sampleData\`。
- [ ] 把备份记录、恢复预览和恢复结果截图保存到 \`${attachmentDirRelative}\`，并确认 \`evidenceFiles.backupRecordScreenshot\`、\`evidenceFiles.restorePreviewScreenshot\` 和 \`evidenceFiles.restoreResultScreenshot\` 指向真实文件。

## ${normalizedPlatform} 作为主机，${asHostPeer} 作为客户端

- [ ] 切换为主机模式并启动主机服务。
- [ ] 记录主机 IP、端口和 6 位配对码。
- [ ] ${asHostPeer} 客户端通过发现或手动地址连接。
- [ ] 配对成功。
- [ ] 临时断开主机服务或网络，客户端明确显示断线。
- [ ] 恢复主机服务后，客户端无需重启即可显示恢复连接。
- [ ] 客户端写入一笔业务单据。
- [ ] 主机侧库存流水和报表一致。
- [ ] 把结果填写到 \`hostClient.asHost\`，peerPlatform 填 \`${asHostPeer}\`。
- [ ] 把客户端写入单据号填写到 \`sampleData.asHostBusinessDocumentNo\`。
- [ ] 把主机连接状态截图保存到 \`${attachmentDirRelative}\`，并确认 \`evidenceFiles.hostModeScreenshot\` 指向真实文件。

## ${normalizedPlatform} 作为客户端，${asClientHost} 作为主机

- [ ] ${asClientHost} 启动主机服务并显示配对码。
- [ ] 本机切换为客户端模式。
- [ ] 通过发现或手动地址连接主机。
- [ ] 配对成功。
- [ ] 主机临时断开时，本机明确显示断线。
- [ ] 主机恢复后，本机无需重启即可显示恢复连接。
- [ ] 本机客户端写入一笔业务单据。
- [ ] 主机侧库存流水和报表一致。
- [ ] 把结果填写到 \`hostClient.asClient\`，peerPlatform 填 \`${asClientHost}\`。
- [ ] 把本机客户端写入单据号填写到 \`sampleData.asClientBusinessDocumentNo\`。
- [ ] 把客户端连接状态截图保存到 \`${attachmentDirRelative}\`，并确认 \`evidenceFiles.clientModeScreenshot\` 指向真实文件。

## 最终校验

把 Windows 和 macOS 两端的 \`docs/manual-acceptance/*.json\` 与 \`docs/release-evidence/*.json\` 放回同一份项目目录后执行：

\`\`\`bash
npm run verify:manual-acceptance -- --strict
\`\`\`

严格校验通过后，才表示 Windows/macOS 双端实机验收证据完整。
`;

writeFileSync(checklistPath, checklist);

console.log(`[acceptance-collect] JSON template: ${jsonPath}${templateResult.skipped ? " (existing)" : ""}`);
console.log(`[acceptance-collect] Checklist: ${checklistPath}`);
console.log(`[acceptance-collect] Attachment directory: ${attachmentDir}`);
