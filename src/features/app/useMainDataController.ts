import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppStatus } from "../../entities/runtime";
import type { Category, Department, Item, Supplier, Unit } from "../../entities/master-data";
import type { CurrentUser } from "../../entities/users";
import type { StockBalanceQuery, StockBalanceRow, StockDocument, StockDocumentQuery, StockMovementQuery, StockMovementRow, StocktakeDocument } from "../../entities/stock";
import type { ReportBundle, ReportQuery } from "../../entities/reports";
import type { DataPageKey, Page } from "../../entities/pagination";
import { formatError } from "../../shared/lib/appRuntime";
import { localMonth } from "../../shared/lib/localDate";
import type { MainAppState } from "./useMainAppState";
import { createMainDataAuxLoaders } from "./mainDataAuxLoaders";
import { createTargetedRefresher } from "./mainDataRefresh";
import { refreshTargetForEditor, type RefreshTarget } from "./refreshTargets";
const currentMonthString = localMonth;
const CLIENT_RECONNECT_INTERVAL_MS = 15000;
export function useMainDataController(state: MainAppState) {
  const {
    adjustmentDocumentQuery, hasManualReportMonth, inboundDocumentQuery,
    itemSearch, itemSupplierId, outboundDocumentQuery, reportMonth, reportQuery, setActiveNav,
    setActiveSupplier, setAdjustmentDocumentQuery, setAdjustmentDocuments,
    setApprovalRequests, setAuditLogs, setBackupRecords, setBudgetRules, setCategories,
    setClientConnectionCheckedAt, setCurrentUser, setDepartments,
    setError, setImportPreview, setImportResult,
    setInboundDocumentQuery, setInboundDocuments, setItems, setLastBackup, setLastExportPath,
    setOutboundDocumentQuery, setOutboundDocuments, setReportBundle, setReportMonth,
    setReportQuery, setPasswordChangeRequired, setHasManualReportMonth,
    setStockBalanceQuery, setStockBalances, setStockMovementQuery,
    setStockMovements, setStocktakes, setSupplierPurchaseRecords, setSuppliers,
    nextPageCursors, setNextPageCursors,
    setSystemSettings, setUnits, setUserAccounts, status, setStatus, stockBalanceQuery,
    stockMovementQuery,
  } = state;
  const isBusinessConnectionReady = status?.runtime.mode !== "client" ||
    (status.runtime.clientPaired && state.hostTestResult?.ok === true);
  const { loadApprovalRequests, loadAuditLogs, loadBackups, loadBudgetRules, loadHostRuntime,
    loadStatus, loadSupplierPurchaseRecords, loadSystemSettings, loadUsers, probeConfiguredHost } =
    createMainDataAuxLoaders(state, isBusinessConnectionReady);

  function setPageCursor(key: DataPageKey, cursor?: string | null) {
    setNextPageCursors((current) => {
      const next = { ...current };
      if (cursor) next[key] = cursor;
      else delete next[key];
      return next;
    });
  }
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
    setNextPageCursors({});
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

  function scheduleRefreshAll(search = itemSearch, supplierId = itemSupplierId) {
    window.requestAnimationFrame(() => {
      window.setTimeout(() => {
        void refreshAll(search, supplierId);
      }, 0);
    });
  }

  function scheduleRefresh(target: RefreshTarget) {
    window.requestAnimationFrame(() => {
      window.setTimeout(() => void refreshTarget(target), 0);
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

  async function loadMasterData(search = itemSearch, supplierId = itemSupplierId) {
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
      invoke<Page<Item>>("list_items_page", { search, supplierId }),
    ]);
    setCategories(nextCategories);
    setUnits(nextUnits);
    setDepartments(nextDepartments);
    setSuppliers(nextSuppliers);
    setItems(nextItems.items);
    setPageCursor("items", nextItems.nextCursor);
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
      invoke<Page<StockDocument>>("list_stock_documents_page", {
        query: inboundDocumentQuery,
      }),
      invoke<Page<StockDocument>>("list_stock_documents_page", {
        query: outboundDocumentQuery,
      }),
      invoke<Page<StockDocument>>("list_stock_documents_page", {
        query: adjustmentDocumentQuery,
      }),
      invoke<Page<StockBalanceRow>>("list_stock_balances_page", {
        query: stockBalanceQuery,
      }),
      invoke<Page<StockMovementRow>>("list_stock_movements_page", {
        query: stockMovementQuery,
      }),
      invoke<StocktakeDocument[]>("list_stocktakes"),
    ]);
    setInboundDocuments(nextInbound.items);
    setOutboundDocuments(nextOutbound.items);
    setAdjustmentDocuments(nextAdjustments.items);
    setStockBalances(nextBalances.items);
    setStockMovements(nextMovements.items);
    setPageCursor("inboundDocuments", nextInbound.nextCursor);
    setPageCursor("outboundDocuments", nextOutbound.nextCursor);
    setPageCursor("adjustmentDocuments", nextAdjustments.nextCursor);
    setPageCursor("stockBalances", nextBalances.nextCursor);
    setPageCursor("stockMovements", nextMovements.nextCursor);
    setStocktakes(nextStocktakes);
  }

  async function loadStockDocuments(
    query: StockDocumentQuery,
    cursor?: string,
    append = false,
  ) {
    const nextDocuments = await invoke<Page<StockDocument>>(
      "list_stock_documents_page",
      { cursor, query },
    );
    if (query.documentType === "inbound") {
      setInboundDocuments((current) => append ? [...current, ...nextDocuments.items] : nextDocuments.items);
      setPageCursor("inboundDocuments", nextDocuments.nextCursor);
    } else if (query.documentType === "outbound") {
      setOutboundDocuments((current) => append ? [...current, ...nextDocuments.items] : nextDocuments.items);
      setPageCursor("outboundDocuments", nextDocuments.nextCursor);
    } else if (query.documentType === "adjustment") {
      setAdjustmentDocuments((current) => append ? [...current, ...nextDocuments.items] : nextDocuments.items);
      setPageCursor("adjustmentDocuments", nextDocuments.nextCursor);
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
    const nextBalances = await invoke<Page<StockBalanceRow>>(
      "list_stock_balances_page",
      {
        query: normalizedQuery,
      },
    );
    setStockBalances(nextBalances.items);
    setPageCursor("stockBalances", nextBalances.nextCursor);
  }

  async function applyStockMovementQuery(query: StockMovementQuery) {
    const normalizedQuery: StockMovementQuery = {
      search: query.search?.trim() || null,
      itemId: query.itemId || null,
      direction: query.direction || null,
      movementType: query.movementType || null,
    };
    setStockMovementQuery(normalizedQuery);
    const nextMovements = await invoke<Page<StockMovementRow>>(
      "list_stock_movements_page",
      {
        query: normalizedQuery,
      },
    );
    setStockMovements(nextMovements.items);
    setPageCursor("stockMovements", nextMovements.nextCursor);
  }

  async function showItemMovements(itemId: string) {
    setActiveNav("movements");
    await applyStockMovementQuery({ itemId });
  }

  async function loadMore(key: DataPageKey) {
    const cursor = nextPageCursors[key];
    if (!cursor) return;
    if (key === "items") {
      const page = await invoke<Page<Item>>("list_items_page", {
        cursor,
        search: itemSearch,
        supplierId: itemSupplierId,
      });
      setItems((current) => [...current, ...page.items]);
      setPageCursor(key, page.nextCursor);
      return;
    }
    if (key === "stockBalances") {
      const page = await invoke<Page<StockBalanceRow>>("list_stock_balances_page", {
        cursor,
        query: stockBalanceQuery,
      });
      setStockBalances((current) => [...current, ...page.items]);
      setPageCursor(key, page.nextCursor);
      return;
    }
    if (key === "stockMovements") {
      const page = await invoke<Page<StockMovementRow>>("list_stock_movements_page", {
        cursor,
        query: stockMovementQuery,
      });
      setStockMovements((current) => [...current, ...page.items]);
      setPageCursor(key, page.nextCursor);
      return;
    }
    const query = key === "inboundDocuments"
      ? inboundDocumentQuery
      : key === "outboundDocuments"
        ? outboundDocumentQuery
        : adjustmentDocumentQuery;
    await loadStockDocuments(query, cursor, true);
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
    status?.runtime.clientPaired,
  ]);

  const refreshTarget = createTargetedRefresher(state, {
    loadApprovalRequests, loadAuditLogs, loadBackups, loadBudgetRules, loadHostRuntime,
    loadMasterData, loadReportsForStatus, loadStatus, loadStockData, loadSystemSettings, loadUsers,
  });

  async function refreshAll(search = itemSearch, supplierId = itemSupplierId) {
    try {
      setError(null);
      const user = await loadCurrentUser();
      const nextStatus = await invoke<AppStatus>("get_app_status");
      setStatus(nextStatus);
      const isUnpairedClient =
        nextStatus.runtime.mode === "client" && !nextStatus.runtime.clientPaired;
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
        canLoadBusinessData ? loadMasterData(search, supplierId) : Promise.resolve(),
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
    loadBudgetRules, loadHostRuntime, loadMasterData, loadMore, loadSupplierPurchaseRecords, loadUsers,
    refreshAll, refreshTarget, refreshTargetForEditor, scheduleRefresh, scheduleRefreshAll,
    showItemMovements,
  };
}
export type MainDataController = ReturnType<typeof useMainDataController>;
export type { RefreshTarget } from "./refreshTargets";
