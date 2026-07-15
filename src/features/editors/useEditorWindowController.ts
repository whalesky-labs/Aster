import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { Update } from "@tauri-apps/plugin-updater";
import type { AppStatus, AppUpdateState, ClientConnectionInfo, HostServiceStatus, SystemSettings } from "../../entities/runtime";
import type { BudgetRule, Category, Department, Item, Supplier, Unit } from "../../entities/master-data";
import type { CurrentUser, Role, UserAccount } from "../../entities/users";
import type { StockBalanceRow, StockBatchRow, StockDocumentDetail, StocktakeDetail, StocktakeDocument } from "../../entities/stock";
import { createI18n } from "../../i18n";
import { checkAppUpdateWithFallback, formatError, notifyEditorSaved, type EditorSavedPayload } from "../../shared/lib/appRuntime";
import { closeCurrentEditorWindow, type EditorKind } from "../../shared/lib/editorWindows";
import { localMonth } from "../../shared/lib/localDate";
import { loadAppearanceSettings } from "../settings/appearance";

const initialUpdateState: AppUpdateState = {
  status: "idle", currentVersion: null, latestVersion: null, notes: null,
  downloadedBytes: 0, totalBytes: null, error: null, checkedAt: null, sourceLabel: null,
};
const defaultI18n = createI18n("zh-CN");
type BackupSummary = { backupFile: string };
const currentMonthString = localMonth;

export function useEditorWindowController({
  editor, id, params,
}: { editor: EditorKind; id?: string; params: URLSearchParams }) {
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [categories, setCategories] = useState<Category[]>([]);
  const [units, setUnits] = useState<Unit[]>([]);
  const [departments, setDepartments] = useState<Department[]>([]);
  const [suppliers, setSuppliers] = useState<Supplier[]>([]);
  const [items, setItems] = useState<Item[]>([]);
  const [currentUser, setCurrentUser] = useState<CurrentUser | null>(null);
  const [stockBalances, setStockBalances] = useState<StockBalanceRow[]>([]);
  const [stockBatches, setStockBatches] = useState<StockBatchRow[]>([]);
  const [stockDocumentDetail, setStockDocumentDetail] =
    useState<StockDocumentDetail | null>(null);
  const [users, setUsers] = useState<UserAccount[]>([]);
  const [roles, setRoles] = useState<Role[]>([]);
  const [budgetRules, setBudgetRules] = useState<BudgetRule[]>([]);
  const [stocktakes, setStocktakes] = useState<StocktakeDocument[]>([]);
  const [stocktakeDetail, setStocktakeDetail] =
    useState<StocktakeDetail | null>(null);
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [hostStatus, setHostStatus] = useState<HostServiceStatus | null>(null);
  const [clientConnections, setClientConnections] = useState<
    ClientConnectionInfo[]
  >([]);
  const [systemSettings, setSystemSettings] = useState<SystemSettings | null>(
    null,
  );
  const [updateState, setUpdateState] =
    useState<AppUpdateState>(initialUpdateState);
  const pendingUpdateRef = useRef<Update | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const periodMonth = params.get("periodMonth") ?? currentMonthString();

  async function loadEditorData() {
    setIsLoading(true);
    try {
      setError(null);
      const needsMaster = [
        "item",
        "category",
        "budget",
        "stockDocument",
        "adjustment",
        "stocktakeCreate",
        "stocktakeCounts",
        "user",
      ].includes(editor);
      if (needsMaster) {
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
          invoke<Item[]>("list_items"),
        ]);
        setCategories(nextCategories);
        setUnits(nextUnits);
        setDepartments(nextDepartments);
        setSuppliers(nextSuppliers);
        setItems(nextItems);
      }
      if (editor === "department") {
        setDepartments(await invoke<Department[]>("list_departments"));
      }
      if (editor === "unit") {
        setUnits(await invoke<Unit[]>("list_units"));
      }
      if (editor === "supplier") {
        setSuppliers(await invoke<Supplier[]>("list_suppliers"));
      }
      if (editor === "user") {
        const [nextUsers, nextRoles] = await Promise.all([
          invoke<UserAccount[]>("list_user_accounts"),
          invoke<Role[]>("list_roles"),
        ]);
        setUsers(nextUsers);
        setRoles(nextRoles);
      }
      if (editor === "budget") {
        setBudgetRules(
          await invoke<BudgetRule[]>("list_budget_rules", { periodMonth }),
        );
      }
      if (editor === "stockDocument") {
        const [nextBalances, nextUser] = await Promise.all([
          invoke<StockBalanceRow[]>("list_stock_balances", {
            query: {},
          }),
          invoke<CurrentUser | null>("get_current_user"),
        ]);
        setStockBalances(nextBalances);
        setCurrentUser(nextUser);
      }
      if (editor === "adjustment") {
        setCurrentUser(await invoke<CurrentUser | null>("get_current_user"));
      }
      if (editor === "stockDocumentDetail" && id) {
        setStockDocumentDetail(
          await invoke<StockDocumentDetail>("get_stock_document_detail", {
            documentId: id,
          }),
        );
      }
      if (editor === "stockBatchDetail" && id) {
        setStockBatches(
          await invoke<StockBatchRow[]>("list_stock_batches", {
            itemId: id,
          }),
        );
      }
      if (editor === "stocktakeDetail" && id) {
        setStocktakeDetail(
          await invoke<StocktakeDetail>("get_stocktake_detail", {
            stocktakeId: id,
          }),
        );
      }
      if (
        editor === "businessSettings" ||
        editor === "softwareUpdate" ||
        editor === "clientConnection" ||
        editor === "clientPairing" ||
        editor === "secondBackupDir" ||
        editor === "restoreBackup"
      ) {
        const [nextStatus, nextSettings] = await Promise.all([
          invoke<AppStatus>("get_app_status"),
          invoke<SystemSettings>("get_system_settings"),
        ]);
        setStatus(nextStatus);
        setSystemSettings(nextSettings);
      }
      if (editor === "connectionWizard") {
        const [nextStatus, nextHostStatus] = await Promise.all([
          invoke<AppStatus>("get_app_status"),
          invoke<HostServiceStatus>("get_host_service_status"),
        ]);
        setStatus(nextStatus);
        setHostStatus(nextHostStatus);
        if (
          nextStatus.runtime.mode === "host" &&
          params.get("clientOnly") !== "1"
        ) {
          setClientConnections(
            await invoke<ClientConnectionInfo[]>("list_client_connections"),
          );
        } else {
          setClientConnections([]);
        }
      }
      if (editor === "stocktakeCounts") {
        const nextStocktakes =
          await invoke<StocktakeDocument[]>("list_stocktakes");
        setStocktakes(nextStocktakes);
        const stocktakeId = id ?? nextStocktakes[0]?.id;
        if (stocktakeId) {
          setStocktakeDetail(
            await invoke<StocktakeDetail>("get_stocktake_detail", {
              stocktakeId,
            }),
          );
        }
      }
    } catch (err) {
      setError(formatError(err));
    } finally {
      setIsLoading(false);
    }
  }

  async function runEditorAction(
    payload: EditorSavedPayload,
    action: () => Promise<unknown>,
  ) {
    try {
      setIsSaving(true);
      setError(null);
      setNotice(null);
      await action();
      await notifyEditorSaved(payload);
      await closeCurrentEditorWindow();
    } catch (err) {
      setError(formatError(err));
    } finally {
      setIsSaving(false);
    }
  }

  async function runSettingsEditorAction(
    message: string,
    action: () => Promise<unknown>,
  ) {
    await runEditorAction({ editor, message }, action);
  }

  async function createEditorBackupBeforeUpdate() {
    if (status?.runtime.mode === "client") {
      return null;
    }
    return invoke<BackupSummary>("create_backup", {
      request: { backupType: "manual" },
    });
  }

  async function checkEditorUpdate(options: { silent?: boolean } = {}) {
    const silent = options.silent ?? false;
    try {
      if (!silent) {
        setError(null);
        setNotice(null);
      }
      setUpdateState((current) => ({
        ...current,
        status: "checking",
        error: null,
      }));
      const { attemptLabel, update } = await checkAppUpdateWithFallback();
      const checkedAt = new Date().toLocaleString(
        loadAppearanceSettings().locale === "zh-CN" ? "zh-CN" : "en-US",
      );
      if (!update) {
        pendingUpdateRef.current = null;
        setUpdateState((current) => ({
          ...current,
          status: "notAvailable",
          currentVersion: status?.appVersion ?? current.currentVersion ?? null,
          latestVersion: null,
          notes: null,
          downloadedBytes: 0,
          totalBytes: null,
          error: null,
          checkedAt,
          sourceLabel: attemptLabel,
        }));
        if (!silent) {
          setNotice(defaultI18n.t("settings.updateNotAvailableNotice"));
        }
        return;
      }

      pendingUpdateRef.current = update;
      setUpdateState((current) => ({
        ...current,
        status: "available",
        currentVersion:
          update.currentVersion ?? status?.appVersion ?? current.currentVersion,
        latestVersion: update.version,
        notes: update.body ?? null,
        downloadedBytes: 0,
        totalBytes: null,
        error: null,
        checkedAt,
        sourceLabel: attemptLabel,
      }));
      setNotice(
        defaultI18n.t("settings.updateAvailableNotice", {
          version: update.version,
        }) + `（${attemptLabel}）`,
      );
    } catch (err) {
      const message = formatError(err);
      setUpdateState((current) => ({
        ...current,
        status: "error",
        error: message,
        checkedAt: new Date().toLocaleString(
          loadAppearanceSettings().locale === "zh-CN" ? "zh-CN" : "en-US",
        ),
        sourceLabel: current.sourceLabel ?? null,
      }));
      if (!silent) {
        setError(message);
      }
    }
  }

  async function downloadAndInstallEditorUpdate() {
    const update = pendingUpdateRef.current;
    if (!update) {
      await checkEditorUpdate();
      return;
    }
    try {
      setIsSaving(true);
      setError(null);
      setNotice(null);
      setUpdateState((current) => ({
        ...current,
        status: "downloading",
        downloadedBytes: 0,
        totalBytes: null,
        error: null,
      }));
      await createEditorBackupBeforeUpdate();
      await invoke("prepare_update_settings_snapshot");
      let downloadedBytes = 0;
      let totalBytes: number | null = null;
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          totalBytes = event.data.contentLength ?? null;
          downloadedBytes = 0;
        } else if (event.event === "Progress") {
          downloadedBytes += event.data.chunkLength;
        } else if (event.event === "Finished" && totalBytes != null) {
          downloadedBytes = totalBytes;
        }
        setUpdateState((current) => ({
          ...current,
          status: "downloading",
          downloadedBytes,
          totalBytes,
        }));
      });
      setUpdateState((current) => ({
        ...current,
        status: "installed",
        downloadedBytes:
          current.totalBytes != null ? current.totalBytes : current.downloadedBytes,
        error: null,
      }));
      setNotice(defaultI18n.t("settings.updateInstalledNotice"));
    } catch (err) {
      const message = formatError(err);
      setUpdateState((current) => ({
        ...current,
        status: "error",
        error: message,
      }));
      setError(message);
    } finally {
      setIsSaving(false);
    }
  }

  useEffect(() => {
    document.body.classList.add("editor-window");
    return () => document.body.classList.remove("editor-window");
  }, []);

  useEffect(() => {
    const currentWindow = getCurrentWindow();
    const syncNativeTheme = () => {
      const theme = document.documentElement.dataset.theme === "dark" ? "dark" : "light";
      void currentWindow.setTheme(theme);
    };
    syncNativeTheme();
    const observer = new MutationObserver(syncNativeTheme);
    observer.observe(document.documentElement, {
      attributeFilter: ["data-theme"],
      attributes: true,
    });
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    void loadEditorData();
  }, []);

  useEffect(() => {
    if (editor !== "softwareUpdate" || isLoading) return;
    void checkEditorUpdate({ silent: true });
  }, [editor, isLoading]);

  const enabledCategories = categories.filter((item) => item.enabled);
  const enabledUnits = units.filter((item) => item.enabled);
  const enabledSuppliers = suppliers.filter((item) => item.enabled);
  const enabledDepartments = departments.filter((item) => item.enabled);
  const enabledItems = items.filter((item) => item.enabled);
  return {
    budgetRules, categories, clientConnections, currentUser, departments, enabledCategories,
    enabledDepartments, enabledItems, enabledSuppliers, enabledUnits, error, hostStatus, isLoading,
    isSaving, items, notice, periodMonth, roles, runEditorAction, runSettingsEditorAction,
    setClientConnections, setError, setHostStatus, setIsSaving, setNotice, setStatus,
    setStocktakeDetail, status, stockBalances, stockBatches, stockDocumentDetail,
    stocktakeDetail, stocktakes, suppliers, systemSettings, units, updateState, users,
    checkEditorUpdate, downloadAndInstallEditorUpdate,
  };
}
export type EditorWindowController = ReturnType<typeof useEditorWindowController>;
