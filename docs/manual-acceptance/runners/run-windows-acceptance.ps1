param(
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
} else {
  Write-Host "请先从 Build Desktop Bundles workflow 下载 aster-windows-x64 和 aster-windows-x64-release-evidence artifacts。" -ForegroundColor Yellow
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
