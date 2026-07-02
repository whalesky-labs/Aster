# Aster

Aster 是酒店物资运营管理桌面客户端，目标平台为 Windows + macOS。

技术栈：

- Tauri 2
- React + TypeScript
- Rust
- SQLite

完整产品范围见 [Aster 执行文档](docs/ASTER_EXECUTION_PLAN.md)。

## 全量交付状态

- Tauri 2 + React 工程已初始化。
- Rust 端已接入 SQLite，并在启动时执行 schema 迁移。
- 已建立单机、主机、客户端三种运行模式。
- 已实现库存闭环、Excel 导入导出、备份恢复、局域网主机/客户端、用户权限、预算审批、盘点和报表增强。
- macOS 本机 release 包已生成；Windows release 包由 GitHub Actions 的 `Build Desktop Bundles` workflow 在 `windows-2022` 上生成，双端互联仍需实机验收。

## 开发命令

```bash
npm install
npm run dev
npm run build
npm run tauri dev
```

打包命令：

```bash
npm run build
npm run tauri -- build
```

完整发布验证：

```bash
npm run verify:release
```

本机自动门禁总入口：

```bash
npm run verify:all-local
```

执行文档覆盖验证：

```bash
npm run verify:coverage
```

`verify:coverage` 会按执行文档检查首页、基础资料、库存闭环、盘点、报表导出、Excel 导入、备份冗灾、用户权限、主机/客户端一致性、预算审批和跨平台打包等功能组的代码、命令、测试或文档证据，并生成本地覆盖报告。

`verify:release` 会先运行 `verify:coverage`，再生成发布验收证据报告到 `docs/release-evidence/`，报告包含平台、架构、工具版本、Tauri 打包配置摘要、产物路径、文件大小、文件 SHA256 和目录内容摘要；macOS 还会校验 `Aster.app` 的 `Info.plist`、bundle identifier、版本号、最低系统版本和可执行文件。该目录为本地验收产物，不纳入版本库。

`verify:all-local` 会串行执行构建、手工验收脚本夹具测试、未完成标记扫描、执行覆盖、Rust 格式检查、Rust 测试、发布打包、验收交接包生成、readiness 检查、交接包自检和归档包生成，并生成 `docs/release-evidence/verify-all-local-*.json`。它只证明当前机器可自动验证的门禁通过，不替代 Windows/macOS 双端实机验收。

验收准备度检查：

```bash
npm run verify:readiness
```

`verify:readiness` 会检查最新自动证据和交接包是否齐全，并生成 `docs/release-evidence/readiness-*.json`。状态为 `ready-for-manual-evidence` 表示自动门禁已准备好，剩余工作是补齐双端实机证据；它不是最终验收通过。

双端实机验收模板和汇总校验：

```bash
npm run acceptance:template -- windows
npm run acceptance:template -- macos
npm run acceptance:collect -- windows --force
npm run acceptance:collect -- macos --force
npm run acceptance:status
npm run acceptance:status -- --summary
npm run acceptance:package
npm run acceptance:archive
npm run acceptance:finalize
npm run verify:acceptance-package
npm run verify:manual-acceptance
npm run verify:manual-acceptance -- --strict
```

手工验收记录保存在 `docs/manual-acceptance/`，用于归档 Windows/macOS 安装、启动、登录、建库、Excel、备份恢复，以及互为主机/客户端的实测证据。`acceptance:collect` 会生成 JSON 模板和逐项勾验清单；`verify:manual-acceptance -- --strict` 会反查 release/coverage 报告的平台、版本、功能覆盖状态和安装包 SHA256。`acceptance:status` 会汇总最新自动证据、readiness、剩余实机证据组数、归档包 SHA256 和下一步命令；现场快速查看可用 `acceptance:status -- --summary`。`acceptance:package` 会生成 `docs/acceptance-package/`，用于把验收文档、最新本机证据和跨设备验收说明交接给实机验收人员；`acceptance:archive` 会生成 `docs/acceptance-archives/aster-acceptance-package-*.zip` 和 SHA256 归档报告；`verify:acceptance-package` 会检查交接包中的关键文档、运行脚本、证据报告和文件完整性清单是否齐全。双端实机证据补齐后，执行 `acceptance:finalize` 会先跑严格手工验收，只有 readiness 进入 `ready-for-final-archive` 时才刷新交接包并生成最终归档报告。

当前 Tauri bundle 已配置 Windows NSIS 安装器元信息和 macOS DMG 布局。Windows 安装器由 GitHub Actions 的 `Build Desktop Bundles` workflow 在 Windows runner 上生成；macOS 可在本机或 workflow 中生成 `Aster.app` 和 DMG。

GitHub Actions：

- `.github/workflows/ci.yml` 会运行前端构建、验收脚本夹具测试、未完成标记扫描、执行覆盖、Rust 格式检查和 Rust 测试。
- `.github/workflows/build-desktop.yml` 参考 Liberty 的桌面构建流程，先准备版本，再用矩阵构建 macOS Intel 和 Windows x64 安装包，并可选发布 GitHub Release。
- macOS job 在 `macos-15-intel` 上使用 `x86_64-apple-darwin` target，生成 Intel/x64 DMG。
- Windows job 在 `windows-2022` 上运行 `npm run verify:release`，生成安装器并上传 `aster-windows-x64` 和 `aster-windows-x64-release-evidence` artifacts。
- Windows 实机验收时，从 GitHub Actions 下载 Windows 安装器和 release evidence 后，再执行 `npm run acceptance:collect -- windows --force` 填写现场记录。
- 如果本机有具备 Actions read 权限的 `GITHUB_TOKEN` 或 `GH_TOKEN`，可执行 `npm run acceptance:download-windows-artifacts` 自动下载最新成功 `Build Desktop Bundles` 的 Windows artifacts，并自动调用导入脚本校验安装器 SHA256。
- 分支已经推送到 GitHub 后，也可执行 `npm run acceptance:run-github-build` 触发 `Build Desktop Bundles` workflow、轮询结果、下载 Windows artifacts 并导入。

Rust 检查：

```bash
cd src-tauri
cargo check
```
