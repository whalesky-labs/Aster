# Aster Windows/macOS 双端验收文档

## 1. 验收目标

确认 Aster 在 Windows 和 macOS 双端都能完成安装、启动、建库、备份恢复、Excel 导入导出，以及互为局域网主机/客户端连接。

## 2. 当前已验证

macOS 本机已完成 Tauri release 打包；GitHub Actions 的 `Build Desktop Bundles` workflow 也会在 `macos-15-intel` 上使用 `x86_64-apple-darwin` target 生成 macOS Intel/x64 包：

- 应用产物：`src-tauri/target/release/bundle/macos/Aster.app`
- DMG 产物：`src-tauri/target/release/bundle/dmg/Aster_0.1.0_x64.dmg`

验证命令：

```bash
npm run verify:release
```

`verify:release` 会同时检查 `Aster.app`、DMG、`Info.plist` 元数据、bundle identifier、版本号、最低系统版本、可执行文件和 DMG 校验结果，并生成本地证据报告到 `docs/release-evidence/`。

发布验证还会自动运行 `npm run verify:coverage` 和 `npm run verify:no-placeholders`，生成执行文档功能覆盖证据报告和交付标记扫描报告，检查首页、基础资料、库存闭环、盘点、报表导出、Excel 导入、备份冗灾、用户权限、主机/客户端一致性、预算审批和跨平台打包等功能组是否仍有代码、命令、测试或文档证据支撑，并确认正式代码和文档没有遗留开发标记。

本机自动门禁可执行：

```bash
npm run verify:all-local
```

`verify:all-local` 会串行执行构建、手工验收脚本夹具测试、未完成标记扫描、执行覆盖、Rust 格式检查、Rust 测试、发布打包、验收交接包生成、readiness 检查、交接包自检和归档包生成，并生成 `docs/release-evidence/verify-all-local-*.json`。该命令只覆盖当前机器可自动验证的门禁；Windows/macOS 双端安装、截图、Excel 人工打开、跨平台恢复和互为主机/客户端仍以 `npm run verify:manual-acceptance -- --strict` 为最终归档门禁。

验收准备度可执行：

```bash
npm run verify:readiness
npm run acceptance:status
npm run acceptance:status -- --summary
```

`verify:readiness` 会检查最新自动证据和交接包是否齐全，并生成 `docs/release-evidence/readiness-*.json`。状态为 `ready-for-manual-evidence` 表示自动门禁已经可以交给现场继续补双端实机证据；最终归档仍必须通过严格手工验收。
`acceptance:status` 会只读汇总最新自动证据、readiness、remainingEvidence、归档包 SHA256 和下一步命令，适合交接前快速确认当前状态；现场人员只需要一屏摘要时使用 `npm run acceptance:status -- --summary`。

## 3. Windows 实机验收

Windows release 包在 GitHub Actions 中生成：进入 `Build Desktop Bundles` workflow，手动运行 `workflow_dispatch` 或推送 `v*` tag，等待 `windows-2022` 的 Windows x64 job 完成后下载 artifacts：

- `aster-windows-x64`：Windows 安装器产物。
- `aster-windows-x64-release-evidence`：`verify-release-win32-*.json`、`execution-coverage-win32-*.json`、`no-placeholder-scan-win32-*.json` 等证据报告。

下载 artifacts 并放回项目目录后，在 Windows 电脑上执行：

```powershell
npm install
npm run acceptance:import-windows-artifacts -- C:\path\to\downloaded-artifacts
npm run acceptance:collect -- windows --force
```

如果本机配置了具备 Actions read 权限的 `GITHUB_TOKEN` 或 `GH_TOKEN`，可不手动下载 artifacts，直接执行：

```powershell
$env:GITHUB_TOKEN = "<github-token>"
npm install
npm run acceptance:download-windows-artifacts
npm run acceptance:collect -- windows --force
```

`acceptance:download-windows-artifacts` 会读取 `whalesky-labs/Aster` 的最新成功 `Build Desktop Bundles` run，下载 `aster-windows-x64` 和 `aster-windows-x64-release-evidence`，解压到 `docs/manual-acceptance/downloaded-windows-artifacts/`，再调用 `acceptance:import-windows-artifacts` 校验 Windows 安装器 SHA256 与 release evidence 一致。

如果分支已推送到 GitHub，也可以由脚本直接触发构建、轮询结果、下载并导入：

```powershell
$env:GITHUB_TOKEN = "<github-token>"
$env:ASTER_GITHUB_BRANCH = "codex/aster-full-execution"
npm install
npm run acceptance:run-github-build
npm run acceptance:collect -- windows --force
```

`acceptance:run-github-build` 不依赖 GitHub CLI；它通过 GitHub REST API 调用 `workflow_dispatch`，等待 `Build Desktop Bundles` 完成，成功后复用 `acceptance:download-windows-artifacts`。

也可以使用验收包中的一键脚本：

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\docs\acceptance-package\runners\run-windows-acceptance.ps1 -Force -ArtifactsDir C:\path\to\downloaded-artifacts
```

一键脚本检测到 `GITHUB_TOKEN` 或 `GH_TOKEN` 时，也会自动下载并导入 artifacts。

GitHub Actions 命令完成后，将 `docs/release-evidence/verify-release-win32-*.json` 作为 Windows 打包证据归档。
同时归档 `docs/release-evidence/execution-coverage-win32-*.json`，作为 Windows 环境下的执行文档功能覆盖证据。
如果使用一键脚本，还会生成 `docs/acceptance-archives/aster-acceptance-package-win32-*.zip` 和对应的 `acceptance-archive-win32-*.json` SHA256 报告。
`acceptance:collect` 会预填电脑名称、系统版本、CPU 架构、应用版本、安装包路径、安装包 SHA256 和同平台自动证据报告路径，并生成逐项勾验清单。然后填写 `docs/manual-acceptance/manual-acceptance-windows-YYYY-MM-DD.json`，记录安装、启动、登录、建库、Excel、备份、恢复和主机/客户端实测结果。

打包配置要求：

- Windows 安装器使用 Tauri NSIS 配置，当前用户安装模式，不强制管理员权限。
- 安装器语言包含简体中文和英文。
- WebView2 使用在线 bootstrapper 静默安装策略；离线酒店电脑需要提前准备 WebView2 Runtime 或改用离线安装包策略后重新打包。

需要记录：

- Windows 版本
- CPU 架构
- 生成的安装包路径
- 首次启动是否成功
- 默认管理员 `admin / admin123` 是否可登录
- SQLite 数据库是否创建到系统应用数据目录
- Excel 导入预览是否可读取旧表
- 月报 Excel 是否可导出并打开
- 手动备份是否生成 zip 包
- 从 macOS 备份包恢复是否成功

## 4. macOS 实机验收

在 macOS 电脑上执行：

```bash
npm install
npm run verify:release
npm run acceptance:collect -- macos --force
```

也可以使用验收包中的一键脚本：

```bash
chmod +x docs/acceptance-package/runners/run-macos-acceptance.sh
./docs/acceptance-package/runners/run-macos-acceptance.sh --force
```

命令完成后，将 `docs/release-evidence/verify-release-darwin-*.json` 作为 macOS 打包证据归档。
同时归档 `docs/release-evidence/execution-coverage-darwin-*.json`，作为 macOS 环境下的执行文档功能覆盖证据。
如果使用一键脚本，还会生成 `docs/acceptance-archives/aster-acceptance-package-darwin-*.zip` 和对应的 `acceptance-archive-darwin-*.json` SHA256 报告。
`acceptance:collect` 会预填电脑名称、系统版本、CPU 架构、应用版本、安装包路径、安装包 SHA256 和同平台自动证据报告路径，并生成逐项勾验清单。然后填写 `docs/manual-acceptance/manual-acceptance-macos-YYYY-MM-DD.json`，记录安装、启动、登录、建库、Excel、备份、恢复和主机/客户端实测结果。

需要记录：

- macOS 版本
- CPU 架构
- `Aster.app` 是否可启动
- DMG 是否可打开和安装
- 默认管理员 `admin / admin123` 是否可登录
- SQLite 数据库是否创建到系统应用数据目录
- Excel 导入预览是否可读取旧表
- 月报 Excel 是否可导出并打开
- 手动备份是否生成 zip 包
- 从 Windows 备份包恢复是否成功

## 5. 局域网互联验收

连接参数统一要求：

- 默认端口为 `17871`，端口必须在 `1024-65535` 范围内。
- 手动填写主机地址时只填 IP 或主机名，不填写 `http://`、端口或路径。
- 配对码必须是主机界面显示的 6 位数字，客户端名称和设备 ID 不能为空。
- Windows 作为主机时，确认系统防火墙允许 Aster 或默认端口 `17871` 的局域网入站连接。
- macOS 作为主机时，确认系统允许 Aster 进行局域网访问。

### Windows 主机 + macOS 客户端

1. Windows 设置为主机模式。
2. 启动主机服务。
3. 记录主机 IP、端口和 6 位配对码。
4. macOS 设置为客户端模式。
5. 使用自动发现或手动填写主机地址。
6. 完成配对。
7. 临时停止 Windows 主机服务或断开网络，确认 macOS 客户端显示连接异常。
8. 恢复 Windows 主机服务，等待客户端自动检测并显示连接恢复。
9. macOS 客户端录入出库单。
10. Windows 主机检查库存流水和报表一致。

### macOS 主机 + Windows 客户端

1. macOS 设置为主机模式。
2. 启动主机服务。
3. 记录主机 IP、端口和 6 位配对码。
4. Windows 设置为客户端模式。
5. 使用自动发现或手动填写主机地址。
6. 完成配对。
7. 临时停止 macOS 主机服务或断开网络，确认 Windows 客户端显示连接异常。
8. 恢复 macOS 主机服务，等待客户端自动检测并显示连接恢复。
9. Windows 客户端录入入库单。
10. macOS 主机检查库存流水和报表一致。

## 6. 备份恢复验收

每组平台都需要验证：

- 当前主机创建手动备份。
- 第二备份目录可写。
- 备份 zip 包包含 `aster.sqlite`、`backup-meta.json`、`app-settings.json` 和 `import-reports/`。
- 恢复执行前系统会生成 `before_restore` 保护备份，并在手工验收 JSON 中记录保护备份路径。
- 另一平台恢复该备份包。
- 恢复后数据库健康检查通过。
- 恢复后物品、库存、流水、报表数据一致。

## 7. 验收记录表

准备给 Windows/macOS 实机验收人员交接时，可先生成验收包：

```bash
npm run acceptance:package
npm run acceptance:archive
```

输出目录为 `docs/acceptance-package/`，其中包含双端验收文档、冗灾 Runbook、手工验收说明、最新本机发布证据和交接 README。归档包输出到 `docs/acceptance-archives/aster-acceptance-package-*.zip`，并在 `docs/release-evidence/acceptance-archive-*.json` 中记录 zip 文件大小、SHA256 和交接包 manifest 摘要。

双端 JSON 填写完成后，在任意一台验收电脑上执行：

```bash
npm run verify:manual-acceptance
```

正式归档前执行严格校验：

```bash
npm run verify:manual-acceptance -- --strict
```

双端实机证据补齐后，也可以使用最终归档入口完成严格校验、readiness、交接包自检和归档包生成：

```bash
npm run acceptance:finalize
```

该命令只有在严格手工验收完整通过、readiness 状态进入 `ready-for-final-archive` 时才会输出最终通过报告；如果仍缺实机证据，它会失败并在 `docs/release-evidence/acceptance-finalize-*.json` 中记录失败命令和剩余证据。

该命令会读取 `docs/manual-acceptance/manual-acceptance-*.json`，检查 Windows、macOS、两种主机/客户端方向、跨平台恢复和双端 Excel 导入导出是否都有实机证据，并生成 `docs/release-evidence/manual-acceptance-summary-*.json`。

手工 JSON 中，验收操作人、电脑名称、系统版本、CPU 架构、应用版本、安装包路径、安装包 SHA256、导出的 Excel 路径、备份文件路径、备份 SHA256、恢复前保护备份路径、恢复使用的备份路径、主机 IP 和端口都必须填写。SHA256 必须为 64 位十六进制；填写当前项目内相对路径时，校验脚本会检查文件存在。Windows/macOS 绝对路径用于归档记录，不会在另一台电脑汇总时强制检查存在。
严格校验还会读取 `verify-release` 和 `execution-coverage` 报告，确认平台、应用版本、功能覆盖状态、功能组和安装包 SHA256 与手工记录一致。
严格校验还要求 `evidenceFiles` 留存安装/登录/建库、Excel 导入导出、备份恢复、作为主机、作为客户端的截图或附件路径；`sampleData` 留存导入样本工作簿名、导出报表月份、备份记录编号、恢复后物品数量、恢复后库存流水数量和双端业务写入单据号。

| 项目 | Windows | macOS | 备注 |
| --- | --- | --- | --- |
| 安装包生成 | GitHub Actions 生成，待实机安装验收 | 已自动校验 | Windows NSIS 配置已补齐，由 `Build Desktop Bundles` 的 Windows x64 job 生成；macOS 已生成并自动校验 app/dmg 与 bundle 元数据 |
| 首次启动 | 待验收 | 待人工打开确认 |  |
| 登录 | 待验收 | 待验收 |  |
| 建库 | 待验收 | 待验收 |  |
| Excel 导入 | 待验收 | 待验收 |  |
| Excel 导出 | 待验收 | 待验收 |  |
| 手动备份 | 待验收 | 待验收 |  |
| 跨平台恢复 | 待验收 | 待验收 |  |
| 作为主机 | 待验收 | 待验收 |  |
| 作为客户端 | 待验收 | 待验收 |  |
