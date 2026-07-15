import { invoke } from "@tauri-apps/api/core";
import { open, type OpenDialogOptions } from "@tauri-apps/plugin-dialog";
import type { ClientConnectionInfo, HostServiceStatus, RuntimeConfig, RuntimeMode } from "../../entities/runtime";
import type { CurrentUser } from "../../entities/users";
import type { ImportPreview, ImportResult } from "../../entities/imports";
import { persistLoginCredential } from "../auth/credential-store";
import { checkAppUpdateWithFallback, formatError } from "../../shared/lib/appRuntime";
import type { I18n } from "../../i18n";
import type { MainAppState, BackupSummary } from "./useMainAppState";
import type { MainDataController, RefreshTarget } from "./useMainDataController";

async function chooseSinglePath(options: OpenDialogOptions) {
  const selected = await open({ ...options, multiple: false });
  if (typeof selected === "string") return selected;
  return Array.isArray(selected) ? selected[0] ?? null : null;
}

export function useMainActions(state: MainAppState, data: MainDataController, i18n: I18n) {
  const {
    appearanceSettings, itemSearch, itemSupplierId, reportQuery, setClientConnectionCheckedAt,
    setClientConnections, setCurrentUser, setError, setHostStatus, setHostTestResult,
    setImportPreview, setImportResult, setIsBackupWorking, setIsImporting,
    setIsLoginPending, setIsSavingMode, setLastBackup, setLastExportPath, setNotice,
    setPasswordChangeRequired, setUpdateState, status,
  } = state;
  const {
    clearSessionScopedState, loadHostRuntime, loadUsers, refreshAll, refreshTarget, scheduleRefreshAll,
  } = data;
  async function runAction(
    message: string,
    action: () => Promise<unknown>,
    target: RefreshTarget = "business",
  ) {
    try {
      setError(null);
      setNotice(null);
      await action();
      await refreshTarget(target);
      setNotice(message);
    } catch (err) {
      setError(formatError(err));
    }
  }

  async function changeMode(mode: RuntimeMode) {
    try {
      setIsSavingMode(true);
      setError(null);
      await invoke<RuntimeConfig>("set_runtime_mode", { mode });
      clearSessionScopedState();
      setHostStatus(null);
      setClientConnections([]);
      setHostTestResult(null);
      setClientConnectionCheckedAt(null);
      await refreshAll();
    } catch (err) {
      setError(formatError(err));
    } finally {
      setIsSavingMode(false);
    }
  }

  async function exportReport(query = reportQuery) {
    await runAction("月度报表已导出", async () => {
      const result = await invoke<{ path: string }>("export_monthly_report", {
        query,
      });
      setLastExportPath(result.path);
    }, "none");
  }

  async function exportItems(search = itemSearch, supplierId = itemSupplierId) {
    try {
      setError(null);
      setNotice(null);
      const result = await invoke<{ path: string }>("export_items", {
        search,
        supplierId,
      });
      setNotice(`物品档案已导出：${result.path}`);
    } catch (err) {
      setError(formatError(err));
    }
  }

  async function exportStockBalances() {
    try {
      setError(null);
      setNotice(null);
      const result = await invoke<{ path: string; rowCount: number }>(
        "export_stock_balances",
      );
      setNotice(`全部库存台账已导出（${result.rowCount} 项）：${result.path}`);
    } catch (err) {
      setError(formatError(err));
    }
  }

  async function previewImport(path: string) {
    try {
      setError(null);
      setNotice(null);
      setImportResult(null);
      setImportPreview(null);
      setIsImporting(true);
      const preview = await invoke<ImportPreview>("preview_excel_import", {
        request: { path },
      });
      setImportPreview(preview);
      setNotice("Excel 预览完成，尚未写入数据库");
    } catch (err) {
      setError(formatError(err));
    } finally {
      setIsImporting(false);
    }
  }

  async function exportImportTemplate() {
    try {
      setError(null);
      setNotice(null);
      setIsImporting(true);
      const result = await invoke<{ path: string }>("export_import_template");
      setNotice(`新版 Excel 导入模板已生成：${result.path}`);
    } catch (err) {
      setError(formatError(err));
    } finally {
      setIsImporting(false);
    }
  }

  async function runImport(path: string, mode: "full" | "itemsOnly") {
    try {
      setError(null);
      setNotice(null);
      setIsImporting(true);
      const result = await invoke<ImportResult>("run_excel_import", {
        request: { path, mode },
      });
      setImportResult(result);
      await refreshTarget("business");
      setNotice(
        mode === "itemsOnly"
          ? "Excel 物品档案已导入"
          : "Excel 已导入，单据、流水和库存余额已生成",
      );
    } catch (err) {
      setError(formatError(err));
    } finally {
      setIsImporting(false);
    }
  }

  async function importItemsFromToolbar() {
    const selected = await chooseSinglePath({
      title: "选择物品档案导入 Excel",
      filters: [{ name: "Excel 工作簿", extensions: ["xlsx"] }],
    });
    if (!selected) return;
    await runImport(selected, "itemsOnly");
  }

  async function createManualBackup() {
    try {
      setError(null);
      setNotice(null);
      setIsBackupWorking(true);
      const summary = await invoke<BackupSummary>("create_backup", {
        request: { backupType: "manual" },
      });
      setLastBackup(summary);
      await refreshTarget("admin");
      setNotice("手动备份已创建");
    } catch (err) {
      setError(formatError(err));
    } finally {
      setIsBackupWorking(false);
    }
  }

  async function checkForAppUpdate(options: { silent?: boolean } = {}) {
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
        appearanceSettings.locale === "zh-CN" ? "zh-CN" : "en-US",
      );
      if (!update) {
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
          setNotice(i18n.t("settings.updateNotAvailableNotice"));
        }
        return;
      }

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
        i18n.t("settings.updateAvailableNotice", {
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
          appearanceSettings.locale === "zh-CN" ? "zh-CN" : "en-US",
        ),
        sourceLabel: current.sourceLabel ?? null,
      }));
      if (!silent) {
        setError(message);
      }
    }
  }

  async function loginUser(
    username: string,
    password: string,
    rememberLogin: boolean,
  ) {
    try {
      setIsLoginPending(true);
      setError(null);
      setNotice(null);
      await new Promise<void>((resolve) => {
        window.requestAnimationFrame(() => resolve());
      });
      const user = await invoke<CurrentUser>("login", {
        request: { username, password },
      });
      const mustChangePassword = await invoke<boolean>(
        "get_password_change_required",
      );
      await persistLoginCredential(
        username,
        password,
        rememberLogin && !mustChangePassword,
      );
      setCurrentUser(user);
      setPasswordChangeRequired(mustChangePassword);
      setNotice(`已登录：${user.displayName}`);
      if (!mustChangePassword) scheduleRefreshAll();
    } catch (err) {
      setError(formatError(err));
    } finally {
      setIsLoginPending(false);
    }
  }

  async function logoutUser() {
    try {
      await invoke("logout");
      setCurrentUser(null);
      setPasswordChangeRequired(false);
      clearSessionScopedState();
      setNotice("已退出登录");
    } catch (err) {
      setError(formatError(err));
    }
  }

  async function toggleUserAccount(userId: string, enabled: boolean) {
    await runAction("用户状态已更新", async () => {
      await invoke("set_user_account_enabled", {
        request: { userId, enabled },
      });
      await loadUsers();
    }, "admin");
  }

  async function startHostRuntime() {
    await runAction("主机服务已启动", async () => {
      const status = await invoke<HostServiceStatus>("start_host_service");
      setHostStatus(status);
      await loadHostRuntime();
    }, "connection");
  }

  async function removeClientConnection(client: ClientConnectionInfo) {
    const confirmed = window.confirm(
      i18n.t("settings.removeClientConfirm", { name: client.clientName }),
    );
    if (!confirmed) return;
    await runAction("客户端设备已移除", async () => {
      await invoke("remove_client_connection", {
        request: { clientDeviceId: client.clientDeviceId },
      });
      await loadHostRuntime();
    }, "connection");
  }

  async function decideApprovalRequest(
    approvalId: string,
    approve: boolean,
    decisionNote: string,
  ) {
    await runAction(approve ? "审批已通过" : "审批已驳回", () =>
      invoke("decide_approval_request", {
        request: { approvalId, approve, decisionNote },
      }),
    "admin");
  }

  async function voidDocument(
    documentId: string,
    reason: string,
    handler: string,
  ) {
    await runAction("单据已作废，冲正流水已生成", () =>
      invoke("void_stock_document", {
        request: { documentId, reason, handler },
      }),
    "stock");
  }

  return {
    changeMode, checkForAppUpdate, createManualBackup, decideApprovalRequest,
    exportImportTemplate, exportItems, exportReport, exportStockBalances, importItemsFromToolbar,
    loginUser, logoutUser, previewImport, removeClientConnection, runAction,
    runImport, startHostRuntime, toggleUserAccount, voidDocument,
  };
}
export type MainActions = ReturnType<typeof useMainActions>;
