# 更新日志

本文档记录 Aster 桌面客户端的版本更新内容。GitHub Release 会在构建时自动读取对应版本小节，并追加安装包下载说明。

## [0.1.0] - 2026-07-02

### 新增

- 完成 Aster Windows 与 macOS 桌面客户端全量功能交付，技术栈为 Tauri 2、React、Rust 和 SQLite。
- 支持本地 SQLite 数据库、库存入库/出库/调拨/调整/作废/冲销闭环、盘点、报表、Excel 导入导出、用户角色权限、预算规则和审批。
- 支持局域网单主机多客户端同步模式，主机持有唯一 SQLite 数据库，客户端通过主机 API 访问，不共享数据库文件。
- 支持本地冗灾能力，包括自动备份、手动备份、第二备份目录、导入前备份、恢复前备份、恢复校验 token、失败回滚和保留策略。
- 新增 Windows/macOS 双端 GitHub Actions 构建流程，Windows 安装包由 GitHub Windows runner 构建，macOS 安装包由 GitHub macOS runner 构建。

### 验收说明

- 自动构建、发布校验、覆盖扫描和验收交接脚本已接入。
- 最终业务验收仍需补齐 Windows/macOS 真实安装、首次启动、登录建库、Excel 导入导出、备份恢复和双机互联证据。
