import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ApprovalRequest } from "../../entities/approvals";
import type { AuditLogRow, BackupRecord } from "../../entities/operations";
import type { AppStatus, ClientConnectionInfo, HostConnectionTestResult, HostServiceStatus, RuntimeConfig, SystemSettings } from "../../entities/runtime";
import type { BudgetRule, Category, Department, Item, Supplier, SupplierPurchaseRecord, Unit } from "../../entities/master-data";
import type { CurrentUser, UserAccount } from "../../entities/users";
import type { StockBalanceQuery, StockBalanceRow, StockDocument, StockDocumentQuery, StockMovementQuery, StockMovementRow, StocktakeDocument } from "../../entities/stock";
import type { ReportBundle, ReportQuery } from "../../entities/reports";
import { formatError } from "../../shared/lib/appRuntime";
import type { MainAppState } from "./useMainAppState";

const currentMonthString = () => new Date().toISOString().slice(0, 7);
const CLIENT_RECONNECT_INTERVAL_MS = 15000;

export function useMainDataController(state: MainAppState) {
  const {
    adjustmentDocumentQuery, currentUser, hasManualReportMonth, inboundDocumentQuery,
    itemSearch, outboundDocumentQuery, reportMonth, reportQuery, setActiveNav,
    setActiveSupplier, setAdjustmentDocumentQuery, setAdjustmentDocuments,
    setApprovalRequests, setAuditLogs, setBackupRecords, setBudgetRules, setCategories,
    setClientConnectionCheckedAt, setClientConnections, setCurrentUser, setDepartments,
    setError, setHostStatus, setHostTestResult, setImportPreview, setImportResult,
    setInboundDocumentQuery, setInboundDocuments, setItems, setLastBackup, setLastExportPath,
    setOutboundDocumentQuery, setOutboundDocuments, setReportBundle, setReportMonth,
    setReportQuery, setPasswordChangeRequired, setHasManualReportMonth,
    setStockBalanceQuery, setStockBalances, setStockMovementQuery,
    setStockMovements, setStocktakes, setSupplierPurchaseRecords, setSuppliers,
    setSystemSettings, setUnits, setUserAccounts, status, setStatus, stockBalanceQuery,
    stockMovementQuery,
  } = state;
  const isBusinessConnectionReady =
    status?.runtime.mode !== "client" ||
    (Boolean(status.runtime.clientToken) && state.hostTestResult?.ok === true);
  function clearBusinessState() {
    setCategories([]);
    setUnits([]);
    setDepartments([]);
    setSuppliers([]);
    setSupplierPurchaseRecords([]);
    setActiveSupplier(null);
    setBudgetRules([]);
    setApprovalRequests([]);
    setItems([]);
    setInboundDocuments([]);
    setOutboundDocuments([]);
    setAdjustmentDocuments([]);
    setInboundDocumentQuery({
      documentType: "inbound",
      month: currentMonthString(),
    });
    setOutboundDocumentQuery({
      documentType: "outbound",
      month: currentMonthString(),
    });
    setAdjustmentDocumentQuery({
      documentType: "adjustment",
      month: currentMonthString(),
    });
    setStockBalances([]);
    setStockMovements([]);
    setStockBalanceQuery({});
    setStockMovementQuery({});
    setStocktakes([]);
    setReportQuery({ month: reportMonth });
    setReportBundle(null);
    setLastExportPath(null);
  }

  function clearAdminState() {
    setBackupRecords([]);
    setAuditLogs([]);
    setLastBackup(null);
    setSystemSettings(null);
    setUserAccounts([]);
    setBudgetRules([]);
    setApprovalRequests([]);
  }

  function clearImportState() {
    setImportPreview(null);
    setImportResult(null);
  }

  function clearSessionScopedState() {
    clearBusinessState();
    clearAdminState();
    clearImportState();
  }

  async function loadCurrentUser() {
    const user = await invoke<CurrentUser | null>("get_current_user");
    setCurrentUser(user);
    return user;
  }

  function scheduleRefreshAll(search = itemSearch) {
    window.requestAnimationFrame(() => {
      window.setTimeout(() => {
        void refreshAll(search);
      }, 0);
    });
  }

  async function bootstrapSession() {
    try {
      setError(null);
      const [user, nextStatus] = await Promise.all([
        invoke<CurrentUser | null>("get_current_user"),
        invoke<AppStatus>("get_app_status"),
      ]);
      setCurrentUser(user);
      setStatus(nextStatus);
      if (user) {
        const mustChangePassword = await invoke<boolean>(
          "get_password_change_required",
        );
        setPasswordChangeRequired(mustChangePassword);
        if (mustChangePassword) {
          clearSessionScopedState();
          return;
        }
        if (
          user.permissions.includes("view_reports") &&
          !hasManualReportMonth
        ) {
          const effectiveMonth =
            nextStatus.latestMovementMonth ?? reportQuery.month;
          if (effectiveMonth !== reportQuery.month) {
            const effectiveQuery = { ...reportQuery, month: effectiveMonth };
            setReportMonth(effectiveMonth);
            setReportQuery(effectiveQuery);
          }
        }
        scheduleRefreshAll();
      } else {
        clearSessionScopedState();
      }
    } catch (err) {
      setError(formatError(err));
    }
  }

  async function loadMasterData(search = itemSearch) {
    const [
      nextCategories,
      nextUnits,
      nextDepartments,
      nextSuppliers,
      nextItems,
    ] = await Promise.all([
      invoke<Category[]>("list_categories"),
      invoke<Unit[]>("list_units"),
      invoke<Department[]>("list_departments"),
      invoke<Supplier[]>("list_suppliers"),
      invoke<Item[]>("list_items", { search }),
    ]);
    setCategories(nextCategories);
    setUnits(nextUnits);
    setDepartments(nextDepartments);
    setSuppliers(nextSuppliers);
    setItems(nextItems);
  }

  async function loadStockData() {
    const [
      nextInbound,
      nextOutbound,
      nextAdjustments,
      nextBalances,
      nextMovements,
      nextStocktakes,
    ] = await Promise.all([
      invoke<StockDocument[]>("list_stock_documents", {
        query: inboundDocumentQuery,
      }),
      invoke<StockDocument[]>("list_stock_documents", {
        query: outboundDocumentQuery,
      }),
      invoke<StockDocument[]>("list_stock_documents", {
        query: adjustmentDocumentQuery,
      }),
      invoke<StockBalanceRow[]>("list_stock_balances", {
        query: stockBalanceQuery,
      }),
      invoke<StockMovementRow[]>("list_stock_movements", {
        query: stockMovementQuery,
      }),
      invoke<StocktakeDocument[]>("list_stocktakes"),
    ]);
    setInboundDocuments(nextInbound);
    setOutboundDocuments(nextOutbound);
    setAdjustmentDocuments(nextAdjustments);
    setStockBalances(nextBalances);
    setStockMovements(nextMovements);
    setStocktakes(nextStocktakes);
  }

  async function loadStockDocuments(query: StockDocumentQuery) {
    const nextDocuments = await invoke<StockDocument[]>(
      "list_stock_documents",
      { query },
    );
    if (query.documentType === "inbound") {
      setInboundDocuments(nextDocuments);
    } else if (query.documentType === "outbound") {
      setOutboundDocuments(nextDocuments);
    } else if (query.documentType === "adjustment") {
      setAdjustmentDocuments(nextDocuments);
    }
  }

  async function applyStockDocumentQuery(query: StockDocumentQuery) {
    if (query.documentType === "inbound") {
      setInboundDocumentQuery(query);
    } else if (query.documentType === "outbound") {
      setOutboundDocumentQuery(query);
    } else if (query.documentType === "adjustment") {
      setAdjustmentDocumentQuery(query);
    }
    await loadStockDocuments(query);
  }

  async function applyStockBalanceQuery(query: StockBalanceQuery) {
    const normalizedQuery: StockBalanceQuery = {
      search: null,
      categoryId: query.categoryId || null,
      itemId: query.itemId || null,
      stockStatus: query.stockStatus || null,
    };
    setStockBalanceQuery(normalizedQuery);
    const nextBalances = await invoke<StockBalanceRow[]>(
      "list_stock_balances",
      {
        query: normalizedQuery,
      },
    );
    setStockBalances(nextBalances);
  }

  async function applyStockMovementQuery(query: StockMovementQuery) {
    const normalizedQuery: StockMovementQuery = {
      search: query.search?.trim() || null,
      itemId: query.itemId || null,
      direction: query.direction || null,
      movementType: query.movementType || null,
    };
    setStockMovementQuery(normalizedQuery);
    const nextMovements = await invoke<StockMovementRow[]>(
      "list_stock_movements",
      {
        query: normalizedQuery,
      },
    );
    setStockMovements(nextMovements);
  }

  async function showItemMovements(itemId: string) {
    setActiveNav("movements");
    await applyStockMovementQuery({ itemId });
  }

  async function loadReports(query = reportQuery) {
    const nextReport = await invoke<ReportBundle>("get_report_bundle", {
      query,
    });
    setReportBundle(nextReport);
  }

  async function loadReportsForStatus(
    nextStatus: AppStatus,
    manualMonth = hasManualReportMonth,
  ) {
    const effectiveMonth = manualMonth
      ? reportQuery.month
      : (nextStatus.latestMovementMonth ?? reportQuery.month);
    const effectiveQuery: ReportQuery = {
      ...reportQuery,
      month: effectiveMonth,
    };
    if (effectiveQuery.month !== reportQuery.month) {
      setReportMonth(effectiveQuery.month);
      setReportQuery(effectiveQuery);
    }
    await loadReports(effectiveQuery);
  }

  async function applyReportQuery(query: ReportQuery) {
    const normalizedQuery: ReportQuery = {
      month: query.month,
      startDate: query.startDate || null,
      endDate: query.endDate || null,
      departmentId: query.departmentId || null,
      categoryId: query.categoryId || null,
      itemId: query.itemId || null,
      supplierId: query.supplierId || null,
    };
    setHasManualReportMonth(true);
    setReportMonth(normalizedQuery.month);
    setReportQuery(normalizedQuery);
    await loadReports(normalizedQuery);
  }

  async function loadBackups() {
    const records = await invoke<BackupRecord[]>("list_backup_records");
    setBackupRecords(records);
  }

  async function loadAuditLogs() {
    const records = await invoke<AuditLogRow[]>("list_audit_logs", {
      limit: 120,
    });
    setAuditLogs(records);
  }

  async function loadHostRuntime() {
    const [nextStatus, nextClients] = await Promise.all([
      invoke<HostServiceStatus>("get_host_service_status"),
      invoke<ClientConnectionInfo[]>("list_client_connections"),
    ]);
    setHostStatus(nextStatus);
    setClientConnections(nextClients);
  }

  async function probeConfiguredHost(runtime: RuntimeConfig) {
    if (runtime.mode !== "client" || !runtime.hostAddress) {
      setClientConnectionCheckedAt(null);
      setHostTestResult(null);
      return true;
    }
    try {
      const result = await invoke<HostConnectionTestResult>(
        "test_host_connection",
        {
          request: {
            hostAddress: runtime.hostAddress,
            hostPort: runtime.hostPort,
          },
        },
      );
      setHostTestResult(result);
      return result.ok;
    } catch (err) {
      setHostTestResult({
        ok: false,
        message: `主机连接异常：${String(err)}`,
        appName: null,
        appVersion: null,
        schemaVersion: null,
      });
      return false;
    } finally {
      setClientConnectionCheckedAt(new Date().toLocaleString("zh-CN"));
    }
  }

  useEffect(() => {
    if (status?.runtime.mode !== "client" || !status.runtime.hostAddress) {
      return;
    }
    const runtime = status.runtime;
    const intervalId = window.setInterval(() => {
      void probeConfiguredHost(runtime);
    }, CLIENT_RECONNECT_INTERVAL_MS);
    return () => window.clearInterval(intervalId);
  }, [
    status?.runtime.mode,
    status?.runtime.hostAddress,
    status?.runtime.hostPort,
    status?.runtime.clientToken,
  ]);

  async function loadSystemSettings() {
    const settings = await invoke<SystemSettings>("get_system_settings");
    setSystemSettings(settings);
  }

  async function loadUsers(user = currentUser) {
    if (!user?.roles.some((role) => role.code === "admin")) {
      return;
    }
    const nextUsers = await invoke<UserAccount[]>("list_user_accounts");
    setUserAccounts(nextUsers);
  }

  async function loadBudgetRules(month = reportMonth) {
    const rules = await invoke<BudgetRule[]>("list_budget_rules", {
      periodMonth: month,
    });
    setBudgetRules(rules);
  }

  async function loadApprovalRequests() {
    const approvals = await invoke<ApprovalRequest[]>("list_approval_requests");
    setApprovalRequests(approvals);
  }

  async function loadSupplierPurchaseRecords(supplier: Supplier) {
    if (!isBusinessConnectionReady) return;
    try {
      setError(null);
      setActiveSupplier(supplier);
      const records = await invoke<SupplierPurchaseRecord[]>(
        "list_supplier_purchase_records",
        {
          supplierId: supplier.id,
        },
      );
      setSupplierPurchaseRecords(records);
    } catch (err) {
      setError(formatError(err));
    }
  }

  async function refreshAll(search = itemSearch) {
    try {
      setError(null);
      const user = await loadCurrentUser();
      const nextStatus = await invoke<AppStatus>("get_app_status");
      setStatus(nextStatus);
      const isUnpairedClient =
        nextStatus.runtime.mode === "client" && !nextStatus.runtime.clientToken;
      let clientHostOk = true;
      if (nextStatus.runtime.mode === "client") {
        clientHostOk = await probeConfiguredHost(nextStatus.runtime);
      } else {
        setClientConnectionCheckedAt(null);
      }
      const canLoadBusinessData =
        Boolean(user) &&
        !isUnpairedClient &&
        (nextStatus.runtime.mode !== "client" || clientHostOk);
      const canViewReports =
        user?.permissions.includes("view_reports") ?? false;
      if (!canLoadBusinessData) {
        clearBusinessState();
        clearImportState();
      } else if (!canViewReports) {
        setReportBundle(null);
        setLastExportPath(null);
      }
      await Promise.all([
        canLoadBusinessData ? loadMasterData(search) : Promise.resolve(),
        canLoadBusinessData ? loadStockData() : Promise.resolve(),
        canLoadBusinessData && canViewReports
          ? loadReportsForStatus(nextStatus)
          : Promise.resolve(),
      ]);
      if (user?.roles.some((role) => role.code === "admin")) {
        if (
          isUnpairedClient ||
          (nextStatus.runtime.mode === "client" && !clientHostOk)
        ) {
          clearAdminState();
          await loadHostRuntime();
        } else {
          await Promise.all([
            loadBackups(),
            loadUsers(user),
            loadHostRuntime(),
            loadSystemSettings(),
            loadAuditLogs(),
            loadBudgetRules(reportMonth),
            loadApprovalRequests(),
          ]);
        }
      } else {
        clearAdminState();
      }
    } catch (err) {
      setError(formatError(err));
    }
  }

  return {
    applyReportQuery, applyStockBalanceQuery, applyStockDocumentQuery,
    applyStockMovementQuery, bootstrapSession, clearSessionScopedState,
    loadBudgetRules, loadHostRuntime, loadSupplierPurchaseRecords, loadUsers,
    refreshAll, scheduleRefreshAll,
    showItemMovements,
  };
}
export type MainDataController = ReturnType<typeof useMainDataController>;
