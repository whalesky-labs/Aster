import { invoke } from "@tauri-apps/api/core";

import type { ApprovalRequest } from "../../entities/approvals";
import type { BudgetRule, Supplier, SupplierPurchaseRecord } from "../../entities/master-data";
import type { AuditLogRow, BackupRecord } from "../../entities/operations";
import type {
  AppStatus,
  ClientConnectionInfo,
  HostConnectionTestResult,
  HostServiceStatus,
  RuntimeConfig,
  SystemSettings,
} from "../../entities/runtime";
import type { UserAccount } from "../../entities/users";
import { formatError } from "../../shared/lib/appRuntime";
import type { MainAppState } from "./useMainAppState";

export function createMainDataAuxLoaders(
  state: MainAppState,
  isBusinessConnectionReady: boolean,
) {
  async function loadBackups() {
    state.setBackupRecords(await invoke<BackupRecord[]>("list_backup_records"));
  }

  async function loadAuditLogs() {
    state.setAuditLogs(await invoke<AuditLogRow[]>("list_audit_logs", { limit: 120 }));
  }

  async function loadHostRuntime() {
    const [status, clients] = await Promise.all([
      invoke<HostServiceStatus>("get_host_service_status"),
      invoke<ClientConnectionInfo[]>("list_client_connections"),
    ]);
    state.setHostStatus(status);
    state.setClientConnections(clients);
  }

  async function probeConfiguredHost(runtime: RuntimeConfig) {
    if (runtime.mode !== "client" || !runtime.hostAddress) {
      state.setClientConnectionCheckedAt(null);
      state.setHostTestResult(null);
      return true;
    }
    try {
      const result = await invoke<HostConnectionTestResult>("test_host_connection", {
        request: { hostAddress: runtime.hostAddress, hostPort: runtime.hostPort },
      });
      state.setHostTestResult(result);
      return result.ok;
    } catch (error) {
      state.setHostTestResult({
        ok: false, message: `主机连接异常：${String(error)}`,
        appName: null, appVersion: null, schemaVersion: null,
      });
      return false;
    } finally {
      state.setClientConnectionCheckedAt(new Date().toLocaleString("zh-CN"));
    }
  }

  async function loadSystemSettings() {
    state.setSystemSettings(await invoke<SystemSettings>("get_system_settings"));
  }

  async function loadUsers(user = state.currentUser) {
    if (!user?.roles.some((role) => role.code === "admin")) return;
    state.setUserAccounts(await invoke<UserAccount[]>("list_user_accounts"));
  }

  async function loadBudgetRules(month = state.reportMonth) {
    state.setBudgetRules(await invoke<BudgetRule[]>("list_budget_rules", { periodMonth: month }));
  }

  async function loadApprovalRequests() {
    state.setApprovalRequests(await invoke<ApprovalRequest[]>("list_approval_requests"));
  }

  async function loadSupplierPurchaseRecords(supplier: Supplier) {
    if (!isBusinessConnectionReady) return;
    try {
      state.setError(null);
      state.setActiveSupplier(supplier);
      state.setSupplierPurchaseRecords(
        await invoke<SupplierPurchaseRecord[]>("list_supplier_purchase_records", {
          supplierId: supplier.id,
        }),
      );
    } catch (error) {
      state.setError(formatError(error));
    }
  }

  async function loadStatus() {
    const status = await invoke<AppStatus>("get_app_status");
    state.setStatus(status);
    return status;
  }

  return {
    loadApprovalRequests, loadAuditLogs, loadBackups, loadBudgetRules, loadHostRuntime,
    loadStatus, loadSupplierPurchaseRecords, loadSystemSettings, loadUsers, probeConfiguredHost,
  };
}
