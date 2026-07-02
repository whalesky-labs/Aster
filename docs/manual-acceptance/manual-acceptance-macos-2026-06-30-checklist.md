# Aster macos 实机验收采集清单

生成时间：2026-06-30T21:59:59.240Z

## 机器信息

- 电脑名称：westMacBook-Pro.local
- 系统：darwin 25.4.0
- CPU：x64
- 平台：macos
- 对端平台：windows

## 自动证据

- 手工验收 JSON：`/Volumes/NQJL/每日博士/开发项目/Aster/docs/manual-acceptance/manual-acceptance-macos-2026-06-30.json`
- verify-release：`/Volumes/NQJL/每日博士/开发项目/Aster/docs/release-evidence/verify-release-darwin-2026-06-30T21-49-16-393Z.json`
- execution-coverage：`/Volumes/NQJL/每日博士/开发项目/Aster/docs/release-evidence/execution-coverage-darwin-2026-06-30T21-59-44-921Z.json`
- 应用版本：0.1.0
- 打包产物：/Volumes/NQJL/每日博士/开发项目/Aster/src-tauri/target/release/bundle/dmg/Aster_0.1.0_x64.dmg
- 打包产物 SHA256：cd87b850813c9607c375d34f06708b0563e35ba44f37ef782062f69711674f05
- 功能覆盖状态：covered

## 必须先执行

```bash
npm install
npm run verify:release
npm run acceptance:collect -- macos --force
```

## 本机安装与启动

- [ ] 安装包已生成，并与 verify-release 报告中的路径和 SHA256 一致。
- [ ] 安装器可打开或应用已安装。
- [ ] 首次启动成功。
- [ ] 默认管理员 `admin / admin123` 可登录。
- [ ] SQLite 数据库创建在系统应用数据目录，不在项目源码目录内。
- [ ] 把上述结果填写到 `localInstall`。
- [ ] 截图或附件路径填写到 `evidenceFiles.installScreenshot`、`evidenceFiles.loginScreenshot` 和 `evidenceFiles.databaseLocationScreenshot`。

## Excel 导入导出

- [ ] 旧酒店 Excel 可进入导入预览，不直接写入正式库。
- [ ] 执行导入后生成物品、流水或导入报告。
- [ ] 月报 Excel 可导出。
- [ ] 导出的 Excel 文件可直接打开查看，不依赖本机安装 Microsoft Office 或 WPS。
- [ ] 把导出文件路径填写到 `excel.exportedWorkbookPath`。
- [ ] 把导入样本工作簿名、导出报表月份填写到 `sampleData`。
- [ ] 把导入预览截图和导出工作簿打开截图填写到 `evidenceFiles.excelImportPreviewScreenshot` 和 `evidenceFiles.exportedWorkbookScreenshot`。

## 备份与恢复

- [ ] 手动备份生成 zip 包。
- [ ] 第二备份目录可写。
- [ ] 备份包包含 `aster.sqlite`、`backup-meta.json`、`app-settings.json` 和 `import-reports/`。
- [ ] 备份记录或恢复预览中已记录来源主机名和来源系统。
- [ ] 备份文件 SHA256 已记录。
- [ ] 从 windows 备份包恢复成功。
- [ ] 跨平台恢复使用的备份来源主机名已记录。
- [ ] 恢复后健康检查通过。
- [ ] 恢复后物品、库存、流水、报表数据一致。
- [ ] 把备份记录编号、恢复后物品数量、恢复后库存流水数量填写到 `sampleData`。
- [ ] 把备份记录、恢复预览和恢复结果截图填写到 `evidenceFiles.backupRecordScreenshot`、`evidenceFiles.restorePreviewScreenshot` 和 `evidenceFiles.restoreResultScreenshot`。

## macos 作为主机，windows 作为客户端

- [ ] 切换为主机模式并启动主机服务。
- [ ] 记录主机 IP、端口和 6 位配对码。
- [ ] windows 客户端通过发现或手动地址连接。
- [ ] 配对成功。
- [ ] 临时断开主机服务或网络，客户端明确显示断线。
- [ ] 恢复主机服务后，客户端无需重启即可显示恢复连接。
- [ ] 客户端写入一笔业务单据。
- [ ] 主机侧库存流水和报表一致。
- [ ] 把结果填写到 `hostClient.asHost`，peerPlatform 填 `windows`。
- [ ] 把客户端写入单据号填写到 `sampleData.asHostBusinessDocumentNo`。
- [ ] 把主机连接状态截图填写到 `evidenceFiles.hostModeScreenshot`。

## macos 作为客户端，windows 作为主机

- [ ] windows 启动主机服务并显示配对码。
- [ ] 本机切换为客户端模式。
- [ ] 通过发现或手动地址连接主机。
- [ ] 配对成功。
- [ ] 主机临时断开时，本机明确显示断线。
- [ ] 主机恢复后，本机无需重启即可显示恢复连接。
- [ ] 本机客户端写入一笔业务单据。
- [ ] 主机侧库存流水和报表一致。
- [ ] 把结果填写到 `hostClient.asClient`，peerPlatform 填 `windows`。
- [ ] 把本机客户端写入单据号填写到 `sampleData.asClientBusinessDocumentNo`。
- [ ] 把客户端连接状态截图填写到 `evidenceFiles.clientModeScreenshot`。

## 最终校验

把 Windows 和 macOS 两端的 `docs/manual-acceptance/*.json` 与 `docs/release-evidence/*.json` 放回同一份项目目录后执行：

```bash
npm run verify:manual-acceptance -- --strict
```

严格校验通过后，才表示 Windows/macOS 双端实机验收证据完整。
