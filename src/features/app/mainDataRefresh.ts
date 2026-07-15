import type { AppStatus } from "../../entities/runtime";
import { formatError } from "../../shared/lib/appRuntime";
import type { MainAppState } from "./useMainAppState";
import type { RefreshTarget } from "./refreshTargets";

type RefreshLoaders = {
  loadApprovalRequests: () => Promise<void>;
  loadAuditLogs: () => Promise<void>;
  loadBackups: () => Promise<void>;
  loadBudgetRules: () => Promise<void>;
  loadHostRuntime: () => Promise<void>;
  loadMasterData: () => Promise<void>;
  loadReportsForStatus: (status: AppStatus) => Promise<void>;
  loadStatus: () => Promise<AppStatus>;
  loadStockData: () => Promise<void>;
  loadSystemSettings: () => Promise<void>;
  loadUsers: () => Promise<void>;
};

export function createTargetedRefresher(state: MainAppState, loaders: RefreshLoaders) {
  return async (target: RefreshTarget) => {
    if (target === "none") return;
    try {
      state.setError(null);
      if (target === "connection") {
        await Promise.all([loaders.loadStatus(), loaders.loadHostRuntime()]);
        return;
      }
      if (target === "admin") {
        await Promise.all([
          loaders.loadStatus(), loaders.loadBackups(), loaders.loadUsers(),
          loaders.loadHostRuntime(), loaders.loadSystemSettings(), loaders.loadAuditLogs(),
          loaders.loadBudgetRules(), loaders.loadApprovalRequests(),
        ]);
        return;
      }
      const nextStatus = await loaders.loadStatus();
      if (target === "master") {
        await Promise.all([
          loaders.loadMasterData(),
          state.currentUser?.roles.some((role) => role.code === "admin")
            ? loaders.loadAuditLogs()
            : Promise.resolve(),
        ]);
        return;
      }
      await Promise.all([
        target === "business" ? loaders.loadMasterData() : Promise.resolve(),
        loaders.loadStockData(),
        state.currentUser?.permissions.includes("view_reports")
          ? loaders.loadReportsForStatus(nextStatus)
          : Promise.resolve(),
        state.currentUser?.roles.some((role) => role.code === "admin")
          ? Promise.all([loaders.loadAuditLogs(), loaders.loadApprovalRequests(), loaders.loadBackups()])
          : Promise.resolve(),
      ]);
    } catch (error) {
      state.setError(formatError(error));
    }
  };
}
