import {
  useEffect,
  useMemo,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { createI18n } from "./i18n";
import {
  loadRememberedUsername,
  migrateLegacyLoginStorage,
  persistLoginCredential,
} from "./features/auth/credential-store";
import { ForcedPasswordChange } from "./features/auth/ForcedPasswordChange";
import { LoginScreen } from "./features/auth/LoginScreen";
import type { NavKey } from "./entities/navigation";
import { navItems } from "./features/navigation/navigation";
import {
  useBroadcastAppearanceSettings,
  useSyncedAppearanceSettings,
} from "./features/settings/appearance";
import { useEditorWindowController } from "./features/editors/useEditorWindowController";
import { renderMasterEditorContent } from "./features/editors/MasterEditorContent";
import { renderSettingsEditorContent } from "./features/editors/SettingsEditorContent";
import { renderStockEditorContent } from "./features/editors/StockEditorContent";
import { useMainAppState } from "./features/app/useMainAppState";
import { useMainDataController } from "./features/app/useMainDataController";
import { useMainActions } from "./features/app/useMainActions";
import { MainContent } from "./features/app/MainContent";
import { MainShell } from "./features/app/MainShell";
import {
  EDITOR_WINDOW_ERROR_EVENT,
  openEditorWindow,
  type EditorKind,
  type EditorMode,
} from "./shared/lib/editorWindows";
import { formatError, type EditorSavedPayload } from "./shared/lib/appRuntime";
import type {
  AppStatus,
  HostConnectionTestResult,
  HostServiceStatus,
  RuntimeMode,
} from "./entities/runtime";
import type { CurrentUser } from "./entities/users";
import "./App.css";

const defaultI18n = createI18n("zh-CN");

function detectDesktopPlatform() {
  const internals = window as Window & { __TAURI_OS_PLUGIN_INTERNALS__?: { platform?: string } };
  const tauriPlatform = internals.__TAURI_OS_PLUGIN_INTERNALS__?.platform;
  if (tauriPlatform) return tauriPlatform.toLowerCase();
  if (navigator.userAgent.includes("Windows")) return "windows";
  if (navigator.userAgent.includes("Mac")) return "macos";
  return "unknown";
}

function formatMoney(value: number) {
  return defaultI18n.formatMoney(value);
}

function formatDateTime(value?: string | null) {
  const rawValue = String(value ?? "").trim();
  if (!rawValue) return "-";
  const normalized = rawValue.replace("T", " ").replace(/Z$/, "");
  const [datePart, timePart] = normalized.split(/\s+/, 2);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(datePart)) {
    return rawValue;
  }
  if (!timePart) {
    return datePart;
  }
  const cleanTime = timePart.split(/[.+-]/)[0];
  const [hour = "00", minute = "00", second = "00"] = cleanTime.split(":");
  return `${datePart} ${hour.padStart(2, "0")}:${minute.padStart(2, "0")}:${second.padStart(2, "0")}`;
}


function modeLabel(mode: RuntimeMode, i18n = defaultI18n) {
  return i18n.modeLabel(mode);
}

function connectionStatusLabel(
  status: AppStatus | null,
  hostTestResult: HostConnectionTestResult | null,
  i18n = defaultI18n,
) {
  if (!status) return i18n.t("connection.loading");
  if (status.runtime.mode === "host") return i18n.t("connection.host");
  if (status.runtime.mode === "client") {
    if (!status.runtime.clientPaired) return i18n.t("connection.unpaired");
    return hostTestResult?.ok === false
      ? i18n.t("connection.abnormal")
      : i18n.t("connection.connected");
  }
  return i18n.t("connection.standalone");
}

function connectionStatusHint(
  status: AppStatus | null,
  hostStatus: HostServiceStatus | null,
  hostTestResult: HostConnectionTestResult | null,
  i18n = defaultI18n,
) {
  if (!status) return i18n.t("connection.hint.loading");
  if (status.runtime.mode === "host") {
    return hostStatus?.running
      ? i18n.t("connection.hint.hostRunning")
      : i18n.t("connection.hint.hostStopped");
  }
  if (status.runtime.mode === "client") {
    if (!status.runtime.clientPaired) {
      return i18n.t("connection.hint.clientUnpaired");
    }
    return hostTestResult?.ok === false
      ? i18n.t("connection.hint.clientAbnormal")
      : i18n.t("connection.hint.clientConnected");
  }
  return i18n.t("connection.hint.standalone");
}

function connectionStatusKind(
  status: AppStatus | null,
  hostStatus: HostServiceStatus | null,
  hostTestResult: HostConnectionTestResult | null,
) {
  if (!status) return "idle";
  if (status.runtime.mode === "host") {
    return hostStatus?.running ? "success" : "warning";
  }
  if (status.runtime.mode === "client") {
    if (!status.runtime.clientPaired || hostTestResult?.ok === false) {
      return "warning";
    }
    return "success";
  }
  return "success";
}

function movementTypeLabel(type: string, i18n = defaultI18n) {
  return i18n.movementTypeLabel(type);
}

function hasPermission(user: CurrentUser | null, permission: string) {
  return user?.permissions.includes(permission) ?? false;
}

function isAdminUser(user: CurrentUser | null) {
  return user?.roles.some((role) => role.code === "admin") ?? false;
}

function canAccessNav(key: NavKey, user: CurrentUser | null) {
  if (!user) return key === "dashboard";
  if (key === "reports") return hasPermission(user, "view_reports");
  if (key === "import") return hasPermission(user, "write_stock");
  if (
    key === "settings" ||
    key === "users" ||
    key === "budgets" ||
    key === "approvals" ||
    key === "backups" ||
    key === "logs"
  )
    return isAdminUser(user);
  return true;
}

function firstAccessibleNav(user: CurrentUser | null) {
  return (
    navItems.find((item) => canAccessNav(item.key, user))?.key ?? "dashboard"
  );
}

function App() {
  migrateLegacyLoginStorage();
  useSyncedAppearanceSettings();
  const editorParams = new URLSearchParams(window.location.search);
  const editorKind = editorParams.get("editor") as EditorKind | null;
  if (editorKind) {
    return (
      <EditorWindowApp
        documentType={
          (editorParams.get("documentType") as "inbound" | "outbound" | null) ??
          undefined
        }
        editor={editorKind}
        id={editorParams.get("id") ?? undefined}
        mode={(editorParams.get("mode") as EditorMode | null) ?? "create"}
        params={editorParams}
      />
    );
  }
  return <MainApp />;
}

function MainApp() {
  const desktopPlatform = useMemo(() => detectDesktopPlatform(), []);
  const state = useMainAppState();
  const {
    activeNav, appearanceSettings, categories, currentUser, error,
    hasCheckedUpdateOnStartupRef, hostStatus, hostTestResult, isLoginPending,
    notice, passwordChangeRequired, setActiveNav, setClientConnectionCheckedAt,
    setError, setHostTestResult, setIsLoginPending, setNotice,
    setPasswordChangeRequired, status, suppliers, units,
  } = state;
  const i18n = useMemo(
    () => createI18n(appearanceSettings.locale),
    [appearanceSettings.locale],
  );
  useSyncedAppearanceSettings(appearanceSettings);
  useBroadcastAppearanceSettings(appearanceSettings);
  useEffect(() => {
    const handleEditorWindowError = (event: Event) => {
      setError((event as CustomEvent<string>).detail);
    };
    window.addEventListener(EDITOR_WINDOW_ERROR_EVENT, handleEditorWindowError);
    return () => window.removeEventListener(EDITOR_WINDOW_ERROR_EVENT, handleEditorWindowError);
  }, [setError]);
  const data = useMainDataController(state);
  const {
    bootstrapSession, clearSessionScopedState, refreshAll, refreshTargetForEditor,
    scheduleRefresh, scheduleRefreshAll,
  } = data;
  const actions = useMainActions(state, data, i18n);
  const {
    checkForAppUpdate, loginUser, logoutUser,
  } = actions;
  useEffect(() => {
    void bootstrapSession();
  }, []);

  useEffect(() => {
    if (!status?.appVersion || hasCheckedUpdateOnStartupRef.current) {
      return;
    }
    hasCheckedUpdateOnStartupRef.current = true;
    void checkForAppUpdate({ silent: true });
  }, [status?.appVersion]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<EditorSavedPayload>("editor:saved", (event) => {
      if (event.payload.editor === "connectionWizard") {
        clearSessionScopedState();
        setHostTestResult(null);
        setClientConnectionCheckedAt(null);
        scheduleRefreshAll();
      } else {
        scheduleRefresh(refreshTargetForEditor(event.payload.editor));
      }
      setNotice(event.payload.message ?? "已保存");
    }).then((nextUnlisten) => {
      unlisten = nextUnlisten;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  const metrics = status?.metrics;
  const metricCards = useMemo(
    () => [
      {
        label: i18n.t("dashboard.metric.items"),
        value: metrics?.itemCount ?? 0,
        suffix: i18n.t("dashboard.unit.itemTypes"),
      },
      {
        label: i18n.t("dashboard.metric.departments"),
        value: metrics?.departmentCount ?? 0,
        suffix: i18n.t("dashboard.unit.departments"),
      },
      {
        label: i18n.t("dashboard.metric.suppliers"),
        value: metrics?.supplierCount ?? 0,
        suffix: i18n.t("dashboard.unit.suppliers"),
      },
      {
        label: i18n.t("dashboard.metric.currentStockAmount"),
        value: i18n.formatMoney(metrics?.currentStockAmount ?? 0),
        suffix: i18n.t("dashboard.unit.currency"),
      },
      {
        label: i18n.t("dashboard.metric.thisMonthInboundAmount"),
        value: i18n.formatMoney(metrics?.thisMonthInboundAmount ?? 0),
        suffix: i18n.t("dashboard.unit.currency"),
      },
      {
        label: i18n.t("dashboard.metric.thisMonthOutboundAmount"),
        value: i18n.formatMoney(metrics?.thisMonthOutboundAmount ?? 0),
        suffix: i18n.t("dashboard.unit.currency"),
      },
      {
        label: i18n.t("dashboard.metric.lowStock"),
        value: metrics?.lowStockCount ?? 0,
        suffix: i18n.t("dashboard.unit.items"),
      },
      {
        label: i18n.t("dashboard.metric.negativeStock"),
        value: metrics?.negativeStockCount ?? 0,
        suffix: i18n.t("dashboard.unit.items"),
      },
    ],
    [i18n, metrics],
  );

  const enabledCategories = categories.filter((item) => item.enabled);
  const enabledUnits = units.filter((item) => item.enabled);
  const enabledSuppliers = suppliers.filter((item) => item.enabled);
  const isClientMode = status?.runtime.mode === "client";
  const isClientPaired = Boolean(status?.runtime.clientPaired);
  const isBusinessConnectionReady =
    !isClientMode || (isClientPaired && hostTestResult?.ok === true);
  const canWriteStock =
    hasPermission(currentUser, "write_stock") && isBusinessConnectionReady;
  const canViewReports =
    hasPermission(currentUser, "view_reports") && isBusinessConnectionReady;
  const canManageSettings = isAdminUser(currentUser);
  const canManageRemoteBusiness =
    canManageSettings && isBusinessConnectionReady;
  const canUseLocalImport = !isClientMode;
  const visibleNavItems = navItems.filter((item) =>
    canAccessNav(item.key, currentUser),
  );
  const settingsNavItem = visibleNavItems.find(
    (item) => item.key === "settings",
  );
  const sidebarConnectionKind = connectionStatusKind(
    status,
    hostStatus,
    hostTestResult,
  );
  const clientPauseMessage =
    isClientMode && currentUser && !isBusinessConnectionReady
      ? i18n.t("app.clientPaused", {
          reason: isClientPaired
            ? i18n.t("app.clientPaused.disconnected")
            : i18n.t("app.clientPaused.unpaired"),
        })
      : null;
  const footerStatus = error
    ? { kind: "error", text: error }
    : clientPauseMessage
      ? { kind: "warning", text: clientPauseMessage }
      : notice
        ? { kind: "notice", text: notice }
        : {
            kind: "idle",
            text: status?.health.message ?? i18n.t("app.ready"),
          };

  useEffect(() => {
    if (!canAccessNav(activeNav, currentUser)) {
      setActiveNav(firstAccessibleNav(currentUser));
    }
  }, [activeNav, currentUser]);

  if (!currentUser) {
    return (
      <LoginScreen
        error={error}
        i18n={i18n}
        isLoginPending={isLoginPending}
        notice={notice}
        onOpenConnectionWizard={() =>
          void openEditorWindow("connectionWizard", {
            extra: { clientOnly: "1" },
            width: 760,
            height: 640,
          })
        }
        onOpenPasswordReset={() =>
          void openEditorWindow("passwordReset", {
            width: 520,
            height: 460,
          })
        }
        onLogin={loginUser}
      />
    );
  }

  if (passwordChangeRequired) {
    return (
      <ForcedPasswordChange
        error={error}
        isPending={isLoginPending}
        onChange={async (oldPassword, newPassword) => {
          try {
            setIsLoginPending(true);
            setError(null);
            await invoke("change_password", {
              request: { newPassword, oldPassword },
            });
            await persistLoginCredential(
              currentUser.username,
              newPassword,
              Boolean(loadRememberedUsername()),
            );
            setPasswordChangeRequired(false);
            setNotice("默认密码已修改");
            scheduleRefreshAll();
          } catch (err) {
            setError(formatError(err));
          } finally {
            setIsLoginPending(false);
          }
        }}
      />
    );
  }

  return (
    <MainShell
      activeNav={activeNav}
      connectionHint={connectionStatusHint(status, hostStatus, hostTestResult, i18n)}
      connectionKind={sidebarConnectionKind}
      connectionLabel={connectionStatusLabel(status, hostTestResult, i18n)}
      desktopPlatform={desktopPlatform}
      footerStatus={footerStatus}
      i18n={i18n}
      logout={logoutUser}
      refresh={refreshAll}
      setActiveNav={setActiveNav}
      settingsNavItem={settingsNavItem}
      status={status}
      visibleNavItems={visibleNavItems}
    >
      <MainContent
        actions={actions}
        data={data}
        i18n={i18n}
        state={state}
        view={{
          canManageRemoteBusiness, canManageSettings, canUseLocalImport, canViewReports,
          canWriteStock, enabledCategories, enabledSuppliers, enabledUnits, formatDateTime,
          formatMoney, metricCards, modeLabel: (mode) => modeLabel(mode, i18n),
          movementTypeLabel: (type) => movementTypeLabel(type, i18n),
        }}
      />
    </MainShell>
  );
}

function EditorWindowApp({
  documentType,
  editor,
  id,
  mode,
  params,
}: {
  documentType?: "inbound" | "outbound";
  editor: EditorKind;
  id?: string;
  mode: EditorMode;
  params: URLSearchParams;
}) {
  const controller = useEditorWindowController({ editor, id, params });
  const content =
    renderMasterEditorContent({ controller, editor, id, mode }) ??
    renderSettingsEditorContent({ controller, editor, params }) ??
    renderStockEditorContent({ controller, documentType, editor });
  return (
    <main className={params.get("titlebar") === "overlay" ? "editor-shell overlay-titlebar" : "editor-shell"}>
      <div className="editor-messages">
        {controller.error ? <div className="error-banner">{controller.error}</div> : null}
        {controller.notice ? <div className="notice-banner">{controller.notice}</div> : null}
      </div>
      <section className="editor-body">
        {content ?? <div className="placeholder-panel"><h2>暂不支持的编辑窗口</h2></div>}
      </section>
    </main>
  );
}

export default App;
