# Aster 供应商筛选、库存全量导出与 Windows 托盘实施计划

日期：2026-07-15

1. 为物品列表建立关键字与供应商联合查询状态，贯通 React 页面、Tauri command、service、主机 API、远程客户端和 SQLite 游标分页。
2. 让物品档案导出复用相同供应商条件，并补分页隔离、客户端查询及筛选导出测试。
3. 新增库存导出领域模型和一次性快照查询，覆盖零库存、负库存、停用物品、分类、供应商及稳定排序。
4. 抽离库存 XLSX writer，支持本地原子文件写入与主机内存缓冲区生成，统一标题、数值格式和错误处理。
5. 新增管理员专用 Tauri 导出 command；独立/主机模式写本地文件及成功审计，客户端模式先验证本地目录，再下载主机二进制工作簿并原子落盘。
6. 扩展主机 HTTP transport 的受控二进制响应和客户端二进制请求，新增 `/api/stock/balances/export`，执行远程管理员校验并记录实际管理员与客户端的数据访问审计。
7. 在库存台账保持既有“页面功能 → 搜索 → 表格”结构，增加仅管理员可见的“导出全部库存”按钮和统一成功/失败反馈。
8. 新增 Windows-only 托盘模块，处理单击恢复、菜单显示、应用级退出及仅主窗口关闭隐藏；保持编辑子窗口和 macOS 行为不变。
9. 补齐前端交互、Rust repository/service/workbook/remote 权限及托盘判定测试；更新工程执行覆盖证据。
10. 执行格式化、TypeScript、Vitest、Playwright、Rust 测试、Clippy、工程规范、Windows target check 和最终 diff review。
