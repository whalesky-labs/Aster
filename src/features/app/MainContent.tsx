import { invoke } from "@tauri-apps/api/core";
import { open, type OpenDialogOptions } from "@tauri-apps/plugin-dialog";
import type { I18n } from "../../i18n";
import type { MainAppState } from "./useMainAppState";
import type { MainDataController } from "./useMainDataController";
import type { MainActions } from "./useMainActions";
import { openEditorWindow } from "../../shared/lib/editorWindows";
import { formatError } from "../../shared/lib/appRuntime";
import { Dashboard } from "../dashboard/Dashboard";
import { ItemsPage } from "../master-data/ItemsPage";
import { CategoriesPage, DepartmentsPage, UnitsPage } from "../master-data/MasterDataPages";
import { SuppliersPage } from "../master-data/SuppliersPage";
import { BudgetRulesPage } from "../master-data/BudgetRulesPage";
import { ApprovalsPage } from "../approvals/ApprovalsPage";
import { StockDocumentPage } from "../stock/StockDocumentPage";
import { StockBalancePage } from "../stock/StockBalancePage";
import { StockMovementPage } from "../stock/StockMovementPage";
import { StocktakePage } from "../stock/StocktakeViews";
import { AdjustmentPage } from "../stock/AdjustmentPage";
import { ReportsPage } from "../reports/ReportsPage";
import { ImportPage } from "../imports/ImportPage";
import { SettingsPage } from "../settings/SettingsPage";
import { BackupRecordsPage } from "../backups/BackupRecordsPage";
import { LogsPage } from "../audit/LogsPage";
import { UsersPage } from "../users/UsersPage";
import { workstreams } from "../navigation/navigation";

async function chooseSinglePath(options: OpenDialogOptions) {
  const selected = await open({ ...options, multiple: false });
  if (typeof selected === "string") return selected;
  return Array.isArray(selected) ? selected[0] ?? null : null;
}
function uniqueTextOptions(values: Array<string | null | undefined>) {
  return [...new Set(values.map((value) => String(value ?? "").trim()).filter(Boolean))];
}
function userDisplayName(user?: { displayName?: string | null; username: string } | null) {
  return user ? user.displayName?.trim() || user.username : "";
}

type View = {
  canManageRemoteBusiness: boolean; canManageSettings: boolean; canUseLocalImport: boolean;
  canViewReports: boolean; canWriteStock: boolean;
  enabledCategories: MainAppState["categories"]; enabledSuppliers: MainAppState["suppliers"];
  enabledUnits: MainAppState["units"];
  formatDateTime: (value?: string | null) => string; formatMoney: (value: number) => string;
  metricCards: Array<{ label: string; value: string | number; suffix: string }>;
  modeLabel: (mode: "standalone" | "host" | "client") => string;
  movementTypeLabel: (type: string) => string;
};

export function MainContent({ actions, data, i18n, state, view }: {
  actions: MainActions; data: MainDataController; i18n: I18n; state: MainAppState; view: View;
}) {
  const {
    activeNav, activeSupplier, adjustmentDocumentQuery, adjustmentDocuments, appearanceSettings,
    approvalRequests, auditLogs, backupRecords, budgetRules, categories, clientConnectionCheckedAt,
    clientConnections, currentUser, departments, hostStatus, hostTestResult, importPreview, importResult,
    inboundDocumentQuery, inboundDocuments, isBackupWorking, isImporting, isSavingMode, itemSearch,
    items, lastBackup, lastExportPath, outboundDocumentQuery, outboundDocuments, reportBundle,
    reportMonth, reportQuery, setActiveNav, setAppearanceSettings, setBudgetRules, setError,
    setHasManualReportMonth, setItemSearch, setReportMonth, stockBalanceQuery, stockBalances,
    stockMovementQuery, stockMovements, stocktakes, supplierPurchaseRecords, suppliers,
    status, systemSettings, units, updateState, userAccounts,
  } = state;
  const {
    changeMode, createManualBackup, decideApprovalRequest, exportImportTemplate, exportItems,
    exportReport, importItemsFromToolbar, logoutUser, previewImport, removeClientConnection,
    runAction, runImport, startHostRuntime, toggleUserAccount, voidDocument,
  } = actions;
  const {
    applyReportQuery, applyStockBalanceQuery, applyStockDocumentQuery, applyStockMovementQuery,
    loadBudgetRules, loadSupplierPurchaseRecords, refreshAll, showItemMovements,
  } = data;
  const {
    canManageRemoteBusiness, canManageSettings, canUseLocalImport, canViewReports, canWriteStock,
    enabledCategories, enabledSuppliers, enabledUnits, formatDateTime, formatMoney, metricCards,
    modeLabel, movementTypeLabel,
  } = view;
  return <>
          {activeNav === "dashboard" ? (
            <Dashboard
              changeMode={changeMode}
              i18n={i18n}
              isSavingMode={isSavingMode}
              metricCards={metricCards}
              modeLabel={modeLabel}
              movementTypeLabel={movementTypeLabel}
              onNavigate={setActiveNav}
              status={status}
              workstreams={workstreams}
            />
          ) : null}

          {activeNav === "items" ? (
            <ItemsPage
              canImportItems={canWriteStock && canUseLocalImport}
              canWrite={canWriteStock}
              categories={enabledCategories}
              formatMoney={formatMoney}
              itemSearch={itemSearch}
              items={items}
              onCreate={() => openEditorWindow("item")}
              onEdit={(id) => openEditorWindow("item", { mode: "edit", id })}
              onSearch={async (search) => {
                setItemSearch(search);
                await refreshAll(search);
              }}
              onImportItems={importItemsFromToolbar}
              onExportItems={() => exportItems(itemSearch)}
              onToggle={(id, enabled, expectedUpdatedAt) =>
                runAction("物品状态已更新", () =>
                  invoke("set_item_enabled", {
                    id,
                    enabled,
                    expectedUpdatedAt,
                  }),
                )
              }
              suppliers={enabledSuppliers}
              units={enabledUnits}
            />
          ) : null}

        {activeNav === "departments" ? (
          <DepartmentsPage
            canWrite={canWriteStock}
            departments={departments}
            onCreate={() => openEditorWindow("department")}
            onEdit={(id) => openEditorWindow("department", { mode: "edit", id })}
            onToggle={(id, enabled, expectedUpdatedAt) =>
              runAction("部门状态已更新", () =>
                invoke("set_department_enabled", {
                  id,
                  enabled,
                  expectedUpdatedAt,
                }),
              )
            }
          />
        ) : null}

        {activeNav === "categories" ? (
          <CategoriesPage
            canWrite={canWriteStock}
            categories={categories}
            onCreate={() => openEditorWindow("category")}
            onEdit={(id) => openEditorWindow("category", { mode: "edit", id })}
            onToggle={(id, enabled, expectedUpdatedAt) =>
              runAction("分类状态已更新", () =>
                invoke("set_category_enabled", {
                  id,
                  enabled,
                  expectedUpdatedAt,
                }),
              )
            }
          />
        ) : null}

        {activeNav === "units" ? (
          <UnitsPage
            canWrite={canWriteStock}
            items={units}
            onCreate={() => openEditorWindow("unit")}
            onEdit={(id) => openEditorWindow("unit", { mode: "edit", id })}
            onToggle={(id, enabled, expectedUpdatedAt) =>
              runAction("单位状态已更新", () =>
                invoke("set_unit_enabled", { id, enabled, expectedUpdatedAt }),
              )
            }
          />
        ) : null}

        {activeNav === "suppliers" ? (
          <SuppliersPage
            canWrite={canWriteStock}
            activeSupplier={activeSupplier}
            formatDateTime={formatDateTime}
            formatMoney={formatMoney}
            onCreate={() => openEditorWindow("supplier")}
            onEdit={(id) => openEditorWindow("supplier", { mode: "edit", id })}
            onSelect={loadSupplierPurchaseRecords}
            onToggle={(id, enabled, expectedUpdatedAt) =>
              runAction("供应商状态已更新", () =>
                invoke("set_supplier_enabled", {
                  id,
                  enabled,
                  expectedUpdatedAt,
                }),
              )
            }
            purchaseRecords={supplierPurchaseRecords}
            suppliers={suppliers}
          />
        ) : null}

        {activeNav === "budgets" ? (
          <BudgetRulesPage
            canManage={canManageRemoteBusiness}
            formatMoney={formatMoney}
            month={reportMonth}
            onCreate={() => openEditorWindow("budget", { extra: { periodMonth: reportMonth } })}
            onEdit={(id, periodMonth) => openEditorWindow("budget", { mode: "edit", id, extra: { periodMonth } })}
            onMonthChange={async (month) => {
              setHasManualReportMonth(true);
              setReportMonth(month);
              if (canManageRemoteBusiness) {
                await loadBudgetRules(month);
              } else {
                setBudgetRules([]);
              }
            }}
            onToggle={(id, enabled, expectedUpdatedAt) =>
              runAction("预算规则状态已更新", () =>
                invoke("set_budget_rule_enabled", {
                  id,
                  enabled,
                  expectedUpdatedAt,
                }),
              )
            }
            rules={budgetRules}
          />
        ) : null}

        {activeNav === "approvals" ? (
          <ApprovalsPage
            approvals={approvalRequests}
            canManage={canManageRemoteBusiness}
            formatDateTime={formatDateTime}
            statusLabel={(status) => i18n.approvalStatusLabel(status)}
            typeLabel={(type) => i18n.approvalTypeLabel(type)}
            onDecide={decideApprovalRequest}
          />
        ) : null}

        {activeNav === "inbound" ? (
          <StockDocumentPage
            canWrite={canWriteStock}
            departments={departments.filter((item) => item.enabled)}
            documentType="inbound"
            documents={inboundDocuments}
            handlerOptions={uniqueTextOptions([
              ...userAccounts.map(userDisplayName),
              userDisplayName(currentUser),
            ])}
            items={items.filter((item) => item.enabled)}
            onQueryChange={async (query) => {
              try {
                setError(null);
                await applyStockDocumentQuery(query);
              } catch (err) {
                setError(formatError(err));
              }
            }}
            onConfirmDraft={(documentId, approvalRequestId) =>
              runAction("入库草稿已确认，库存已更新", () =>
                invoke("confirm_stock_document_draft", {
                  request: { documentId, approvalRequestId },
                }),
              )
            }
            onVoid={voidDocument}
            query={inboundDocumentQuery}
            suppliers={suppliers.filter((item) => item.enabled)}
          />
        ) : null}

        {activeNav === "outbound" ? (
          <StockDocumentPage
            canWrite={canWriteStock}
            departments={departments.filter((item) => item.enabled)}
            documentType="outbound"
            documents={outboundDocuments}
            handlerOptions={uniqueTextOptions([
              ...userAccounts.map(userDisplayName),
              userDisplayName(currentUser),
            ])}
            items={items.filter((item) => item.enabled)}
            onQueryChange={async (query) => {
              try {
                setError(null);
                await applyStockDocumentQuery(query);
              } catch (err) {
                setError(formatError(err));
              }
            }}
            onConfirmDraft={(documentId, approvalRequestId) =>
              runAction("出库/领用草稿已确认，库存已更新", () =>
                invoke("confirm_stock_document_draft", {
                  request: { documentId, approvalRequestId },
                }),
              )
            }
            onVoid={voidDocument}
            query={outboundDocumentQuery}
            suppliers={suppliers.filter((item) => item.enabled)}
          />
        ) : null}

        {activeNav === "stock" ? (
          <StockBalancePage
            balances={stockBalances}
            categories={enabledCategories}
            items={items.filter((item) => item.enabled)}
            onQueryChange={async (query) => {
              try {
                setError(null);
                await applyStockBalanceQuery(query);
              } catch (err) {
                setError(formatError(err));
              }
            }}
            onViewMovements={async (itemId) => {
              try {
                setError(null);
                await showItemMovements(itemId);
              } catch (err) {
                setError(formatError(err));
              }
            }}
            onViewBatches={(itemId) =>
              openEditorWindow("stockBatchDetail", {
                id: itemId,
                mode: "edit",
              })
            }
            query={stockBalanceQuery}
          />
        ) : null}

        {activeNav === "movements" ? (
          <StockMovementPage
            items={items.filter((item) => item.enabled)}
            movements={stockMovements}
            onQueryChange={async (query) => {
              try {
                setError(null);
                await applyStockMovementQuery(query);
              } catch (err) {
                setError(formatError(err));
              }
            }}
            query={stockMovementQuery}
          />
        ) : null}

        {activeNav === "stocktake" ? (
          <StocktakePage
            canWrite={canWriteStock}
            stocktakes={stocktakes}
          />
        ) : null}

        {activeNav === "adjustments" ? (
          <AdjustmentPage
            canWrite={canWriteStock}
            documents={adjustmentDocuments}
            handlerOptions={uniqueTextOptions([
              ...userAccounts.map(userDisplayName),
              userDisplayName(currentUser),
            ])}
            items={items.filter((item) => item.enabled)}
            onQueryChange={async (query) => {
              try {
                setError(null);
                await applyStockDocumentQuery(query);
              } catch (err) {
                setError(formatError(err));
              }
            }}
            onVoid={voidDocument}
            query={adjustmentDocumentQuery}
          />
        ) : null}

        {activeNav === "reports" ? (
          <ReportsPage
            bundle={reportBundle}
            categories={enabledCategories}
            canViewReports={canViewReports}
            departments={departments.filter((item) => item.enabled)}
            exportPath={lastExportPath}
            items={items.filter((item) => item.enabled)}
            onExport={exportReport}
            onQueryChange={applyReportQuery}
            query={reportQuery}
            suppliers={suppliers.filter((item) => item.enabled)}
          />
        ) : null}

        {activeNav === "import" ? (
          <ImportPage
            canPreviewImport={canUseLocalImport}
            canRunImport={canWriteStock && canUseLocalImport}
            isWorking={isImporting}
            formatMoney={formatMoney}
            onExportTemplate={exportImportTemplate}
            onPreview={previewImport}
            onRun={runImport}
            onSelectFile={() => chooseSinglePath({ title: "选择 Excel 导入文件", filters: [{ name: "Excel 工作簿", extensions: ["xlsx"] }] })}
            preview={importPreview}
            result={importResult}
          />
        ) : null}

        {activeNav === "settings" ? (
          <SettingsPage
            appearanceSettings={appearanceSettings}
            canManage={canManageSettings}
            clientConnectionCheckedAt={clientConnectionCheckedAt}
            currentUser={currentUser!}
            isWorking={isBackupWorking}
            lastBackup={lastBackup}
            onBackup={createManualBackup}
            onOpenConnectionWizard={() =>
              void openEditorWindow("connectionWizard", {
                width: 760,
                height: 640,
              })
            }
            onOpenSoftwareUpdate={() => void openEditorWindow("softwareUpdate")}
            onLogout={logoutUser}
            onOpenChangePassword={() => void openEditorWindow("changePassword", { width: 520, height: 360 })}
            onOpenBusinessSettings={() => void openEditorWindow("businessSettings", { width: 720, height: 620 })}
            onOpenSecondBackupDir={() => void openEditorWindow("secondBackupDir", { width: 620, height: 340 })}
            onOpenRestoreBackup={() => void openEditorWindow("restoreBackup", { width: 720, height: 560 })}
            onRemoveClientConnection={removeClientConnection}
            onStartHostService={startHostRuntime}
            clientConnections={clientConnections}
            hostStatus={hostStatus}
            hostTestResult={hostTestResult}
            i18n={i18n}
            status={status}
            systemSettings={systemSettings}
            updateState={updateState}
            onAppearanceChange={setAppearanceSettings}
          />
        ) : null}

        {activeNav === "backups" ? (
          <BackupRecordsPage backups={backupRecords} i18n={i18n} />
        ) : null}

        {activeNav === "logs" ? (
          <LogsPage auditLogs={auditLogs} i18n={i18n} />
        ) : null}

        {activeNav === "users" ? (
          <UsersPage
            currentUser={currentUser}
            onCreate={() => openEditorWindow("user")}
            onEdit={(id) => openEditorWindow("user", { mode: "edit", id })}
            onToggle={toggleUserAccount}
            users={userAccounts}
          />
        ) : null}
  </>;
}
