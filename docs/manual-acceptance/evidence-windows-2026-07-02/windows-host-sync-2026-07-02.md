# Windows 主机模式配对与同步验收记录

验收时间：2026-07-02 16:15 CST

## 环境

- Windows 主机：`192.168.5.22`
- 主机名：`User-2026QWVKQA`
- 系统：`Microsoft Windows NT 10.0.26200.0`
- Aster 版本：`0.1.0`
- 安装路径：`C:\Users\Administrator\AppData\Local\Aster`
- 主机服务：`0.0.0.0:17871`
- 数据目录：`C:\Users\Administrator\AppData\Roaming\Aster\Aster\data`
- 数据库：`C:\Users\Administrator\AppData\Roaming\Aster\Aster\data\aster.sqlite`
- macOS 客户端 IP：`192.168.10.17`

## 主机服务验证

- Windows 本机 `curl.exe http://127.0.0.1:17871/api/health` 返回 200。
- macOS 访问 `http://192.168.5.22:17871/api/health` 返回 200。
- `netstat` 显示 `aster.exe` 监听：
  - `TCP 0.0.0.0:17871 LISTENING`
  - `UDP 0.0.0.0:17871`
- 健康检查返回：

```json
{"appName":"Aster","appVersion":"0.1.0","schemaVersion":1,"databaseOk":true,"message":"主机数据库健康"}
```

## 配对与重启持久性

- 初始配对码：`339858`
- macOS 侧通过原始 TCP HTTP 请求调用 `/api/pair` 配对成功。
- 返回 token：`04139616-e01d-4372-b0c2-b9fa563719ec`
- 使用 `admin / admin123` 通过主机 API 登录成功，用户 ID：`user-admin`
- Windows `aster.exe` 重启后，旧 token 继续访问 `/api/status` 成功，状态码 200。
- 重启后客户端列表仍显示 macOS 验收客户端在线：
  - clientName：`macOS验收客户端-1782978876734`
  - clientDeviceId：`macos-acceptance-1782978876734`
  - appVersion：`0.1.0`
  - status：`online`

## 同步写入数据

macOS 客户端通过 Windows 主机 API 写入一笔入库业务，主机侧 API 和 SQLite 离线查询均确认数据存在。

- 数据前缀：`E2E-20260702075534`
- 分类 ID：`39657636-903a-4ca7-a038-c40fea56f719`
- 单位 ID：`c84439b9-f1d7-4bde-afe8-5ccca58da4f7`
- 供应商 ID：`ef1b39e4-ec18-406b-8b4d-f31df0750406`
- 物品 ID：`a160a248-b4c9-472b-ac5b-237d0d353b34`
- 物品编码：`E2E-20260702075534-ITEM`
- 单据 ID：`189e79be-7ec8-41a4-86a9-5af0fda13ecd`
- 单据号：`IN-20260702-0001`
- 操作人：`macOS客户端验收`

主机 API `/api/status` 摘要：

- itemCount：`1`
- departmentCount：`8`
- supplierCount：`1`
- currentStockAmount：`37.5`
- thisMonthInboundAmount：`37.5`
- health：`数据库健康检查通过，库存余额与流水一致`

主机 API `/api/stock/balances?itemId=...` 返回：

- quantity：`3`
- amount：`37.5`
- averagePrice：`12.5`
- lastInboundPrice：`12.5`
- stockStatus：`normal`

主机 API `/api/stock/movements?itemId=...` 返回：

- direction：`in`
- quantity：`3`
- unitPrice：`12.5`
- amount：`37.5`
- documentNo：`IN-20260702-0001`
- movementType：`inbound`
- operator：`macOS客户端验收`
- remark：`client-submit`

## SQLite 离线证据

复制 Windows 数据库、WAL 和 SHM 到本机后执行 `PRAGMA integrity_check`，结果为 `ok`。本地读取触发 checkpoint 后，样本计数如下：

- master_items：`1`
- stock_documents：`1`
- stock_movements：`1`
- stock_balances：`1`
- client_connections：`1`

SQLite 查询确认：

- `master_items` 存在 `E2E-20260702075534-ITEM / E2E-20260702075534-物品`
- `stock_documents` 存在 `IN-20260702-0001`，状态 `confirmed`
- `stock_balances` 对应物品余额 `3 / 37.5 / 12.5`
- `stock_movements` 对应入库流水 `in / inbound / 3 / 37.5`
- `client_connections` 存在 macOS 客户端 `macos-acceptance-1782978876734`

本地证据文件 SHA256：

```text
924d6d655e33b29156a706cb23df73f0bc9535c7c74e5417cdbe2b5c86846267  aster.sqlite
e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  aster.sqlite-wal
2027b6aa0267b3af5c3e14990ebaf451669fa1a0ce1de57b8e8a58831e3cf5ce  aster.sqlite-shm
```

## 备份与冗灾证据

Windows 主机数据目录下存在自动备份：

- `aster-backup-20260702-153723-1e1caeec.zip`
- `aster-backup-20260702-155223-c9e84fc4.zip`

最近备份包 `aster-backup-20260702-155223-c9e84fc4.zip` 包含：

- `backup-meta.json`
- `app-settings.json`
- `aster.sqlite`
- `import-reports/`

备份记录表显示：

- `auto_startup` 备份成功，来源主机 `User-2026QWVKQA`，系统 `windows`
- `auto_interval` 备份成功，来源主机 `User-2026QWVKQA`，系统 `windows`
- 备份内数据库 SHA256 与 `backup_jobs.sha256` 一致：
  - `dcf509f25cd01e817a8be8299c08eca2eefbb840b4ded0c38201dce68b631d77`

备份文件 SHA256：

```text
b78ce138467b0b422104d61435ae8aaf1cb86fbc2da108789c11400dbaa5f16b  aster-backup-20260702-155223-c9e84fc4.zip
dcf509f25cd01e817a8be8299c08eca2eefbb840b4ded0c38201dce68b631d77  backup-unzip/aster.sqlite
```

注意：最新同步入库发生在 `2026-07-02 15:55:35`，晚于最近自动备份 `2026-07-02 15:52:23`。因此本轮证据证明备份机制和备份包结构正常，但该备份包不包含这笔最新入库数据。需要在 Windows 主机 UI 手动创建一次备份，或等待下一次间隔备份后，再补充“最新同步数据已进入备份包”的证据。

## 当前结论

- Windows 作为主机、macOS 作为客户端：配对通过。
- macOS 客户端写入 Windows 主机：通过。
- Windows 主机重启后旧 token 继续可用：通过。
- Windows 主机数据库健康与库存余额一致性：通过。
- 自动备份存在、结构完整、来源主机和 SHA256 可追溯：通过。
- 第二备份目录状态：主机 API 当前显示 `secondBackupOk=false`，仍需在 UI 中配置或确认第二备份目录。
- 尚未覆盖：Windows 客户端连接 macOS 主机、断线/重连 UI 截图、Windows UI 手动备份后最新数据入包、双端正式手工验收 JSON 勾选。
