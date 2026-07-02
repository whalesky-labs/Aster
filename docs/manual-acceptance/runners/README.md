# Aster 实机验收运行脚本

这些脚本用于 Windows/macOS 实机验收。Windows 脚本默认使用 GitHub Actions 生成的 Windows 安装器和 release evidence，只执行依赖安装、验收 JSON 模板生成、逐项清单生成、剩余证据汇总、readiness 检查、交接包自检和归档包生成；macOS 脚本会在本机执行完整发布验证。

从项目根目录执行 Windows PowerShell：

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\docs\acceptance-package\runners\run-windows-acceptance.ps1 -Force
```

如果已经下载 GitHub Actions artifacts，可以让脚本自动导入：

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\docs\acceptance-package\runners\run-windows-acceptance.ps1 -Force -ArtifactsDir C:\path\to\downloaded-artifacts
```

如果当前目录已经是 `docs/acceptance-package`，也可以执行：

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\runners\run-windows-acceptance.ps1 -Force
```

从项目根目录执行 macOS：

```bash
chmod +x docs/acceptance-package/runners/run-macos-acceptance.sh
./docs/acceptance-package/runners/run-macos-acceptance.sh --force
```

如果当前目录已经是 `docs/acceptance-package`，也可以执行：

```bash
chmod +x runners/run-macos-acceptance.sh
./runners/run-macos-acceptance.sh --force
```

脚本不会自动把实机项目勾为通过；安装、启动、登录、Excel、备份恢复和互为主机/客户端结果仍需要验收人员按清单实测后填写 JSON。严格手工验收未通过时，脚本会继续生成 remainingEvidence、readiness、交接包完整性证据和归档包 SHA256 报告。
