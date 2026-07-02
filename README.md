<p align="center">
  <img src="https://avatars.githubusercontent.com/u/277389313?s=200&v=4" width="128" height="128" alt="Aster">
</p>

<h1 align="center">Aster</h1>

<p align="center">
  酒店物资运营管理桌面客户端
</p>

<p align="center">
  库存闭环 · Excel 导入导出 · 本地冗灾 · 局域网多电脑协同
</p>

<p align="center">
  <a href="src-tauri/tauri.conf.json"><img src="https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white" alt="Tauri 2"></a>
  <a href="package.json"><img src="https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=111111" alt="React 19"></a>
  <a href="package.json"><img src="https://img.shields.io/badge/TypeScript-5-3178C6?logo=typescript&logoColor=white" alt="TypeScript 5"></a>
  <a href="src-tauri/Cargo.toml"><img src="https://img.shields.io/badge/Rust-stable-000000?logo=rust&logoColor=white" alt="Rust stable"></a>
  <a href="src-tauri/Cargo.toml"><img src="https://img.shields.io/badge/SQLite-local-003B57?logo=sqlite&logoColor=white" alt="SQLite local"></a>
  <a href=".github/workflows/build-desktop.yml"><img src="https://img.shields.io/badge/Windows%20%2B%20macOS-desktop-4B5563" alt="Windows and macOS"></a>
</p>

Aster 是面向酒店内部使用的 Windows 和 macOS 双端桌面客户端。系统以本地 SQLite 为核心数据源，支持单机使用、局域网主机/客户端协同、库存业务闭环、Excel 导入导出、用户权限、预算审批、盘点、报表和本地冗灾恢复。

完整产品范围见 [Aster 执行文档](docs/ASTER_EXECUTION_PLAN.md)。

## 当前能力

- 管理供应商、物资分类、物资档案、仓库、部门、用户和角色权限等基础资料。
- 支持入库、出库、调拨、调整、作废、冲销、盘点和库存流水查询。
- 支持 Excel 导入导出、报表导出、预算规则、预算审批和业务校验。
- 支持单机、主电脑、其他电脑三种运行方式；主电脑持有唯一 SQLite 数据库，其他电脑通过局域网连接主电脑。
- 支持连接向导，覆盖开启主机服务、局域网搜索、手动地址、配对码输入和连接保存。
- 支持自动备份、手动备份、第二备份目录、导入前备份、恢复前备份、恢复校验 token、失败回滚和保留策略。
- 已接入 Windows/macOS GitHub Actions 构建流程，Windows 安装包由 GitHub Windows runner 生成。

## 运行模式

| 模式 | 适用场景 | 数据位置 |
| --- | --- | --- |
| 单机模式 | 一台电脑独立使用 | 当前电脑 SQLite |
| 主电脑模式 | 局域网内作为共享主机 | 主电脑 SQLite |
| 其他电脑模式 | 连接主电脑共同使用 | 通过主电脑 API 读写 |

局域网协同时，主电脑是唯一数据权威来源。其他电脑不直接复制业务数据库，避免多端写入冲突；本地冗灾由主电脑侧备份策略保障。

## 技术架构

| 层级 | 当前实现 |
| --- | --- |
| 桌面壳 | Tauri 2 |
| 前端 | React 19 + TypeScript + Vite |
| 原生服务 | Rust + Tauri commands |
| 本地数据 | SQLite + Rust schema migration |
| 文件处理 | Excel 导入导出、验收证据和本地备份脚本 |
| 同步方式 | 局域网主机 API + 客户端配对 token |
| 目标平台 | Windows x64、macOS Intel |

## 项目结构

```text
.
├─ .github/workflows/              GitHub Actions CI 和双端桌面构建
├─ docs/                           执行文档、冗灾手册、验收模板和实测证据
├─ scripts/                        覆盖校验、发布验证、验收归档和构建辅助脚本
├─ src/                            React 前端界面和业务交互
├─ src-tauri/                      Tauri 配置、Rust 命令、SQLite 和本机能力
├─ package.json                    前端、构建和验收脚本入口
└─ tsconfig.json                   TypeScript 配置
```

## 开发命令

安装依赖：

```bash
npm install
```

启动前端开发服务：

```bash
npm run dev
```

启动 Tauri 桌面应用：

```bash
npm run tauri dev
```

构建前端：

```bash
npm run build
```

构建桌面安装包：

```bash
npm run tauri -- build
```

Rust 检查：

```bash
cd src-tauri
cargo check
```

## 验证与验收

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

验收准备度检查：

```bash
npm run verify:readiness
```

`verify:coverage` 会按执行文档检查首页、基础资料、库存闭环、盘点、报表导出、Excel 导入、备份冗灾、用户权限、主机/客户端一致性、预算审批和跨平台打包等功能组，并生成本地覆盖报告。

`verify:release` 会先运行覆盖验证，再生成发布验收证据到 `docs/release-evidence/`。报告包含平台、架构、工具版本、Tauri 打包配置摘要、产物路径、文件大小、文件 SHA256 和目录内容摘要；该目录为本地验收产物，不纳入版本库。

`verify:all-local` 会串行执行构建、手工验收脚本夹具测试、未完成标记扫描、执行覆盖、Rust 格式检查、Rust 测试、发布打包、验收交接包生成、readiness 检查、交接包自检和归档包生成。它只证明当前机器可自动验证的门禁通过，不替代 Windows/macOS 双端实机验收。

## 双端实机验收

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

手工验收记录保存在 `docs/manual-acceptance/`，用于归档 Windows/macOS 安装、启动、登录、建库、Excel、备份恢复，以及互为主机/客户端的实测证据。

`acceptance:package` 会生成 `docs/acceptance-package/`，用于把验收文档、最新本机证据和跨设备验收说明交接给实机验收人员；`acceptance:archive` 会生成 `docs/acceptance-archives/aster-acceptance-package-*.zip` 和 SHA256 归档报告。双端实机证据补齐后，`acceptance:finalize` 会先跑严格手工验收，只有 readiness 进入 `ready-for-final-archive` 时才刷新交接包并生成最终归档报告。

## 发布与 CI

当前 Tauri bundle 已配置 Windows NSIS 安装器元信息和 macOS DMG 布局。Windows 安装器由 GitHub Actions 的 `Build Desktop Bundles` workflow 在 Windows runner 上生成；macOS 可在本机或 workflow 中生成 `Aster.app` 和 DMG。

- `.github/workflows/ci.yml` 会运行前端构建、验收脚本夹具测试、未完成标记扫描、执行覆盖、Rust 格式检查和 Rust 测试。
- `.github/workflows/build-desktop.yml` 参考 Liberty 的桌面构建流程，先准备版本，再用矩阵构建 macOS Intel 和 Windows x64 安装包，并可选发布 GitHub Release。
- macOS job 在 `macos-15-intel` 上使用 `x86_64-apple-darwin` target，生成 Intel/x64 DMG。
- Windows job 在 `windows-2022` 上运行 `npm run verify:release`，生成安装器并上传 `aster-windows-x64` 和 `aster-windows-x64-release-evidence` artifacts。
- 如果本机有具备 Actions read 权限的 `GITHUB_TOKEN` 或 `GH_TOKEN`，可执行 `npm run acceptance:download-windows-artifacts` 自动下载最新成功 `Build Desktop Bundles` 的 Windows artifacts，并自动调用导入脚本校验安装器 SHA256。
- 分支已经推送到 GitHub 后，也可执行 `npm run acceptance:run-github-build` 触发 `Build Desktop Bundles` workflow、轮询结果、下载 Windows artifacts 并导入。

## 文档入口

- [Aster 执行文档](docs/ASTER_EXECUTION_PLAN.md)
- [跨平台验收说明](docs/ASTER_CROSS_PLATFORM_ACCEPTANCE.md)
- [冗灾处理手册](docs/ASTER_DISASTER_RECOVERY_RUNBOOK.md)
- [手工验收说明](docs/manual-acceptance/README.md)
- [局域网连接向导设计](docs/superpowers/specs/2026-07-02-lan-connection-wizard-design.md)
- [Windows 主机同步实测证据](docs/manual-acceptance/evidence-windows-2026-07-02/windows-host-sync-2026-07-02.md)
