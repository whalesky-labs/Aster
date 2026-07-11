import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { arch, platform, release } from "node:os";

const root = process.cwd();

const sources = {
  executionPlan: "docs/ASTER_EXECUTION_PLAN.md",
  crossPlatformAcceptance: "docs/ASTER_CROSS_PLATFORM_ACCEPTANCE.md",
  disasterRecoveryRunbook: "docs/ASTER_DISASTER_RECOVERY_RUNBOOK.md",
  readme: "README.md",
  githubCi: ".github/workflows/ci.yml",
  githubDesktopBuild: ".github/workflows/build-desktop.yml",
  packageJson: "package.json",
  verifyRelease: "scripts/verify-release.mjs",
  verifyAllLocal: "scripts/verify-all-local.mjs",
  verifyReadiness: "scripts/verify-readiness.mjs",
  acceptanceStatus: "scripts/acceptance-status.mjs",
  verifyNoPlaceholders: "scripts/verify-no-placeholders.mjs",
  app: [
    "src/App.tsx",
    "src/features/dashboard/Dashboard.tsx",
    "src/features/backups/BackupRecordsPage.tsx",
    "src/features/reports/ReportsPage.tsx",
    "src/features/reports/ReportComponents.tsx",
    "src/features/settings/SettingsPage.tsx",
    "src/features/stock/StockBalancePage.tsx",
    "src/features/stock/StockMovementPage.tsx",
  ],
  lib: "src-tauri/src/lib.rs",
  tauriConfig: "src-tauri/tauri.conf.json",
  cargoToml: "src-tauri/Cargo.toml",
  migrations: "src-tauri/migrations/001_initial_schema.sql",
  stockRepository: [
    "src-tauri/src/db/stock_repository.rs",
    "src-tauri/src/db/stock_repository/queries.rs",
    "src-tauri/src/db/stock_repository/persistence.rs",
    "src-tauri/src/db/stock_repository/movements.rs",
    "src-tauri/src/db/stock_repository/fifo.rs",
    "src-tauri/src/db/stock_repository/validation.rs",
    "src-tauri/src/db/stock_repository/helpers.rs",
    "src-tauri/src/db/stock_repository/tests/batches.rs",
    "src-tauri/src/db/stock_repository/tests/documents.rs",
    "src-tauri/src/db/stock_repository/tests/queries.rs",
    "src-tauri/src/db/stock_repository/tests/rules.rs",
  ],
  stocktakeRepository: "src-tauri/src/db/stocktake_repository.rs",
  reportRepository: "src-tauri/src/db/report_repository.rs",
  reportsDomain: "src-tauri/src/domain/reports.rs",
  backupRepository: "src-tauri/src/db/backup_repository.rs",
  repository: "src-tauri/src/db/repository.rs",
  masterDataRepository: "src-tauri/src/db/master_data_repository.rs",
  userRepository: "src-tauri/src/db/user_repository.rs",
  approvalRepository: "src-tauri/src/db/approval_repository.rs",
  backupService: [
    "src-tauri/src/services/backup_service.rs",
    "src-tauri/src/services/backup_service/archive.rs",
    "src-tauri/src/services/backup_service/automation.rs",
    "src-tauri/src/services/backup_service/restore.rs",
    "src-tauri/src/services/backup_service/tests/archive.rs",
    "src-tauri/src/services/backup_service/tests/automation.rs",
  ],
  approvalService: "src-tauri/src/services/approval_service.rs",
  hostService: [
    "src-tauri/src/services/host_service.rs",
    "src-tauri/src/services/host_service/remote_master_data.rs",
    "src-tauri/src/services/host_service/remote_operations.rs",
    "src-tauri/src/services/host_service/remote_stock.rs",
    "src-tauri/src/services/host_service/tests/connections.rs",
    "src-tauri/src/services/host_service/tests/permissions.rs",
    "src-tauri/src/services/host_service/tests/routes.rs",
  ],
  importService: [
    "src-tauri/src/services/import_service.rs",
    "src-tauri/src/services/import_service/execution.rs",
    "src-tauri/src/services/import_service/master_data.rs",
    "src-tauri/src/services/import_service/models.rs",
    "src-tauri/src/services/import_service/parser.rs",
    "src-tauri/src/services/import_service/workbook.rs",
    "src-tauri/src/services/import_service/tests/execution.rs",
    "src-tauri/src/services/import_service/tests/workbook.rs",
  ],
  reportService: "src-tauri/src/services/report_service.rs",
  stockService: "src-tauri/src/services/stock_service.rs",
  stocktakeService: "src-tauri/src/services/stocktake_service.rs",
  statusService: "src-tauri/src/services/status_service.rs",
  userService: "src-tauri/src/services/user_service.rs",
  migrationsService: "src-tauri/src/db/migrations.rs",
  masterDataCommands: "src-tauri/src/commands/master_data_commands.rs",
  stockCommands: "src-tauri/src/commands/stock_commands.rs",
  stocktakeCommands: "src-tauri/src/commands/stocktake_commands.rs",
  reportCommands: "src-tauri/src/commands/report_commands.rs",
  importCommands: "src-tauri/src/commands/import_commands.rs",
  backupCommands: "src-tauri/src/commands/backup_commands.rs",
  hostCommands: "src-tauri/src/commands/host_commands.rs",
  userCommands: "src-tauri/src/commands/user_commands.rs",
  approvalCommands: "src-tauri/src/commands/approval_commands.rs",
  verifyManualAcceptance: "scripts/verify-manual-acceptance.mjs",
  testManualAcceptance: "scripts/test-manual-acceptance-paths.mjs",
  collectAcceptanceEvidence: "scripts/collect-acceptance-evidence.mjs",
  createAcceptanceRunners: "scripts/create-acceptance-runners.mjs",
  createAcceptancePackage: "scripts/create-acceptance-package.mjs",
  importWindowsArtifacts: "scripts/import-windows-artifacts.mjs",
  downloadWindowsArtifacts: "scripts/download-github-windows-artifacts.mjs",
  runGithubDesktopBuild: "scripts/run-github-desktop-build.mjs",
  archiveAcceptancePackage: "scripts/archive-acceptance-package.mjs",
  finalizeAcceptance: "scripts/finalize-acceptance.mjs",
  verifyAcceptancePackage: "scripts/verify-acceptance-package.mjs",
  manualAcceptanceReadme: "docs/manual-acceptance/README.md",
};

const cache = new Map();
const failures = [];

function text(key) {
  const sourcePaths = sources[key];
  if (!sourcePaths) {
    throw new Error(`Unknown source key: ${key}`);
  }
  if (!cache.has(key)) {
    const relativePaths = Array.isArray(sourcePaths) ? sourcePaths : [sourcePaths];
    const contents = [];
    for (const relativePath of relativePaths) {
      const path = join(root, relativePath);
      if (!existsSync(path)) {
        failures.push({
          feature: "source",
          check: `source file exists: ${relativePath}`,
          source: relativePath,
        });
      } else {
        contents.push(readFileSync(path, "utf8"));
      }
    }
    cache.set(key, contents.join("\n"));
  }
  return cache.get(key);
}

function has(key, pattern) {
  const value = text(key);
  if (pattern instanceof RegExp) {
    return pattern.test(value);
  }
  return value.includes(pattern);
}

function check(feature, checkName, key, pattern) {
  const ok = has(key, pattern);
  const source = Array.isArray(sources[key]) ? sources[key].join(", ") : sources[key];
  return {
    feature,
    check: checkName,
    source,
    ok,
    pattern: pattern instanceof RegExp ? pattern.source : pattern,
  };
}

function extractRegisteredTauriCommands() {
  const match = text("lib").match(/generate_handler!\s*\[([\s\S]*?)\]\)/);
  if (!match) return [];
  return [...match[1].matchAll(/\b([a-z][a-z0-9_]+)\b/g)]
    .map((item) => item[1])
    .filter((command) => !["tauri", "generate_handler"].includes(command));
}

function extractInvokedTauriCommands() {
  return [...text("app").matchAll(/invoke(?:<[^>]+>)?\(\s*["']([^"']+)["']/g)].map((item) => item[1]);
}

function tauriCommandConsistencyReport() {
  const registered = [...new Set(extractRegisteredTauriCommands())].sort();
  const invoked = [...new Set(extractInvokedTauriCommands())].sort();
  const registeredSet = new Set(registered);
  const invokedWithoutRegistration = invoked.filter((command) => !registeredSet.has(command));
  return {
    id: "tauri_command_consistency",
    name: "前端 Tauri 命令调用一致性",
    status: invokedWithoutRegistration.length === 0 ? "covered" : "missing-evidence",
    registered,
    invoked,
    invokedWithoutRegistration,
    registeredOnly: registered.filter((command) => !invoked.includes(command)),
  };
}

const requirements = [
  {
    id: "workspace_dashboard",
    name: "首页工作台",
    checks: [
      ["执行文档包含首页工作台验收要求", "executionPlan", "### 3.1 首页工作台"],
      ["前端包含首页快捷入口和统计入口", "app", "function Dashboard"],
      ["首页状态来自 Rust status 服务", "statusService", "build_app_status_uses_database_metrics_and_recent_operations"],
      ["首页状态按部门查看员范围收窄", "statusService", "build_app_status_scopes_department_viewer_recent_operations_and_outbound_amount"],
      ["远程首页状态按部门查看员范围收窄", "hostService", "remote_status_forces_department_viewer_scope"],
      ["最近操作来自数据库查询", "repository", "recent_operations_returns_stock_movements_with_business_context"],
      ["Tauri 注册 get_app_status", "lib", "get_app_status"],
    ],
  },
  {
    id: "master_data",
    name: "基础资料：物品、分类、单位、部门、供应商",
    checks: [
      ["执行文档包含基础资料范围", "executionPlan", "### 3.2 物品档案"],
      ["前端包含物品条码和基础资料入口", "app", "barcode"],
      ["Tauri 注册基础资料命令", "lib", "list_supplier_purchase_records"],
      ["命令层包含物品/分类/单位/部门/供应商", "masterDataCommands", "pub fn save_supplier"],
      ["数据库层覆盖分类类型约束", "masterDataRepository", "save_category_supports_large_and_small_categories_only"],
      ["物品档案拒绝停用分类单位供应商", "masterDataRepository", "save_item_requires_enabled_category_unit_and_supplier_references"],
      ["物品列表支持 1000+ 有测试覆盖", "masterDataRepository", "list_items_supports_more_than_one_thousand_items"],
      ["供应商采购记录有查询测试", "masterDataRepository", "list_supplier_purchase_records_filters_inbound_movements_by_supplier"],
      ["迁移包含物品唯一编码", "migrations", "code TEXT NOT NULL UNIQUE"],
    ],
  },
  {
    id: "stock_lifecycle",
    name: "库存闭环：入库、出库、草稿、确认、作废冲正、调整",
    checks: [
      ["执行文档包含入库和出库要求", "executionPlan", "### 3.6 入库管理"],
      ["前端包含入库/出库单据界面", "app", "StockDocumentPage"],
      ["前端入库/出库列表暴露月份对象物品筛选", "app", "document-filters"],
      ["前端筛选通过 query 调用库存单据接口", "app", "onQueryChange"],
      ["前端单据行支持手工修正金额", "app", "effectiveLineAmount(line)"],
      ["Tauri 注册库存命令", "lib", "confirm_stock_document_draft"],
      ["命令层包含提交/草稿/确认/作废/调整", "stockCommands", "pub fn void_stock_document"],
      ["命令层兼容 query 和旧 documentType 参数", "stockCommands", "query: Option<StockDocumentQuery>"],
      ["入库出库支持月份/对象/物品筛选有测试覆盖", "stockRepository", "list_stock_documents_filters_by_month_party_item_and_search"],
      ["手工修正金额进入流水和余额有测试覆盖", "stockRepository", "submit_stock_document_uses_manual_line_amount_when_provided"],
      ["历史部门和供应商名称快照有测试覆盖", "stockRepository", "stock_documents_and_movements_keep_party_name_snapshots_after_rename"],
      ["报表明细使用历史名称快照有测试覆盖", "reportRepository", "report_details_use_movement_party_name_snapshots_after_rename"],
      ["停用部门和供应商不能用于新确认单据", "stockRepository", "submit_stock_document_rejects_disabled_department_and_supplier"],
      ["确认草稿会重新校验持久化业务规则", "stockRepository", "confirm_draft_revalidates_persisted_business_rules"],
      ["库存台账和流水筛选前端已暴露", "app", "StockBalanceQuery"],
      ["库存台账可跳转查看单物品流水", "app", "onViewMovements"],
      ["库存流水展示类型操作人和备注", "app", "movementTypeLabel(row.movementType)"],
      ["库存事务测试覆盖确认后更新余额", "stockRepository", "save_and_confirm_draft_updates_inventory_only_on_confirm"],
      ["库存台账和流水支持 1000+ 有测试覆盖", "stockRepository", "stock_balance_and_movement_lists_support_more_than_one_thousand_rows"],
      ["库存台账和流水结构化筛选有测试覆盖", "stockRepository", "stock_balance_and_movement_lists_support_structured_filters"],
      ["作废和调整写入库存流水", "stockRepository", "submit_adjustment_and_void_document_write_inventory_movements"],
      ["调整类型和方向语义有测试覆盖", "stockService", "validate_adjustment_enforces_type_direction_semantics"],
      ["负库存策略有测试覆盖", "stockRepository", "allow_negative_stock_setting_allows_outbound_below_zero"],
      ["超预算阻断有测试覆盖", "stockRepository", "submit_outbound_rejects_when_budget_limit_would_be_exceeded"],
    ],
  },
  {
    id: "stocktake",
    name: "盘点管理",
    checks: [
      ["执行文档包含盘点管理", "executionPlan", "### 3.10 盘点管理"],
      ["前端包含盘点页面", "app", "StocktakePage"],
      ["Tauri 注册盘点命令", "lib", "export_stocktake_sheet"],
      ["命令层包含创建/录入/确认/导出", "stocktakeCommands", "pub fn confirm_stocktake"],
      ["数据库测试覆盖盘盈盘亏流水和余额", "stocktakeRepository", "confirm_stocktake_writes_gain_loss_movements_and_updates_balance"],
      ["盘点范围拒绝停用分类和物品", "stocktakeRepository", "create_stocktake_rejects_disabled_category_and_items"],
      ["盘点数量录入拒绝不存在或跨单明细", "stocktakeRepository", "update_stocktake_counts_rejects_unknown_or_foreign_line"],
      ["已确认盘点支持作废冲正", "stockRepository", "void_confirmed_stocktake_writes_reversal_and_marks_stocktake_voided"],
      ["前端盘点页包含作废盘点入口", "app", "作废盘点"],
      ["服务层支持导出盘点表", "stocktakeCommands", "export_stocktake_sheet"],
      ["盘点表导出使用默认导出目录", "stocktakeCommands", "export_stocktake_sheet"],
      ["盘点表默认导出目录有测试覆盖", "stocktakeService", "export_stocktake_sheet_uses_default_export_dir_setting"],
    ],
  },
  {
    id: "reports_export_print",
    name: "报表中心、Excel 导出和打印",
    checks: [
      ["执行文档包含报表中心", "executionPlan", "### 3.12 报表中心"],
      ["前端报表页包含打印", "app", "window.print"],
      ["前端报表页暴露部门分类物品供应商筛选", "app", "report-filters"],
      ["前端报表页使用月份选择面板", "app", "MonthSelect"],
      ["前端报表页包含指标卡分组", "app", "report-metrics-grid"],
      ["前端报表页包含报表分组标题", "app", "ReportGroupHeader"],
      ["报表查询领域包含日期范围", "reportsDomain", "start_date"],
      ["Tauri 注册报表查询和导出", "lib", "export_monthly_report"],
      ["导出工作簿包含月度进销存", "reportService", "月度进销存"],
      ["导出工作簿包含部门领用明细", "reportService", "部门领用明细"],
      ["导出工作簿包含库存余额", "reportService", "库存余额"],
      ["导出工作簿包含盘点差异", "reportService", "盘点差异"],
      ["报表领域包含盘点差异集合", "reportsDomain", "stocktake_differences"],
      ["XLSX 可打开性有测试覆盖", "reportService", "write_report_workbook_creates_openable_xlsx_package"],
      ["月报导出使用默认导出目录有测试覆盖", "reportService", "export_monthly_report_uses_default_export_dir_setting"],
      ["部门范围报表过滤有测试覆盖", "reportRepository", "get_report_bundle_filters_department_summary_and_details_by_department_scope"],
      ["分类物品供应商报表过滤有测试覆盖", "reportRepository", "get_report_bundle_filters_category_item_and_supplier"],
      ["日期范围报表过滤有测试覆盖", "reportRepository", "get_report_bundle_filters_movement_reports_by_date_range"],
      ["物品消耗排行导出完整列表有测试覆盖", "reportRepository", "item_consumption_ranking_exports_all_consumed_items"],
      ["盘点差异报表过滤有测试覆盖", "reportRepository", "stocktake_difference_report_filters_month_category_and_item"],
      ["前端用卡片展示盘点差异摘要", "app", "盘点差异"],
      ["前端包含图表报表视图", "app", "BarChartPanel"],
      ["前端打印调用浏览器打印", "app", "window.print"],
      ["远程报表 API 透传开始日期", "hostService", "startDate"],
      ["远程报表 API 透传结束日期", "hostService", "endDate"],
    ],
  },
  {
    id: "excel_import",
    name: "Excel 导入和迁移报告",
    checks: [
      ["执行文档包含 Excel 导入", "executionPlan", "### 3.13 Excel 导入"],
      ["前端导入页有预览和执行入口", "app", "ImportPage"],
      ["Tauri 注册导入预览和执行", "lib", "preview_excel_import"],
      ["导入服务使用 calamine 读取 XLSX", "importService", "calamine"],
      ["Tauri 注册导入模板导出", "lib", "export_import_template"],
      ["新版三表模板导出有测试覆盖", "importService", "export_import_template_creates_three_sheet_workbook"],
      ["新版三表模板预览有测试覆盖", "importService", "preview_import_template_reads_item_inbound_outbound_sheets"],
      ["旧月报模板拒绝有测试覆盖", "importService", "preview_import_template_rejects_old_monthly_workbook"],
      ["单位单价数量金额和业务时间校验有测试覆盖", "importService", "preview_template_workbook_reports_validation_messages"],
      ["公式错误校验有测试覆盖", "importService", "detect_formula_errors_reports_cell_errors"],
      ["导入生成采购批次和客销成本有测试覆盖", "importService", "import_template_creates_batches_and_guest_sale_costs"],
      ["导入复用已有供应商部门 ID 有测试覆盖", "importService", "import_template_uses_existing_supplier_and_department_ids"],
      ["只导入物品档案有测试覆盖", "importService", "import_items_only_creates_items_without_documents_or_movements"],
      ["迁移报告有测试覆盖", "importService", "write_import_report_creates_json_migration_report"],
      ["客户端模式导入被拒绝", "importService", "preview_excel_import_rejects_client_mode_before_parsing_workbook"],
      ["正式导入复用预览错误门禁且不写正式库", "importService", "run_excel_import_rejects_preview_errors_without_backup_or_writes"],
      ["正式导入前创建 before_import 保护备份", "importService", "run_excel_import_creates_before_import_backup_before_writes"],
    ],
  },
  {
    id: "backup_disaster_recovery",
    name: "数据安全、备份恢复和冗灾",
    checks: [
      ["执行文档包含冗灾目标", "executionPlan", "### 5.7 冗灾目标"],
      ["冗灾 Runbook 已落地", "disasterRecoveryRunbook", "主机损坏后的切换流程"],
      ["冗灾 Runbook 要求核对来源主机", "disasterRecoveryRunbook", "来源主机"],
      ["前端包含备份恢复和第二备份配置", "app", "第二备份"],
      ["前端备份记录展示版本和 SHA256 追踪信息", "app", "backup.sha256"],
      ["前端备份记录展示来源主机和系统", "app", "backup.hostName"],
      ["前端手动备份结果展示来源主机", "app", "lastBackup.sourceHostName"],
      ["Tauri 注册备份恢复命令", "lib", "restore_backup"],
      ["备份包结构包含必需条目", "backupService", "backup_zip_contains_required_archive_entries"],
      ["备份记录包含来源主机和系统", "backupService", "created_backup_record_tracks_source_host_and_os"],
      ["备份创建使用默认备份目录有测试覆盖", "backupService", "create_backup_uses_default_backup_dir_setting"],
      ["快速连续备份文件名唯一且记录可追踪", "backupService", "rapid_backups_use_unique_file_names_and_records"],
      ["恢复预览校验 Schema、来源系统和 SHA256", "backupService", "preview_restore_backup_validates_schema_source_and_database_sha"],
      ["恢复预览拒绝损坏 SHA256", "backupService", "preview_restore_backup_rejects_tampered_database_sha"],
      ["恢复必须绑定预览校验令牌", "backupService", "restore_backup_requires_matching_preview_validation_token"],
      ["第二备份目录复制有测试覆盖", "backupService", "create_backup_copies_to_second_backup_dir"],
      ["恢复失败自动回滚有测试覆盖", "backupService", "restore_backup_rolls_back_when_restored_database_fails_integrity_check"],
      ["恢复后重置本机路径有测试覆盖", "backupService", "restore_backup_resets_machine_local_paths"],
      ["自动备份保留策略有测试覆盖", "backupService", "auto_backup_retention_keeps_recent_daily_and_monthly_records"],
      ["启动自动备份有测试覆盖", "backupService", "startup_backup_runs_once_per_day_when_enabled"],
      ["运行中定时备份有测试覆盖", "backupService", "interval_backup_runs_when_due_and_respects_settings"],
      ["SQLite 备份时间按 UTC 解析有测试覆盖", "backupService", "parse_backup_timestamp_treats_sqlite_timestamp_as_utc"],
      ["危险操作限制客户端模式", "backupService", "create_backup_rejects_client_mode_even_for_admin"],
      ["备份恢复危险操作限制客户端模式", "backupService", "backup_dangerous_operations_reject_client_mode"],
      ["应用启动时触发启动自动备份", "lib", "run_startup_backup_if_needed"],
      ["运行中定时备份 worker 已启动", "lib", "start_interval_backup_worker"],
      ["系统状态暴露最近定时备份和第二备份健康", "statusService", "latest_interval_backup_at"],
      ["健康检查核对库存余额与流水一致性", "statusService", "build_app_status_marks_stock_balance_mismatch_unhealthy"],
      ["手工验收要求记录备份来源主机", "verifyManualAcceptance", "backup.sourceHostName"],
      ["手工验收要求 Excel 正式导入生成记录", "verifyManualAcceptance", "excel.importRunCreatedRecords"],
      ["手工验收要求记录恢复前保护备份", "verifyManualAcceptance", "restore.beforeRestoreBackupCreated"],
      ["手工验收要求记录恢复前保护备份路径", "verifyManualAcceptance", "restore.beforeRestoreBackupFile"],
    ],
  },
  {
    id: "users_permissions",
    name: "用户、角色、权限和审计日志",
    checks: [
      ["执行文档包含用户与权限", "executionPlan", "### 3.17 用户与权限"],
      ["前端包含用户管理页", "app", "UsersPage"],
      ["Tauri 注册用户命令", "lib", "change_password"],
      ["命令层包含登录/改密码/用户启停", "userCommands", "pub fn set_user_account_enabled"],
      ["角色种子包含管理员/仓库员/部门查看员/只读", "migrations", "department_viewer"],
      ["用户变更写审计日志有测试覆盖", "userService", "save_and_disable_user_on_conn_write_audit_logs"],
      ["客户端未配对只允许本机管理员配对设置", "userService", "client_mode_without_pairing_token_allows_local_admin_for_pairing_setup"],
      ["审计日志查询有测试覆盖", "repository", "list_audit_logs_returns_latest_rows_with_limit"],
      ["系统设置保存写审计日志有测试覆盖", "statusService", "save_system_settings_persists_values_and_writes_audit_log"],
      ["客户端模式系统设置仅保存本机目录", "statusService", "save_system_settings_in_client_mode_only_persists_local_directories"],
      ["前端按权限隐藏模块", "app", "canAccessNav"],
    ],
  },
  {
    id: "host_client_consistency",
    name: "局域网主机/客户端一致性",
    checks: [
      ["执行文档包含一致性原则", "executionPlan", "### 5.3 一致性原则"],
      ["前端包含主机服务、发现、配对", "app", "discover_hosts"],
      ["Tauri 注册主机连接命令", "lib", "pair_with_host"],
      ["主机服务包含 TCP 和 UDP 发现", "hostService", "serve_discovery"],
      ["客户端连接参数校验有测试覆盖", "hostService", "host_connection_validation_rejects_common_operator_input_errors"],
      ["配对码和客户端身份校验有测试覆盖", "hostService", "pairing_validation_requires_twelve_digit_code_and_client_identity"],
      ["客户端设备 ID 本地稳定持久化", "statusService", "get_runtime_config_generates_stable_client_device_id"],
      ["主机客户端连接记录落库", "hostService", "client_connections_are_persisted_and_touch_updates_status"],
      ["设置页客户端列表读取持久记录", "hostService", "list_client_connections_reads_persisted_host_records"],
      ["主机服务重启后客户端凭据仍可认证", "hostService", "persisted_client_token_survives_host_runtime_restart"],
      ["旧数据库可兼容迁移客户端连接表", "migrationsService", "compatibility_migrations_upgrade_existing_version_one_database"],
      ["客户端连接表按设备 ID 去重", "migrationsService", "idx_client_connections_device_id"],
      ["基础资料编辑使用 updated_at 乐观锁", "masterDataRepository", "save_item_requires_matching_updated_at_for_existing_records"],
      ["前端编辑基础资料传递 expectedUpdatedAt", "app", "expectedUpdatedAt: item.updatedAt"],
      ["基础资料启停也使用 updated_at 乐观锁", "masterDataRepository", "set_item_enabled(&conn, &updated.id, false, Some(&updated.updated_at))"],
      ["前端启停基础资料传递 expectedUpdatedAt", "app", "expectedUpdatedAt,"],
      ["基础资料启停不存在记录会报错", "masterDataRepository", "set_enabled_requires_existing_master_data_record"],
      ["用户保存拒绝未知角色", "userRepository", "save_user_rejects_unknown_role_codes"],
      ["用户启停不存在账号会报错", "userRepository", "set_user_enabled_rejects_missing_user"],
      ["部门查看员必须绑定启用部门", "userService", "save_user_requires_enabled_department_for_department_viewer"],
      ["用户管理保留至少一个启用管理员", "userService", "save_and_disable_user_preserve_last_enabled_admin"],
      ["切换主机会清除旧 token", "hostService", "save_client_config_clears_pairing_token_when_host_changes"],
      ["远程权限控制有测试覆盖", "hostService", "require_remote_permission_rejects_readonly_user"],
      ["远程库存盘点写入复用服务校验", "hostService", "remote_stock_and_stocktake_routes_reuse_service_validation"],
      ["远程基础资料写入复用服务校验", "hostService", "remote_master_data_routes_reuse_service_validation"],
      ["远程预算写入复用服务校验", "hostService", "remote_budget_route_reuses_service_validation"],
      ["本机库存列表强制部门查看员范围", "stockService", "department_viewer_stock_lists_are_scoped_to_bound_department"],
      ["远程库存列表强制部门查看员范围", "hostService", "remote_stock_lists_force_department_viewer_scope"],
      ["跨平台验收文档包含互为主机客户端", "crossPlatformAcceptance", "Windows 主机 + macOS 客户端"],
      ["客户端断线时回退 shell 状态有测试覆盖", "statusService", "client_mode_status_with_unreachable_host_falls_back_to_local_shell_status"],
      ["未配对客户端暂停业务入口", "app", "isBusinessConnectionReady"],
    ],
  },
  {
    id: "budget_approval",
    name: "预算控制和审批",
    checks: [
      ["执行文档落地状态包含预算和审批", "executionPlan", "预算控制"],
      ["前端包含预算和审批页面", "app", "ApprovalsPage"],
      ["Tauri 注册预算和审批命令", "lib", "decide_approval_request"],
      ["命令层包含审批处理", "approvalCommands", "pub fn decide_approval_request"],
      ["库存确认可关联审批单", "stockCommands", "ConfirmStockDocumentDraftRequest"],
      ["超预算阻断有测试覆盖", "stockRepository", "submit_outbound_rejects_when_budget_limit_would_be_exceeded"],
      ["审批通过可放行超预算", "stockRepository", "approved_budget_override_allows_over_budget_outbound"],
      ["出库单持久化审批单 ID 可追溯", "stockRepository", "approval_request_id.as_deref()"],
      ["前端出库列表展示审批单 ID", "app", "approvalRequestId"],
      ["预算规则拒绝停用部门和分类", "masterDataRepository", "save_budget_rule_requires_enabled_department_and_category"],
      ["审批创建校验部门月份对象", "approvalService", "create_approval_request_validates_supported_budget_entity"],
      ["审批决策拒绝重复处理", "approvalRepository", "decide_approval_request_rejects_already_processed_request"],
      ["远程审批绑定远程用户", "hostService", "remote_approval_api_validates_entity_and_binds_remote_users"],
    ],
  },
  {
    id: "cross_platform_packaging",
    name: "Windows/macOS 打包与本地文件能力",
    checks: [
      ["执行文档包含跨平台要求", "executionPlan", "### 5.6 跨平台要求"],
      ["Tauri 配置目标为 all", "tauriConfig", "\"targets\": \"all\""],
      ["Windows NSIS 当前用户安装", "tauriConfig", "\"installMode\": \"currentUser\""],
      ["macOS DMG 布局已配置", "tauriConfig", "\"minimumSystemVersion\""],
      ["Excel 不依赖 Office/WPS 的库已配置", "cargoToml", "rust_xlsxwriter"],
      ["GitHub CI 已配置自动门禁", "githubCi", "npm run test:manual-acceptance"],
      ["GitHub CI 执行覆盖校验", "githubCi", "npm run verify:coverage"],
      ["GitHub CI 执行 Rust 测试", "githubCi", "cargo test"],
      ["GitHub 桌面构建包含 Windows x64", "githubDesktopBuild", "Windows x64"],
      ["GitHub 桌面构建运行 Windows runner", "githubDesktopBuild", "windows-2022"],
      ["GitHub 桌面构建指定 Windows x64 Rust target", "githubDesktopBuild", "x86_64-pc-windows-msvc"],
      ["GitHub 桌面构建包含 macOS", "githubDesktopBuild", "macOS"],
      ["GitHub 桌面构建运行 macOS Intel runner", "githubDesktopBuild", "macos-15-intel"],
      ["GitHub 桌面构建指定 macOS Intel Rust target", "githubDesktopBuild", "x86_64-apple-darwin"],
      ["GitHub 桌面构建运行 macOS Apple Silicon runner", "githubDesktopBuild", "macos-15"],
      ["GitHub 桌面构建指定 macOS Apple Silicon Rust target", "githubDesktopBuild", "aarch64-apple-darwin"],
      ["GitHub Release 收集 macOS Apple Silicon DMG", "githubDesktopBuild", "macos-aarch64.dmg"],
      ["GitHub 桌面构建执行 release 验证", "githubDesktopBuild", "npm run verify:release"],
      ["GitHub 桌面构建上传 Windows 安装器", "githubDesktopBuild", "aster-windows-x64"],
      ["GitHub 桌面构建上传 release evidence", "githubDesktopBuild", "release-evidence"],
      ["GitHub 桌面构建支持可选发布 Release", "githubDesktopBuild", "Publish GitHub Release"],
      ["GitHub Release 支持手动发布时更新既有 tag", "githubDesktopBuild", "--method PATCH"],
      ["GitHub Release 更新既有 tag 使用 force", "githubDesktopBuild", "-F force=true"],
      ["GitHub Release 只收集安装包 artifacts", "githubDesktopBuild", "copy_single_asset"],
      ["GitHub Release 校验 macOS Intel DMG asset", "githubDesktopBuild", "macOS Intel DMG release asset missing"],
      ["GitHub Release 校验 macOS Apple Silicon DMG asset", "githubDesktopBuild", "macOS Apple Silicon DMG release asset missing"],
      ["GitHub Release 校验 Windows installer asset", "githubDesktopBuild", "Windows installer release asset missing"],
      ["发布验证脚本已配置", "packageJson", "\"verify:release\""],
      ["发布验证脚本支持 Tauri target 透传", "verifyRelease", "TAURI_BUILD_TARGET"],
      ["发布验证脚本执行 target 构建参数", "verifyRelease", "--target"],
      ["发布验证脚本支持 target bundle 目录", "verifyRelease", "target\", tauriBuildTarget, \"release\", \"bundle"],
      ["GitHub 桌面构建上传 target bundle 产物", "githubDesktopBuild", "src-tauri/target/*/release/bundle"],
      ["本机自动总门禁脚本已配置", "packageJson", "\"verify:all-local\""],
      ["本机自动总门禁生成证据报告", "verifyAllLocal", "verify-all-local-${process.platform}"],
      ["本机自动总门禁执行 release 验证", "verifyAllLocal", "npm run verify:release"],
      ["本机自动总门禁刷新验收交接包", "verifyAllLocal", "npm run acceptance:package"],
      ["本机自动总门禁执行验收准备度校验", "verifyAllLocal", "npm run verify:readiness"],
      ["本机自动总门禁最终校验验收交接包", "verifyAllLocal", "npm run verify:acceptance-package"],
      ["验收准备度脚本已配置", "packageJson", "\"verify:readiness\""],
      ["验收准备度生成证据报告", "verifyReadiness", "readiness-${process.platform}"],
      ["验收准备度识别待实机证据状态", "verifyReadiness", "ready-for-manual-evidence"],
      ["验收准备度提示 Windows artifacts 导入", "verifyReadiness", "acceptance:import-windows-artifacts"],
      ["验收状态命令已配置", "packageJson", "\"acceptance:status\""],
      ["验收状态命令汇总剩余证据", "acceptanceStatus", "remainingEvidence"],
      ["验收状态命令汇总归档 SHA256", "acceptanceStatus", "archiveSha256"],
      ["验收状态命令提示 Windows artifacts 导入", "acceptanceStatus", "acceptance:import-windows-artifacts"],
      ["验收状态命令支持摘要输出", "acceptanceStatus", "--summary"],
      ["验收状态命令测试覆盖待实机证据状态", "testManualAcceptance", "setupAcceptanceStatusFixture"],
      ["验收交接包归档命令已配置", "packageJson", "\"acceptance:archive\""],
      ["验收交接包归档生成 zip", "archiveAcceptancePackage", "aster-acceptance-package-${platformName}-${timestamp}.zip"],
      ["验收交接包归档支持 Windows PowerShell", "archiveAcceptancePackage", "Compress-Archive"],
      ["验收交接包归档生成 SHA256 报告", "archiveAcceptancePackage", "acceptance-archive-${process.platform}"],
      ["最终验收归档命令已配置", "packageJson", "\"acceptance:finalize\""],
      ["最终验收归档先执行 strict 手工验收", "finalizeAcceptance", "npm run verify:manual-acceptance -- --strict"],
      ["最终验收归档要求 ready-for-final-archive", "finalizeAcceptance", "ready-for-final-archive"],
      ["最终验收归档会生成归档包", "finalizeAcceptance", "npm run acceptance:archive"],
      ["最终验收归档测试拒绝未完成实机证据", "testManualAcceptance", "setupFinalizeRejectsManualEvidenceFixture"],
      ["占位和未完成标记扫描脚本已配置", "packageJson", "\"verify:no-placeholders\""],
      ["占位和未完成标记扫描会生成证据报告", "verifyNoPlaceholders", "no-placeholder-scan-${process.platform}"],
      ["发布验证执行占位扫描", "verifyRelease", "npm run verify:no-placeholders"],
      ["手工严格验收要求占位扫描命令成功", "verifyManualAcceptance", "npm run verify:no-placeholders"],
      ["验收交接包携带占位扫描证据", "createAcceptancePackage", "no-placeholder-scan-${evidencePlatform}"],
      ["验收交接包校验占位扫描证据", "verifyAcceptancePackage", "no-placeholder-scan-${evidencePlatform}-"],
      ["验收交接包校验 release 证据状态", "verifyAcceptancePackage", "verify-release 最新证据状态必须为 passed"],
      ["验收交接包校验 coverage 证据状态", "verifyAcceptancePackage", "execution-coverage 最新证据状态必须为 covered"],
      ["验收交接包校验占位扫描证据状态", "verifyAcceptancePackage", "no-placeholder-scan 最新证据状态必须为 passed"],
      ["验收交接包携带本机总门禁证据", "createAcceptancePackage", "verify-all-local-${evidencePlatform}"],
      ["验收交接包校验本机总门禁证据", "verifyAcceptancePackage", "verify-all-local-${evidencePlatform}-"],
      ["验收交接包校验本机总门禁命令成功", "verifyAcceptancePackage", "verify-all-local 最新证据命令未通过"],
      ["验收交接包测试覆盖本机总门禁证据", "testManualAcceptance", "verify-all-local-${expectedEvidencePlatform}-"],
      ["验收交接包携带准备度证据", "createAcceptancePackage", "readiness-${evidencePlatform}"],
      ["验收交接包校验准备度证据", "verifyAcceptancePackage", "readiness-${evidencePlatform}-"],
      ["验收交接包校验准备度无自动阻塞项", "verifyAcceptancePackage", "readiness 最新证据 blockers 必须为空"],
      ["验收交接包测试覆盖准备度证据", "testManualAcceptance", "readiness-${expectedEvidencePlatform}-"],
      ["验收交接包测试拒绝失败占位扫描证据", "testManualAcceptance", "setupRejectedAcceptancePackageFixture"],
      ["浏览器预览缺少 Tauri 容器时显示友好提示", "app", "当前页面需要在 Aster 桌面客户端中运行"],
      ["发布验证记录每条命令执行结果", "verifyManualAcceptance", "requiredCommandResultsSuccessful"],
      [
        "README 说明 Windows 由 GitHub Actions 构建",
        "readme",
        /Windows (?:安装器由 GitHub Actions 的 `Build Desktop Bundles` workflow 在 Windows runner 上生成|installer is generated by the `Build Desktop Bundles` GitHub Actions workflow)/,
      ],
      ["双端验收文档包含 Windows/macOS", "crossPlatformAcceptance", "Aster Windows/macOS 双端验收文档"],
      ["实机采集脚本生成模板和清单", "collectAcceptanceEvidence", "manual-acceptance-${normalizedPlatform}-${date}-checklist.md"],
      ["Windows artifacts 导入命令已配置", "packageJson", "\"acceptance:import-windows-artifacts\""],
      ["Windows artifacts 自动下载命令已配置", "packageJson", "\"acceptance:download-windows-artifacts\""],
      ["GitHub 桌面构建触发命令已配置", "packageJson", "\"acceptance:run-github-build\""],
      ["GitHub 桌面构建触发使用 workflow dispatch", "runGithubDesktopBuild", "/dispatches"],
      ["GitHub 桌面构建触发会轮询 workflow run", "runGithubDesktopBuild", "workflow_dispatch"],
      ["GitHub 桌面构建触发成功后下载 Windows artifacts", "runGithubDesktopBuild", "acceptance:download-windows-artifacts"],
      ["GitHub 桌面构建触发要求 token", "runGithubDesktopBuild", "GITHUB_TOKEN"],
      ["Windows artifacts 自动下载读取 GitHub Actions", "downloadWindowsArtifacts", "/actions/workflows/"],
      ["Windows artifacts 自动下载目标包含安装器", "downloadWindowsArtifacts", "aster-windows-x64"],
      ["Windows artifacts 自动下载目标包含 release evidence", "downloadWindowsArtifacts", "aster-windows-x64-release-evidence"],
      ["Windows artifacts 自动下载后调用导入校验", "downloadWindowsArtifacts", "acceptance:import-windows-artifacts"],
      ["Windows artifacts 自动下载要求 token", "downloadWindowsArtifacts", "GITHUB_TOKEN"],
      ["Windows artifacts 导入复制 release evidence", "importWindowsArtifacts", "aster-windows-x64-release-evidence"],
      ["Windows artifacts 导入复制安装器", "importWindowsArtifacts", "windows-installers"],
      ["Windows artifacts 导入校验安装器 SHA256", "importWindowsArtifacts", "matchesReleaseEvidence"],
      ["Windows 验收文档说明自动下载 artifacts", "crossPlatformAcceptance", "acceptance:download-windows-artifacts"],
      ["Windows 验收文档说明触发 GitHub 构建", "crossPlatformAcceptance", "acceptance:run-github-build"],
      ["Windows 手工验收 README 说明自动下载 artifacts", "manualAcceptanceReadme", "acceptance:download-windows-artifacts"],
      ["Windows 手工验收 README 说明触发 GitHub 构建", "manualAcceptanceReadme", "acceptance:run-github-build"],
      ["Windows 采集清单提示导入 artifacts", "collectAcceptanceEvidence", "acceptance:import-windows-artifacts"],
      ["验收交接包说明 token 自动下载 artifacts", "createAcceptancePackage", "acceptance:download-windows-artifacts"],
      ["Windows runner 支持 token 自动触发 GitHub 构建", "createAcceptanceRunners", "acceptance:run-github-build"],
      ["Windows runner 支持 artifacts 自动导入参数", "createAcceptanceRunners", "ArtifactsDir"],
      ["实机采集脚本生成附件目录", "collectAcceptanceEvidence", "evidence-${normalizedPlatform}-${date}"],
      ["手工验收模板预填项目内附件相对路径", "collectAcceptanceEvidence", "05-exported-workbook-opened.png"],
      ["实机运行脚本包含 Windows PowerShell", "createAcceptanceRunners", "run-windows-acceptance.ps1"],
      ["实机运行脚本包含 macOS shell", "createAcceptanceRunners", "run-macos-acceptance.sh"],
      ["实机运行脚本说明项目根目录执行路径", "createAcceptanceRunners", "docs\\\\acceptance-package\\\\runners\\\\run-windows-acceptance.ps1"],
      ["实机运行脚本生成严格验收剩余证据汇总", "createAcceptanceRunners", "npm run verify:manual-acceptance -- --strict"],
      ["实机运行脚本执行 readiness 检查", "createAcceptanceRunners", "npm run verify:readiness"],
      ["实机运行脚本校验交接包完整性", "createAcceptanceRunners", "npm run verify:acceptance-package"],
      ["实机运行脚本生成验收归档包", "createAcceptanceRunners", "npm run acceptance:archive"],
      ["手工验收校验反查 release 报告", "verifyManualAcceptance", "validateReleaseReport"],
      ["手工验收校验反查 coverage 报告", "verifyManualAcceptance", "validateCoverageReport"],
      ["手工验收同平台多记录选择最新记录", "verifyManualAcceptance", "latestRecordForPlatform"],
      ["手工验收汇总按证据类型分组剩余项", "verifyManualAcceptance", "remainingEvidence"],
      ["手工验收要求截图附件证据", "verifyManualAcceptance", "evidenceFiles.installScreenshot"],
      ["手工验收要求样本数据证据", "verifyManualAcceptance", "sampleData.asHostBusinessDocumentNo"],
      ["验收交接包包含采集命令", "createAcceptancePackage", "acceptance:collect"],
      ["验收交接包包含运行脚本", "createAcceptancePackage", "create-acceptance-runners.mjs"],
      ["验收交接包按当前平台选择证据", "createAcceptancePackage", "evidencePlatform"],
      ["验收交接包校验当前平台证据", "verifyAcceptancePackage", "manifest.generatedOn?.evidencePlatform"],
      ["验收交接包携带剩余证据分组", "createAcceptancePackage", "remainingEvidence"],
      ["验收交接包 README 指向剩余证据分组", "createAcceptancePackage", "acceptance-package-manifest.json\\` 里的 \\`remainingEvidence"],
      ["验收交接包生成剩余证据 Markdown", "createAcceptancePackage", "REMAINING_EVIDENCE.md"],
      ["验收交接包校验剩余证据 Markdown", "verifyAcceptancePackage", "REMAINING_EVIDENCE.md 剩余证据组数必须匹配最新 summary"],
      ["验收交接包生成文件完整性清单", "createAcceptancePackage", "fileInventorySha256"],
      ["验收交接包校验文件完整性清单", "verifyAcceptancePackage", "fileInventory SHA256 不匹配"],
      ["验收交接包测试覆盖剩余证据 Markdown", "testManualAcceptance", "剩余证据组数：1"],
      ["验收交接包携带当前手工验收样例", "createAcceptancePackage", "copiedManualAcceptance"],
      ["验收交接包校验当前手工验收样例", "verifyAcceptancePackage", "manifest.copiedManualAcceptance"],
      ["验收交接包校验文档内容新鲜度", "verifyAcceptancePackage", "交接包文档不是最新"],
      ["验收交接包校验手工样例内容新鲜度", "verifyAcceptancePackage", "交接包手工验收样例不是最新"],
      ["验收交接包测试覆盖附件 README 复制", "testManualAcceptance", "evidence-${expectedManualPlatform}-${fixtureManualDate}/README.md"],
      ["验收交接包校验剩余证据分组", "verifyAcceptancePackage", "remainingEvidence.remainingCount"],
      ["验收交接包平台选择支持自动化模拟", "createAcceptancePackage", "ASTER_ACCEPTANCE_PLATFORM"],
      ["验收交接包平台选择覆盖 Windows 和 macOS", "testManualAcceptance", "setupAcceptancePackageFixture(\"win32\")"],
      ["手工验收说明包含真实性校验", "manualAcceptanceReadme", "证据报告的平台、版本、覆盖状态、功能组、安装包路径和安装包 SHA256 与手工记录一致"],
      ["手工验收说明要求命令执行成功证据", "manualAcceptanceReadme", "逐条命令执行结果必须全部成功"],
    ],
  },
];

const featureReports = requirements.map((requirement) => {
  const checks = requirement.checks.map(([checkName, sourceKey, pattern]) =>
    check(requirement.id, checkName, sourceKey, pattern),
  );
  for (const item of checks) {
    if (!item.ok) {
      failures.push({
        feature: requirement.id,
        check: item.check,
        source: item.source,
        pattern: item.pattern,
      });
    }
  }
  return {
    id: requirement.id,
    name: requirement.name,
    status: checks.every((item) => item.ok) ? "covered" : "missing-evidence",
    checks,
  };
});

const commandConsistency = tauriCommandConsistencyReport();
if (commandConsistency.invokedWithoutRegistration.length > 0) {
  failures.push({
    feature: commandConsistency.id,
    check: `前端调用了未注册的 Tauri 命令：${commandConsistency.invokedWithoutRegistration.join(", ")}`,
    source: `${sources.app} / ${sources.lib}`,
    pattern: "invoke(...) must be registered in generate_handler![...]",
  });
}

const features = [...featureReports, commandConsistency];
const report = {
  generatedAt: new Date().toISOString(),
  platform: platform(),
  platformRelease: release(),
  arch: arch(),
  sourceDocument: sources.executionPlan,
  coverageStatus: failures.length === 0 ? "covered" : "missing-evidence",
  features,
  remainingManualEvidence: [
    "Windows GitHub Actions 安装器下载、安装和首次启动",
    "Windows 主机 + macOS 客户端连接、断线、恢复和业务写入",
    "macOS 主机 + Windows 客户端连接、断线、恢复和业务写入",
    "Windows/macOS 双端备份恢复",
    "Windows/macOS 双端 Excel 导入导出后人工打开确认",
  ],
};

const evidenceDir = join(root, "docs", "release-evidence");
mkdirSync(evidenceDir, { recursive: true });
const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
const reportPath = join(evidenceDir, `execution-coverage-${process.platform}-${timestamp}.json`);
writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);

if (failures.length > 0) {
  console.error("\n[verify-execution-coverage] Missing execution-plan evidence:");
  for (const failure of failures) {
    console.error(`- ${failure.feature}: ${failure.check} (${failure.source})`);
  }
  console.error(`\n[verify-execution-coverage] Evidence report: ${reportPath}`);
  process.exit(1);
}

console.log(`[verify-execution-coverage] Covered ${features.length} execution-plan feature groups.`);
console.log(`[verify-execution-coverage] Evidence report: ${reportPath}`);
