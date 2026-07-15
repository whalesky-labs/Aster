import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { arch, platform, release } from "node:os";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";

const root = process.cwd();
const outputDir = join(root, "docs", "acceptance-package");
const docsDir = join(outputDir, "docs");
const evidenceDir = join(outputDir, "release-evidence");
const templatesDir = join(outputDir, "manual-acceptance");
const currentPlatform = process.env.ASTER_ACCEPTANCE_PLATFORM || platform();
const evidencePlatform =
  currentPlatform === "darwin" ? "darwin" : currentPlatform === "win32" ? "win32" : currentPlatform;
const manualPlatform = evidencePlatform === "darwin" ? "macos" : evidencePlatform === "win32" ? "windows" : evidencePlatform;

if (existsSync(outputDir)) {
  rmSync(outputDir, { recursive: true, force: true });
}
mkdirSync(docsDir, { recursive: true });
mkdirSync(evidenceDir, { recursive: true });
mkdirSync(templatesDir, { recursive: true });

const runnersResult = spawnSync(process.execPath, ["scripts/create-acceptance-runners.mjs", "--package"], {
  cwd: root,
  encoding: "utf8",
  stdio: "inherit",
});
if (runnersResult.status !== 0) {
  process.exit(runnersResult.status ?? 1);
}

function listFiles(dir, predicate) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter(predicate)
    .map((name) => join(dir, name))
    .sort();
}

function latest(prefix) {
  return listFiles(join(root, "docs", "release-evidence"), (name) =>
    name.startsWith(prefix) && name.endsWith(".json"),
  ).sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs || b.localeCompare(a))[0];
}

function latestManualAcceptance(prefix, suffix) {
  const manualDir = join(root, "docs", "manual-acceptance");
  if (!existsSync(manualDir)) return null;
  return readdirSync(manualDir, { recursive: true })
    .filter((name) => name.startsWith(prefix) && name.endsWith(suffix))
    .map((name) => join(manualDir, name))
    .sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs || b.localeCompare(a))[0];
}

function copyIfExists(source, targetDir, targetName = basename(source)) {
  if (!source || !existsSync(source)) return null;
  const target = join(targetDir, targetName);
  mkdirSync(dirname(target), { recursive: true });
  copyFileSync(source, target);
  return target;
}

function readJsonIfExists(path) {
  if (!path || !existsSync(path)) return null;
  return JSON.parse(readFileSync(path, "utf8"));
}

function collectFiles(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    return entry.isDirectory() ? collectFiles(path) : [path];
  });
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function packageRelative(path) {
  return path.replace(root, "").replace(/^[/\\]/, "").replaceAll("\\", "/");
}

const docSources = [
  "README.md",
  "docs/ASTER_EXECUTION_PLAN.md",
  "docs/ASTER_CROSS_PLATFORM_ACCEPTANCE.md",
  "docs/ASTER_DISASTER_RECOVERY_RUNBOOK.md",
  "docs/manual-acceptance/README.md",
];
const copiedDocs = docSources
  .map((relativePath) =>
    copyIfExists(join(root, relativePath), docsDir, relativePath.replaceAll("/", "__")),
  )
  .filter(Boolean);

const evidenceSources = [
  latest(`verify-all-local-${evidencePlatform}`),
  latest(`verify-release-${evidencePlatform}`),
  latest(`execution-coverage-${evidencePlatform}`),
  latest(`no-placeholder-scan-${evidencePlatform}`),
  latest(`readiness-${evidencePlatform}`),
  latest(`manual-acceptance-summary-${evidencePlatform}`),
].filter(Boolean);
const copiedEvidence = evidenceSources.map((source) => copyIfExists(source, evidenceDir)).filter(Boolean);
const latestManualSummary = readJsonIfExists(latest(`manual-acceptance-summary-${evidencePlatform}`));
const latestManualRecord = latestManualAcceptance(`manual-acceptance-${manualPlatform}-`, ".json");
const latestManualChecklist = latestManualAcceptance(`manual-acceptance-${manualPlatform}-`, "-checklist.md");
const latestAttachmentReadme = latestManualAcceptance(`evidence-${manualPlatform}-`, "README.md");
const manualSources = [latestManualRecord, latestManualChecklist, latestAttachmentReadme].filter(Boolean);
const copiedManualAcceptance = manualSources
  .map((source) => {
    const relativeName = source.replace(join(root, "docs", "manual-acceptance"), "").replace(/^[/\\]/, "");
    return copyIfExists(source, templatesDir, relativeName);
  })
  .filter(Boolean);
const copiedManualAcceptanceRelative = copiedManualAcceptance.map((path) =>
  path.replace(templatesDir, "").replace(/^[/\\]/, "").replaceAll("\\", "/"),
);
const copiedEvidenceRelative = copiedEvidence.map((path) =>
  path.replace(evidenceDir, "").replace(/^[/\\]/, "").replaceAll("\\", "/"),
);

const windowsTemplate = {
  schemaVersion: 1,
  platform: "windows",
  source: "Run `npm run acceptance:template -- windows` on the Windows machine to generate the authoritative dated JSON.",
};
const macosTemplate = {
  schemaVersion: 1,
  platform: "macos",
  source: "Run `npm run acceptance:template -- macos` on the macOS machine to generate the authoritative dated JSON.",
};
writeFileSync(join(templatesDir, "manual-acceptance-windows-template-note.json"), `${JSON.stringify(windowsTemplate, null, 2)}\n`);
writeFileSync(join(templatesDir, "manual-acceptance-macos-template-note.json"), `${JSON.stringify(macosTemplate, null, 2)}\n`);

const packageManifest = {
  generatedAt: new Date().toISOString(),
  generatedOn: {
    platform: currentPlatform,
    platformRelease: release(),
    arch: arch(),
    evidencePlatform,
  },
  purpose: "Aster Windows/macOS 双端实机验收交接包",
  commands: {
    windows: [
      "Set-ExecutionPolicy -Scope Process Bypass",
      "$env:GITHUB_TOKEN = '<GitHub token with Actions read access>'",
      "npm run acceptance:run-github-build",
      "npm run acceptance:download-windows-artifacts",
      ".\\docs\\acceptance-package\\runners\\run-windows-acceptance.ps1 -Force",
      ".\\docs\\acceptance-package\\runners\\run-windows-acceptance.ps1 -Force -ArtifactsDir C:\\path\\to\\downloaded-artifacts",
      "填写 docs/manual-acceptance/manual-acceptance-windows-YYYY-MM-DD.json",
    ],
    macos: [
      "chmod +x docs/acceptance-package/runners/run-macos-acceptance.sh",
      "./docs/acceptance-package/runners/run-macos-acceptance.sh --force",
      "填写 docs/manual-acceptance/manual-acceptance-macos-YYYY-MM-DD.json",
    ],
    finalCheck: [
      "把 Windows 与 macOS 的 release-evidence 和 manual-acceptance JSON 放回同一份项目目录",
      "npm run verify:manual-acceptance -- --strict",
    ],
  },
  copiedDocs: copiedDocs.map(packageRelative),
  copiedEvidence: copiedEvidence.map(packageRelative),
  copiedManualAcceptance: copiedManualAcceptance.map(packageRelative),
  remainingEvidence: latestManualSummary?.remainingEvidence ?? null,
  remainingEvidenceRequired: [
    "Windows GitHub Actions 安装器下载、安装和首次启动",
    "Windows 主机 + macOS 客户端连接、断线、恢复和业务写入",
    "macOS 主机 + Windows 客户端连接、断线、恢复和业务写入",
    "Windows/macOS 双端备份恢复",
    "Windows/macOS 双端 Excel 导入导出后人工打开确认",
  ],
};

function remainingEvidenceMarkdown(remainingEvidence) {
  const lines = [
    "# Aster 剩余实机验收证据清单",
    "",
    `生成时间：${packageManifest.generatedAt}`,
    "",
  ];
  if (!remainingEvidence || remainingEvidence.status === "complete") {
    lines.push("当前严格手工验收没有剩余证据项。");
    return `${lines.join("\n")}\n`;
  }
  lines.push(`剩余证据组数：${remainingEvidence.remainingCount}`);
  lines.push("");
  for (const item of remainingEvidence.items ?? []) {
    lines.push(`## ${item.label}`);
    lines.push("");
    lines.push(`- 负责人/平台：${item.owner}`);
    lines.push(`- 状态：${item.status}`);
    lines.push(`- 缺项数量：${item.missing?.length ?? 0}`);
    lines.push("");
    for (const missing of item.missing ?? []) {
      lines.push(`- [ ] ${missing}`);
    }
    lines.push("");
  }
  lines.push("补齐后，把 Windows 与 macOS 两端的 `docs/manual-acceptance/*.json`、`docs/manual-acceptance/evidence-*` 和 `docs/release-evidence/*.json` 放回同一份项目目录，再执行：");
  lines.push("");
  lines.push("```bash");
  lines.push("npm run verify:manual-acceptance -- --strict");
  lines.push("```");
  return `${lines.join("\n")}\n`;
}

writeFileSync(join(outputDir, "REMAINING_EVIDENCE.md"), remainingEvidenceMarkdown(packageManifest.remainingEvidence));

const readme = `# Aster 双端实机验收交接包

生成时间：${packageManifest.generatedAt}

## 使用方式

1. 在 GitHub Actions 的 \`Build Desktop Bundles\` workflow 运行完成后，下载以下 artifacts 并放回项目目录：

- \`aster-windows-x64\`：Windows 安装器产物。
- \`aster-windows-x64-release-evidence\`：Windows 自动发布验证和覆盖证据。

然后在 Windows 电脑上执行：

\`\`\`powershell
npm install
npm run acceptance:import-windows-artifacts -- C:\\path\\to\\downloaded-artifacts
npm run acceptance:collect -- windows --force
\`\`\`

如果当前环境有具备 Actions read 权限的 \`GITHUB_TOKEN\` 或 \`GH_TOKEN\`，也可以自动下载并导入最新成功 workflow run 的 Windows artifacts：

\`\`\`powershell
$env:GITHUB_TOKEN = "<github-token>"
npm install
npm run acceptance:download-windows-artifacts
npm run acceptance:collect -- windows --force
\`\`\`

如果分支已推送到 GitHub，也可以自动触发远端构建、轮询结果、下载并导入：

\`\`\`powershell
$env:GITHUB_TOKEN = "<github-token>"
$env:ASTER_GITHUB_BRANCH = "codex/aster-full-execution"
npm install
npm run acceptance:run-github-build
npm run acceptance:collect -- windows --force
\`\`\`

或者使用交接包中的 PowerShell 脚本：

\`\`\`powershell
Set-ExecutionPolicy -Scope Process Bypass
.\\docs\\acceptance-package\\runners\\run-windows-acceptance.ps1 -Force -ArtifactsDir C:\\path\\to\\downloaded-artifacts
\`\`\`

PowerShell 脚本检测到 \`GITHUB_TOKEN\` 或 \`GH_TOKEN\` 时，也会自动执行 \`npm run acceptance:run-github-build\`。

2. 在 macOS 电脑上执行：

\`\`\`bash
npm install
npm run verify:release
npm run acceptance:collect -- macos --force
\`\`\`

或者使用交接包中的 shell 脚本：

\`\`\`bash
chmod +x docs/acceptance-package/runners/run-macos-acceptance.sh
./docs/acceptance-package/runners/run-macos-acceptance.sh --force
\`\`\`

3. 按 \`docs/ASTER_CROSS_PLATFORM_ACCEPTANCE.md\` 和自动生成的 \`docs/manual-acceptance/*-checklist.md\` 完成安装、启动、登录、建库、Excel、备份恢复和互为主机/客户端验收。交接包内的 \`manual-acceptance/\` 目录会携带当前平台最近一次采集出的 JSON、checklist 和附件目录 README，可作为现场填写样例。

4. 填写两端 \`docs/manual-acceptance/manual-acceptance-*.json\`，并把两端 \`docs/release-evidence/*.json\` 放回同一份项目目录。

5. 查看 \`REMAINING_EVIDENCE.md\` 或 \`acceptance-package-manifest.json\` 里的 \`remainingEvidence\`，逐组补齐仍缺的实机记录、截图、Excel 打开确认、备份恢复和主机/客户端互联证据。

6. 正式归档前执行：

\`\`\`bash
npm run verify:manual-acceptance -- --strict
\`\`\`

严格校验通过后，才表示 Windows/macOS 双端实机验收证据完整。

## 当前携带的验收样例

${copiedManualAcceptanceRelative.length > 0 ? copiedManualAcceptanceRelative.map((path) => `- \`manual-acceptance/${path}\``).join("\n") : "- 暂无当前平台手工验收样例，请先运行 `npm run acceptance:collect -- 平台 --force`。"}

## 当前携带的自动证据

${copiedEvidenceRelative.length > 0 ? copiedEvidenceRelative.map((path) => `- \`release-evidence/${path}\``).join("\n") : "- 暂无当前平台自动证据，请先运行 `npm run verify:release`。"}

\`npm run verify:release\` 已包含 \`npm run verify:no-placeholders\`，交接包校验会拒收缺少该命令或该命令失败的 release 证据。
`;
writeFileSync(join(outputDir, "README.md"), readme);

const manifestPath = join(outputDir, "acceptance-package-manifest.json");
packageManifest.fileInventory = collectFiles(outputDir)
  .filter((path) => path !== manifestPath)
  .sort()
  .map((path) => ({
    path: packageRelative(path),
    bytes: statSync(path).size,
    sha256: sha256(path),
  }));
packageManifest.fileInventorySha256 = createHash("sha256")
  .update(JSON.stringify(packageManifest.fileInventory))
  .digest("hex");
writeFileSync(manifestPath, `${JSON.stringify(packageManifest, null, 2)}\n`);

console.log(`[acceptance-package] Created: ${outputDir}`);
