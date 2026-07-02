# Aster 手工实机验收记录

本目录用于保存 Windows/macOS 实机验收 JSON。

生成模板：

```bash
npm run acceptance:template -- windows
npm run acceptance:template -- macos
```

生成模板和逐项采集清单：

```bash
npm run acceptance:collect -- windows --force
npm run acceptance:collect -- macos --force
```

生成给实机验收人员使用的 Windows PowerShell 与 macOS shell 运行脚本：

```bash
npm run acceptance:runners
```

macOS 建议先执行 `npm run verify:release`，再生成模板。Windows 需要先从 GitHub Actions 的 `Build Desktop Bundles` workflow 下载 `aster-windows-x64` 和 `aster-windows-x64-release-evidence` artifacts，再生成模板。模板会自动读取同平台最新 `verify-release` 和 `execution-coverage` 报告，并预填电脑名称、系统版本、CPU 架构、应用版本、安装包路径和安装包 SHA256。验收操作人、启动/登录/Excel/备份/恢复/主机客户端等实测结果仍需人工填写。
Windows 下载 artifacts 后建议先执行 `npm run acceptance:import-windows-artifacts -- 下载目录`，脚本会把 `verify-release-win32-*.json`、`execution-coverage-win32-*.json`、`no-placeholder-scan-win32-*.json` 复制到 `docs/release-evidence/`，把 `.exe/.msi` 复制到 `docs/manual-acceptance/windows-installers/`，并校验安装器 SHA256 与 release evidence 一致。
如果本机有具备 Actions read 权限的 `GITHUB_TOKEN` 或 `GH_TOKEN`，也可以执行 `npm run acceptance:download-windows-artifacts`，脚本会从 `whalesky-labs/Aster` 的最新成功 `Build Desktop Bundles` run 下载 `aster-windows-x64` 和 `aster-windows-x64-release-evidence`，解压后自动调用导入脚本完成 SHA256 校验。
如果当前分支已经推送到 GitHub，可执行 `npm run acceptance:run-github-build`，脚本会触发 `Build Desktop Bundles`、等待 workflow 完成、下载 Windows artifacts 并自动导入；该命令同样需要 `GITHUB_TOKEN` 或 `GH_TOKEN`。
也可以直接运行 `docs/acceptance-package/runners/run-windows-acceptance.ps1 -Force -ArtifactsDir 下载目录`，由一键脚本先导入 artifacts 再生成 Windows 验收 JSON 和清单。
`acceptance:collect` 会额外生成 `manual-acceptance-平台-日期-checklist.md`，用于现场逐项勾验安装、启动、Excel、备份恢复和互为主机/客户端连接。
`acceptance:collect` 还会创建 `docs/manual-acceptance/evidence-平台-日期/` 附件目录，并在 JSON 模板的 `evidenceFiles` 中预填该目录下的标准截图文件名。现场可以直接把截图放入该目录；如果使用其他文件名，需要同步更新 JSON 路径。
`acceptance:runners` 生成的脚本会执行发布验证、生成清单、尝试严格手工验收汇总、生成 readiness、刷新交接包、校验交接包完整性并生成归档包 SHA256 报告；脚本不会自动把任何实机项目标记为通过。

填写完成后校验：

```bash
npm run acceptance:status
npm run acceptance:status -- --summary
npm run verify:manual-acceptance
```

`acceptance:status` 不修改任何验收文件，只汇总最新自动证据、readiness、remainingEvidence、归档包 SHA256 和下一步命令，便于交接前快速确认还缺什么；现场人员只需要一屏摘要时使用 `npm run acceptance:status -- --summary`。

正式验收时使用严格模式：

```bash
npm run verify:manual-acceptance -- --strict
```

双端实机证据补齐后，可用最终归档入口一次完成严格验收、readiness、交接包自检和归档包生成：

```bash
npm run acceptance:finalize
```

如果仍缺实机证据，该命令会失败并生成 `docs/release-evidence/acceptance-finalize-*.json`，用于记录失败命令、最新 strict summary 和 remainingEvidence。

校验脚本会检查：

- Windows 和 macOS 都有手工验收记录。
- 双端都有 `verify-release` 与 `execution-coverage` 证据，并且证据报告的平台、版本、覆盖状态、功能组、安装包路径和安装包 SHA256 与手工记录一致。
- `verify-release` 证据必须记录已执行 `npm run build`、`npm run verify:coverage`、`npm run verify:no-placeholders`、`cargo fmt --check`、`cargo test` 和 Tauri build；GitHub Actions 可记录为 `npm run tauri -- build --target ...`。逐条命令执行结果必须全部成功，不能只填写命令清单。
- 汇总报告会在 `platformSummaries[].evidence.validation` 中记录 release 与 coverage 证据的逐项校验结果，便于复核验收 JSON 是否和自动证据自洽。
- 汇总报告会在 `remainingEvidence` 中按安装、Excel、备份恢复、截图归档和主机/客户端互联分组列出剩余证据，便于区分功能覆盖状态和实机证据缺口。
- 双端记录了验收操作人、电脑名称、系统版本、CPU 架构、应用版本、安装包路径和安装包 SHA256。
- 双端必须记录安装/登录/建库、Excel 导入导出、备份恢复、作为主机、作为客户端的截图或附件路径到 `evidenceFiles`；项目内相对路径会检查存在，跨设备绝对路径作为归档线索记录。
- 双端必须记录导入样本工作簿名、导出报表月份、备份记录编号、恢复后物品数量、恢复后库存流水数量，以及互为主机/客户端写入的业务单据号到 `sampleData`。
- 同一平台存在多份 `manual-acceptance-*.json` 时，校验脚本按 `generatedAt` 或文件修改时间选择最新记录；旧的空模板可以保留，但不会覆盖最新实测记录。
- 双端完成安装、首次启动、登录、建库、Excel 预览、Excel 正式导入生成记录、Excel 导出、手动备份、第二备份目录、恢复前 `before_restore` 保护备份和跨平台恢复。
- 双端备份记录必须填写来源主机名、来源系统和备份 SHA256；跨平台恢复记录必须填写恢复来源备份的主机名，便于主机故障切换时追溯备份来自哪台电脑。
- 勾选安装、导出、备份、恢复时，必须填写对应路径；恢复前保护备份路径填写到 `restore.beforeRestoreBackupFile`。如果填写的是当前项目内的相对路径，校验脚本会检查文件存在。跨设备绝对路径只做记录，不在另一台电脑上强制检查存在。
- 安装包 SHA256 和备份 SHA256 必须是 64 位十六进制。
- Windows 主机 + macOS 客户端已完成连接、配对、断线检测、重连和业务写入一致性检查。
- macOS 主机 + Windows 客户端已完成连接、配对、断线检测、重连和业务写入一致性检查。
- 主机侧必须记录主机 IP、端口和对端平台。

`manual-acceptance-*.json` 是实机验收产物，默认不纳入版本库。
