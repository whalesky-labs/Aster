import { chmodSync, existsSync, mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const root = process.cwd();
const outputDir = process.argv.includes("--package")
  ? join(root, "docs", "acceptance-package", "runners")
  : join(root, "docs", "manual-acceptance", "runners");

mkdirSync(outputDir, { recursive: true });

const windowsScript = String.raw`param(
  [switch]$Force,
  [string]$ArtifactsDir = ""
)

$ErrorActionPreference = "Stop"

function Step($Message) {
  Write-Host ""
  Write-Host "==> $Message" -ForegroundColor Cyan
}

Step "安装依赖"
npm install

Step "确认 GitHub Actions Windows 构建证据"
if ($ArtifactsDir -and $ArtifactsDir.Trim().Length -gt 0) {
  npm run acceptance:import-windows-artifacts -- $ArtifactsDir
} elseif ($env:GITHUB_TOKEN -or $env:GH_TOKEN) {
  npm run acceptance:run-github-build
} else {
  Write-Host "请先从 Build Desktop Bundles workflow 下载 aster-windows-x64 和 aster-windows-x64-release-evidence artifacts。" -ForegroundColor Yellow
  Write-Host "如果需要脚本自动触发 GitHub 构建并下载，请先设置 GITHUB_TOKEN 或 GH_TOKEN，然后重新运行本脚本。" -ForegroundColor Yellow
  Write-Host "如需自动导入，请添加 -ArtifactsDir 参数，例如：.\\docs\\acceptance-package\\runners\\run-windows-acceptance.ps1 -Force -ArtifactsDir C:\\path\\to\\downloaded-artifacts" -ForegroundColor Yellow
}
Write-Host "Windows 现场脚本只采集安装、启动和人工验收记录，不在现场重新打包。" -ForegroundColor Yellow

Step "生成 Windows 实机验收 JSON 和清单"
if ($Force) {
  npm run acceptance:collect -- windows --force
} else {
  npm run acceptance:collect -- windows
}

Step "生成当前剩余证据汇总"
try {
  npm run verify:manual-acceptance -- --strict
} catch {
  Write-Host "严格手工验收尚未通过；已生成 remainingEvidence 汇总，继续生成交接包。" -ForegroundColor Yellow
}

Step "生成并校验验收交接包"
npm run acceptance:package
npm run verify:readiness
npm run acceptance:package
npm run verify:acceptance-package
npm run acceptance:archive

Step "下一步"
Write-Host "1. 按 docs/manual-acceptance/manual-acceptance-windows-*-checklist.md 完成实机勾验。"
Write-Host "2. 填写 docs/manual-acceptance/manual-acceptance-windows-YYYY-MM-DD.json。"
Write-Host "3. 保留 docs/acceptance-package/、docs/acceptance-archives/*.zip、docs/release-evidence/*.json、docs/manual-acceptance/*.json 和 evidence-* 附件目录。"
Write-Host "4. 在合并后的项目根目录执行 npm run verify:manual-acceptance -- --strict。"
`;

const macosScript = `#!/usr/bin/env bash
set -euo pipefail

step() {
  printf '\\n==> %s\\n' "$1"
}

force_arg=""
if [[ "\${1:-}" == "--force" ]]; then
  force_arg="--force"
fi

step "安装依赖"
npm install

step "执行完整发布验证"
npm run verify:release

step "生成 macOS 实机验收 JSON 和清单"
npm run acceptance:collect -- macos \${force_arg}

step "生成当前剩余证据汇总"
if ! npm run verify:manual-acceptance -- --strict; then
  echo "严格手工验收尚未通过；已生成 remainingEvidence 汇总，继续生成交接包。"
fi

step "生成并校验验收交接包"
npm run acceptance:package
npm run verify:readiness
npm run acceptance:package
npm run verify:acceptance-package
npm run acceptance:archive

step "下一步"
echo "1. 按 docs/manual-acceptance/manual-acceptance-macos-*-checklist.md 完成实机勾验。"
echo "2. 填写 docs/manual-acceptance/manual-acceptance-macos-YYYY-MM-DD.json。"
echo "3. 保留 docs/acceptance-package/、docs/acceptance-archives/*.zip、docs/release-evidence/*.json、docs/manual-acceptance/*.json 和 evidence-* 附件目录。"
echo "4. 在合并后的项目根目录执行 npm run verify:manual-acceptance -- --strict。"
`;

const readme = `# Aster 实机验收运行脚本

这些脚本用于 Windows/macOS 实机验收。Windows 脚本默认使用 GitHub Actions 生成的 Windows 安装器和 release evidence，只执行依赖安装、验收 JSON 模板生成、逐项清单生成、剩余证据汇总、readiness 检查、交接包自检和归档包生成；macOS 脚本会在本机执行完整发布验证。Windows 脚本检测到 \`GITHUB_TOKEN\` 或 \`GH_TOKEN\` 时，会自动触发 \`Build Desktop Bundles\` workflow，等待成功后下载 Windows artifacts 并导入。

从项目根目录执行 Windows PowerShell：

\`\`\`powershell
Set-ExecutionPolicy -Scope Process Bypass
.\\docs\\acceptance-package\\runners\\run-windows-acceptance.ps1 -Force
\`\`\`

如果已经下载 GitHub Actions artifacts，可以让脚本自动导入：

\`\`\`powershell
Set-ExecutionPolicy -Scope Process Bypass
.\\docs\\acceptance-package\\runners\\run-windows-acceptance.ps1 -Force -ArtifactsDir C:\\path\\to\\downloaded-artifacts
\`\`\`

如果没有手动下载 artifacts，也可以设置 token 后让脚本自动触发构建并下载：

\`\`\`powershell
$env:GITHUB_TOKEN = "<github-token>"
Set-ExecutionPolicy -Scope Process Bypass
.\\docs\\acceptance-package\\runners\\run-windows-acceptance.ps1 -Force
\`\`\`

如果当前目录已经是 \`docs/acceptance-package\`，也可以执行：

\`\`\`powershell
Set-ExecutionPolicy -Scope Process Bypass
.\\runners\\run-windows-acceptance.ps1 -Force
\`\`\`

从项目根目录执行 macOS：

\`\`\`bash
chmod +x docs/acceptance-package/runners/run-macos-acceptance.sh
./docs/acceptance-package/runners/run-macos-acceptance.sh --force
\`\`\`

如果当前目录已经是 \`docs/acceptance-package\`，也可以执行：

\`\`\`bash
chmod +x runners/run-macos-acceptance.sh
./runners/run-macos-acceptance.sh --force
\`\`\`

脚本不会自动把实机项目勾为通过；安装、启动、登录、Excel、备份恢复和互为主机/客户端结果仍需要验收人员按清单实测后填写 JSON。严格手工验收未通过时，脚本会继续生成 remainingEvidence、readiness、交接包完整性证据和归档包 SHA256 报告。
`;

const windowsPath = join(outputDir, "run-windows-acceptance.ps1");
const macosPath = join(outputDir, "run-macos-acceptance.sh");
const readmePath = join(outputDir, "README.md");

writeFileSync(windowsPath, `${windowsScript}\n`);
writeFileSync(macosPath, macosScript);
writeFileSync(readmePath, readme);

if (existsSync(macosPath)) {
  chmodSync(macosPath, 0o755);
}

console.log(`[acceptance-runners] Created: ${outputDir}`);
