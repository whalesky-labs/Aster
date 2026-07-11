import {
  type KeyboardEvent,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit, emitTo, listen } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { open, type OpenDialogOptions } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type CheckOptions, type Update } from "@tauri-apps/plugin-updater";
import { createI18n, type I18n, type LocaleCode } from "./i18n";
import {
  loadRememberedUsername,
  migrateLegacyLoginStorage,
  persistLoginCredential,
} from "./features/auth/credential-store";
import { ForcedPasswordChange } from "./features/auth/ForcedPasswordChange";
import { LoginScreen, PantsLogo } from "./features/auth/LoginScreen";
import { BackupRecordsPage } from "./features/backups/BackupRecordsPage";
import { LogsPage } from "./features/audit/LogsPage";
import { UsersPage } from "./features/users/UsersPage";
import { ItemsPage } from "./features/master-data/ItemsPage";
import { CategoriesPage, DepartmentsPage, UnitsPage } from "./features/master-data/MasterDataPages";
import { SuppliersPage } from "./features/master-data/SuppliersPage";
import { BudgetRulesPage } from "./features/master-data/BudgetRulesPage";
import { ApprovalsPage } from "./features/approvals/ApprovalsPage";
import type { ApprovalRequest } from "./entities/approvals";
import type { NavKey } from "./entities/navigation";
import { Dashboard } from "./features/dashboard/Dashboard";
import { ImportPage } from "./features/imports/ImportPage";
import { ReportsPage } from "./features/reports/ReportsPage";
import { SoftwareUpdateWindow } from "./features/settings/SoftwareUpdateWindow";
import { SettingsPage } from "./features/settings/SettingsPage";
import { StockBalancePage } from "./features/stock/StockBalancePage";
import { StockMovementPage } from "./features/stock/StockMovementPage";
import { ConnectionWizard } from "./features/connections/ConnectionWizard";
import { ItemSearchSelect } from "./shared/ui/ItemSearchSelect";
import type { AuditLogRow, BackupRecord } from "./entities/operations";
import type {
  AppStatus,
  AppUpdateState,
  AppearanceSettings,
  ClientConnectionInfo,
  HostConnectionTestResult,
  HostDiscoveryResult,
  HostServiceStatus,
  LiquidGlassStyle,
  ProxyCandidate,
  RuntimeConfig,
  RuntimeMode,
  SystemSettings,
  ThemeMode,
} from "./entities/runtime";
import type { CurrentUser, Role, UserAccount } from "./entities/users";
import type {
  BudgetRule,
  Category,
  Department,
  Item,
  OptionRecord,
  Supplier,
  SupplierPurchaseRecord,
  Unit,
} from "./entities/master-data";
import type {
  StockBalanceQuery,
  StockBalanceRow,
  StockBatchRow,
  StockDocument,
  StockDocumentDetail,
  StockDocumentQuery,
  StockMovementQuery,
  StockMovementRow,
  StocktakeDetail,
  StocktakeDocument,
} from "./entities/stock";
import type { ReportBundle, ReportQuery } from "./entities/reports";
import type { ImportPreview, ImportResult } from "./entities/imports";
import {
  EmptyRow,
  Field,
  MasterTablePanel,
  MonthSelect,
  PaginatedTable,
  PathPickerField,
} from "./shared/ui/DataTable";
import "./App.css";

function submitOnEnter(
  event: KeyboardEvent<HTMLDivElement>,
  onSubmit: () => void,
) {
  if (event.key !== "Enter") return;
  const target = event.target as HTMLElement;
  if (target.tagName === "TEXTAREA") return;
  event.preventDefault();
  onSubmit();
}

const initialUpdateState: AppUpdateState = {
  status: "idle",
  currentVersion: null,
  latestVersion: null,
  notes: null,
  downloadedBytes: 0,
  totalBytes: null,
  error: null,
  checkedAt: null,
  sourceLabel: null,
};

type BackupSummary = {
  backupFile: string;
  backupType: string;
  createdAt: string;
  schemaVersion: number;
  sourceHostName: string;
  sourceOs: string;
  databaseSize: number;
  databaseSha256: string;
  secondBackupFile?: string | null;
};

type BackupMetadata = {
  appName: string;
  appVersion: string;
  schemaVersion: number;
  createdAt: string;
  backupType: string;
  databaseFile: string;
  databaseSize: number;
  databaseSha256: string;
  sourceOs: string;
  sourceHostName?: string | null;
};

type RestorePreview = {
  backupFile: string;
  metadata: BackupMetadata;
  valid: boolean;
  message: string;
  validationToken: string;
};

type StockDocumentDraft = {
  documentId?: string;
  documentType: "inbound" | "outbound";
  outboundKind?: "internal" | "guest_sale";
  businessDate: string;
  departmentId: string;
  supplierId: string;
  handler: string;
  purpose: string;
  remark: string;
  approvalRequestId: string;
  lines: StockDocumentLineDraft[];
};

type StockDocumentLineDraft = {
  itemId: string;
  quantity: number;
  unitPrice: number;
  amount?: number | null;
  purchaseUnitPrice?: number | null;
  purchaseAmount?: number | null;
  saleUnitPrice?: number | null;
  saleAmount?: number | null;
  costUnitPrice?: number | null;
  costAmount?: number | null;
  remark: string;
};

type AdjustmentDraft = {
  businessDate: string;
  adjustmentType: "gain" | "loss" | "damage" | "correction";
  handler: string;
  reason: string;
  lines: AdjustmentLineDraft[];
};

type AdjustmentLineDraft = {
  itemId: string;
  direction: "in" | "out";
  quantity: number;
  unitPrice: number;
  amount?: number | null;
  remark: string;
};

type DepartmentDraft = {
  id?: string;
  expectedUpdatedAt?: string;
  code: string;
  name: string;
  manager: string;
  enabled: boolean;
  sortOrder: number;
  remark: string;
};

type SimpleNameDraft = {
  id?: string;
  expectedUpdatedAt?: string;
  name: string;
  enabled: boolean;
  sortOrder: number;
};

type CategoryDraft = {
  id?: string;
  expectedUpdatedAt?: string;
  parentId: string;
  name: string;
  enabled: boolean;
  sortOrder: number;
};

type SupplierDraft = {
  id?: string;
  expectedUpdatedAt?: string;
  name: string;
  contact: string;
  phone: string;
  address: string;
  enabled: boolean;
  remark: string;
};

type BudgetRuleDraft = {
  id?: string;
  expectedUpdatedAt?: string;
  departmentId: string;
  categoryId?: string | null;
  periodMonth: string;
  amountLimit: number;
  enabled: boolean;
};

type CreateApprovalRequestDraft = {
  entityType: string;
  entityId: string;
  reason: string;
};

type EditorKind =
  | "item"
  | "department"
  | "category"
  | "unit"
  | "supplier"
  | "budget"
  | "user"
  | "changePassword"
  | "passwordReset"
  | "businessSettings"
  | "softwareUpdate"
  | "clientConnection"
  | "clientPairing"
  | "connectionWizard"
  | "secondBackupDir"
  | "restoreBackup"
  | "stockDocument"
  | "stockDocumentDetail"
  | "stockBatchDetail"
  | "adjustment"
  | "stocktakeCreate"
  | "stocktakeDetail"
  | "stocktakeCounts";

type EditorMode = "create" | "edit";

type EditorSavedPayload = {
  editor: EditorKind;
  message?: string;
  documentType?: "inbound" | "outbound";
  stocktakeId?: string;
};

type NavItem = {
  key: NavKey;
  labelKey: string;
};

const navIconProps = {
  "aria-hidden": true,
  className: "nav-item-icon",
  fill: "none",
  focusable: "false",
  stroke: "currentColor",
  strokeLinecap: "round",
  strokeLinejoin: "round",
  strokeWidth: 1.8,
  viewBox: "0 0 24 24",
} as const;

const navIconContent: Record<NavKey, React.ReactNode> = {
  dashboard: (
    <>
      <path d="M4 11.4 12 4l8 7.4" />
      <path d="M6.5 10.5V20h11v-9.5" />
      <path d="M10 20v-5h4v5" />
    </>
  ),
  items: (
    <>
      <path d="M6 7.5 12 4l6 3.5v7L12 18l-6-3.5z" />
      <path d="m6.3 8 5.7 3.3L17.7 8" />
      <path d="M12 11.3V18" />
    </>
  ),
  inbound: (
    <>
      <path d="M5 19h14V9H5z" />
      <path d="M8 9V6h8v3" />
      <path d="M12 5v8" />
      <path d="m9 10 3 3 3-3" />
    </>
  ),
  outbound: (
    <>
      <path d="M5 19h14V9H5z" />
      <path d="M8 9V6h8v3" />
      <path d="M12 14V5" />
      <path d="m9 8 3-3 3 3" />
    </>
  ),
  stock: (
    <>
      <path d="M5 8h14v11H5z" />
      <path d="M7 5h10v3H7z" />
      <path d="M9 12h6" />
      <path d="M9 16h4" />
    </>
  ),
  movements: (
    <>
      <path d="M4 7h11" />
      <path d="m12 4 3 3-3 3" />
      <path d="M20 17H9" />
      <path d="m12 14-3 3 3 3" />
    </>
  ),
  stocktake: (
    <>
      <path d="M8 4h8l2 3v13H6V7z" />
      <path d="M8 7h8" />
      <path d="m9 13 2 2 4-5" />
    </>
  ),
  adjustments: (
    <>
      <path d="M5 7h14" />
      <path d="M8 7v10" />
      <path d="M16 7v10" />
      <path d="M6 17h6" />
      <path d="M14 17h4" />
      <path d="M10 13h8" />
    </>
  ),
  reports: (
    <>
      <path d="M5 20V5" />
      <path d="M5 20h14" />
      <path d="M8.5 16v-4" />
      <path d="M12 16V8" />
      <path d="M15.5 16v-6" />
    </>
  ),
  import: (
    <>
      <path d="M6 4h8l4 4v12H6z" />
      <path d="M14 4v4h4" />
      <path d="M12 10v6" />
      <path d="m9 13 3 3 3-3" />
    </>
  ),
  departments: (
    <>
      <path d="M4 20h16" />
      <path d="M6 20V8h6v12" />
      <path d="M12 20V5h6v15" />
      <path d="M8 11h2" />
      <path d="M14 9h2" />
      <path d="M14 13h2" />
    </>
  ),
  categories: (
    <>
      <path d="M5 6h5l2 2h7v10H5z" />
      <path d="M8 13h8" />
      <path d="M8 16h5" />
    </>
  ),
  units: (
    <>
      <path d="M6 18h12" />
      <path d="M8 6v12" />
      <path d="M16 6v12" />
      <path d="M8 8h8" />
      <path d="M8 14h8" />
    </>
  ),
  suppliers: (
    <>
      <path d="M4 17h2" />
      <path d="M18 17h2" />
      <path d="M6 17h12" />
      <path d="M7 13V7h7v10" />
      <path d="M14 10h3l2 3v4" />
      <path d="M8 17a2 2 0 1 0 4 0" />
      <path d="M16 17a2 2 0 1 0 4 0" />
    </>
  ),
  budgets: (
    <>
      <path d="M6 5h12v14H6z" />
      <path d="M9 9h6" />
      <path d="M9 13h2" />
      <path d="M13 13h2" />
      <path d="M9 16h6" />
    </>
  ),
  approvals: (
    <>
      <path d="M7 4h10v16H7z" />
      <path d="M9.5 8h5" />
      <path d="m9.5 14 2 2 3.5-4" />
    </>
  ),
  backups: (
    <>
      <path d="M5 7h14v12H5z" />
      <path d="M8 7V5h8v2" />
      <path d="M8 12h8" />
      <path d="M8 15h5" />
    </>
  ),
  logs: (
    <>
      <path d="M6 5h12v14H6z" />
      <path d="M9 9h6" />
      <path d="M9 12h6" />
      <path d="M9 15h4" />
      <path d="M6 5 4.5 6.5" />
      <path d="M18 5l1.5 1.5" />
    </>
  ),
  settings: (
    <>
      <circle cx="12" cy="12" r="3" />
      <path d="M12 4v2" />
      <path d="M12 18v2" />
      <path d="m5.6 5.6 1.4 1.4" />
      <path d="m17 17 1.4 1.4" />
      <path d="M4 12h2" />
      <path d="M18 12h2" />
      <path d="m5.6 18.4 1.4-1.4" />
      <path d="m17 7 1.4-1.4" />
    </>
  ),
  users: (
    <>
      <circle cx="9" cy="8" r="3" />
      <path d="M4.5 19a4.5 4.5 0 0 1 9 0" />
      <path d="M16 10a2.5 2.5 0 0 1 0 5" />
      <path d="M18 19a4 4 0 0 0-3-3.8" />
    </>
  ),
};

function NavIcon({ name }: { name: NavKey }) {
  return <svg {...navIconProps}>{navIconContent[name]}</svg>;
}

function GitHubIcon() {
  return (
    <svg
      aria-hidden="true"
      className="topbar-icon"
      focusable="false"
      viewBox="0 0 24 24"
    >
      <path
        d="M12 2.25a9.75 9.75 0 0 0-3.08 19c.49.09.67-.21.67-.47v-1.67c-2.72.59-3.3-1.18-3.3-1.18-.44-1.13-1.08-1.43-1.08-1.43-.88-.6.07-.59.07-.59.98.07 1.49 1 1.49 1 .87 1.48 2.27 1.05 2.82.8.09-.62.34-1.05.61-1.29-2.17-.25-4.45-1.09-4.45-4.83 0-1.07.38-1.94 1-2.62-.1-.25-.43-1.24.1-2.58 0 0 .82-.26 2.68 1a9.2 9.2 0 0 1 4.88 0c1.86-1.26 2.68-1 2.68-1 .53 1.34.2 2.33.1 2.58.62.68 1 1.55 1 2.62 0 3.75-2.28 4.58-4.46 4.82.35.3.66.9.66 1.82v2.7c0 .26.18.56.68.46A9.75 9.75 0 0 0 12 2.25Z"
        fill="currentColor"
      />
    </svg>
  );
}

const defaultI18n = createI18n("zh-CN");

const navItems: NavItem[] = [
  { key: "dashboard", labelKey: "nav.dashboard" },
  { key: "items", labelKey: "nav.items" },
  { key: "inbound", labelKey: "nav.inbound" },
  { key: "outbound", labelKey: "nav.outbound" },
  { key: "stock", labelKey: "nav.stock" },
  { key: "movements", labelKey: "nav.movements" },
  { key: "stocktake", labelKey: "nav.stocktake" },
  { key: "adjustments", labelKey: "nav.adjustments" },
  { key: "reports", labelKey: "nav.reports" },
  { key: "import", labelKey: "nav.import" },
  { key: "departments", labelKey: "nav.departments" },
  { key: "categories", labelKey: "nav.categories" },
  { key: "units", labelKey: "nav.units" },
  { key: "suppliers", labelKey: "nav.suppliers" },
  { key: "budgets", labelKey: "nav.budgets" },
  { key: "approvals", labelKey: "nav.approvals" },
  { key: "backups", labelKey: "nav.backups" },
  { key: "logs", labelKey: "nav.logs" },
  { key: "settings", labelKey: "nav.settings" },
  { key: "users", labelKey: "nav.users" },
];

const navGroups: { titleKey: string; keys: NavKey[] }[] = [
  { titleKey: "navGroup.overview", keys: ["dashboard"] },
  {
    titleKey: "navGroup.inventory",
    keys: [
      "items",
      "inbound",
      "outbound",
      "stocktake",
      "adjustments",
    ],
  },
  {
    titleKey: "navGroup.reports",
    keys: ["stock", "movements", "reports"],
  },
  {
    titleKey: "navGroup.masterData",
    keys: ["departments", "categories", "units", "suppliers"],
  },
  {
    titleKey: "navGroup.management",
    keys: ["import", "budgets", "approvals", "users"],
  },
  {
    titleKey: "navGroup.logs",
    keys: ["backups", "logs"],
  },
];

const workstreams = [
  {
    titleKey: "workstream.inventory.title",
    bodyKey: "workstream.inventory.body",
  },
  {
    titleKey: "workstream.host.title",
    bodyKey: "workstream.host.body",
  },
  {
    titleKey: "workstream.recovery.title",
    bodyKey: "workstream.recovery.body",
  },
  {
    titleKey: "workstream.delivery.title",
    bodyKey: "workstream.delivery.body",
  },
];

const CLIENT_RECONNECT_INTERVAL_MS = 15000;
const MAIN_WINDOW_LABEL = "main";
const openingEditorWindows = new Set<string>();

const APPEARANCE_STORAGE_KEY = "aster.appearance";
const APPEARANCE_CHANGED_EVENT = "appearance:changed";

declare global {
  interface Window {
    __TAURI_OS_PLUGIN_INTERNALS__?: {
      platform?: string;
    };
  }
}

function detectDesktopPlatform() {
  const tauriPlatform = window.__TAURI_OS_PLUGIN_INTERNALS__?.platform;
  if (tauriPlatform) return tauriPlatform.toLowerCase();
  if (navigator.userAgent.includes("Windows")) return "windows";
  if (navigator.userAgent.includes("Mac")) return "macos";
  return "unknown";
}
const defaultAppearanceSettings: AppearanceSettings = {
  themeMode: "auto",
  liquidGlassStyle: "tinted",
  accentColor: "#2f6dff",
  locale: "zh-CN",
};

const emptyItem = {
  id: undefined as string | undefined,
  expectedUpdatedAt: undefined as string | undefined,
  code: "",
  barcode: "",
  name: "",
  categoryId: "",
  spec: "",
  unitId: "",
  defaultPrice: 0,
  salePrice: 0,
  supplierId: "",
  warningQuantity: 0,
  enabled: true,
  remark: "",
};

function currentDateTimeString() {
  const now = new Date();
  const offsetMs = now.getTimezoneOffset() * 60 * 1000;
  return new Date(now.getTime() - offsetMs).toISOString().slice(0, 19);
}

function currentMonthString() {
  return new Date().toISOString().slice(0, 7);
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

async function chooseSinglePath(options: OpenDialogOptions) {
  const selected = await open({ ...options, multiple: false });
  if (typeof selected === "string") {
    return selected;
  }
  if (Array.isArray(selected)) {
    return selected[0] ?? null;
  }
  return null;
}

function resolveTheme(mode: ThemeMode): "light" | "dark" {
  if (mode === "dark" || mode === "light") {
    return mode;
  }
  if (
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-color-scheme: dark)").matches
  ) {
    return "dark";
  }
  return "light";
}

function resolveAccentContrast(color: string) {
  const normalized = color.replace("#", "");
  if (!/^[0-9a-fA-F]{6}$/.test(normalized)) {
    return "#ffffff";
  }
  const red = Number.parseInt(normalized.slice(0, 2), 16) / 255;
  const green = Number.parseInt(normalized.slice(2, 4), 16) / 255;
  const blue = Number.parseInt(normalized.slice(4, 6), 16) / 255;
  const luminance = 0.2126 * red + 0.7152 * green + 0.0722 * blue;
  return luminance > 0.66 ? "#101114" : "#ffffff";
}

function normalizeAppearanceSettings(
  value: Partial<AppearanceSettings> | null,
): AppearanceSettings {
  const themeMode: ThemeMode =
    value?.themeMode === "light" || value?.themeMode === "dark"
      ? value.themeMode
      : "auto";
  const liquidGlassStyle: LiquidGlassStyle =
    value?.liquidGlassStyle === "transparent" ? "transparent" : "tinted";
  const accentColor =
    typeof value?.accentColor === "string" &&
    /^#[0-9a-fA-F]{6}$/.test(value.accentColor)
      ? value.accentColor.toLowerCase()
      : defaultAppearanceSettings.accentColor;
  const locale: LocaleCode = value?.locale === "en-US" ? "en-US" : "zh-CN";

  return { themeMode, liquidGlassStyle, accentColor, locale };
}

function loadAppearanceSettings() {
  try {
    const raw = window.localStorage.getItem(APPEARANCE_STORAGE_KEY);
    return normalizeAppearanceSettings(
      raw ? (JSON.parse(raw) as Partial<AppearanceSettings>) : null,
    );
  } catch {
    return defaultAppearanceSettings;
  }
}

function applyAppearanceSettings(settings: AppearanceSettings) {
  const root = document.documentElement;
  const theme = resolveTheme(settings.themeMode);
  root.dataset.theme = theme;
  root.dataset.glassStyle = settings.liquidGlassStyle;
  root.style.setProperty("--accent", settings.accentColor);
  root.style.setProperty(
    "--accent-contrast",
    resolveAccentContrast(settings.accentColor),
  );
}

function useSyncedAppearanceSettings(settings?: AppearanceSettings) {
  useEffect(() => {
    const appearance = settings ?? loadAppearanceSettings();
    applyAppearanceSettings(appearance);
    document.documentElement.lang = appearance.locale;
    if (settings) {
      window.localStorage.setItem(
        APPEARANCE_STORAGE_KEY,
        JSON.stringify(settings),
      );
    }
  }, [settings]);

  useEffect(() => {
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handleSystemThemeChange = () => {
      const appearance = settings ?? loadAppearanceSettings();
      applyAppearanceSettings(appearance);
    };
    mediaQuery.addEventListener("change", handleSystemThemeChange);
    return () =>
      mediaQuery.removeEventListener("change", handleSystemThemeChange);
  }, [settings]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<AppearanceSettings>(APPEARANCE_CHANGED_EVENT, (event) => {
      const appearance = normalizeAppearanceSettings(event.payload);
      applyAppearanceSettings(appearance);
      document.documentElement.lang = appearance.locale;
    }).then((nextUnlisten) => {
      unlisten = nextUnlisten;
    });
    return () => {
      unlisten?.();
    };
  }, []);
}

function useBroadcastAppearanceSettings(settings: AppearanceSettings) {
  useEffect(() => {
    void emit(APPEARANCE_CHANGED_EVENT, settings);
  }, [settings]);
}

function effectiveLineAmount(line: {
  quantity: number;
  unitPrice: number;
  amount?: number | null;
}) {
  return line.amount && line.amount > 0
    ? line.amount
    : line.quantity * line.unitPrice;
}

function effectiveDraftAmount(
  line: StockDocumentLineDraft,
  documentType: "inbound" | "outbound",
  outboundKind?: "internal" | "guest_sale",
) {
  if (documentType === "inbound") {
    const unitPrice = line.purchaseUnitPrice ?? line.unitPrice;
    return line.purchaseAmount && line.purchaseAmount > 0
      ? line.purchaseAmount
      : line.amount && line.amount > 0
        ? line.amount
        : line.quantity * unitPrice;
  }
  if (outboundKind === "guest_sale") {
    const unitPrice = line.saleUnitPrice ?? line.unitPrice;
    return line.saleAmount && line.saleAmount > 0
      ? line.saleAmount
      : line.amount && line.amount > 0
        ? line.amount
        : line.quantity * unitPrice;
  }
  return line.costAmount && line.costAmount > 0
    ? line.costAmount
    : line.amount && line.amount > 0
      ? line.amount
      : 0;
}

function uniqueTextOptions(values: Array<string | null | undefined>) {
  const seen = new Set<string>();
  const options: string[] = [];
  values.forEach((value) => {
    const text = String(value ?? "").trim();
    if (!text || seen.has(text)) return;
    seen.add(text);
    options.push(text);
  });
  return options;
}

function userDisplayName(user?: CurrentUser | UserAccount | null) {
  if (!user) return "";
  return user.displayName?.trim() || user.username;
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
    if (!status.runtime.clientToken) return i18n.t("connection.unpaired");
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
    if (!status.runtime.clientToken) {
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
    if (!status.runtime.clientToken || hostTestResult?.ok === false) {
      return "warning";
    }
    return "success";
  }
  return "success";
}

function optionName(options: OptionRecord[], id?: string | null) {
  return options.find((item) => item.id === id)?.name ?? "-";
}

function approvalTypeLabel(type: string, i18n = defaultI18n) {
  return i18n.approvalTypeLabel(type);
}

function approvalStatusLabel(status: string, i18n = defaultI18n) {
  return i18n.approvalStatusLabel(status);
}

function stocktakeStatusLabel(status: string, i18n = defaultI18n) {
  return i18n.stocktakeStatusLabel(status);
}

function stocktakeScopeLabel(scope: string, i18n = defaultI18n) {
  return i18n.stocktakeScopeLabel(scope);
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

function formatError(err: unknown) {
  const message = err instanceof Error ? err.message : String(err);
  if (
    message.includes("Cannot read properties of undefined") &&
    message.includes("invoke")
  ) {
    return "当前页面需要在 Aster 桌面客户端中运行；浏览器预览只能查看界面壳，不能访问本地 SQLite 和 Tauri 命令。";
  }
  return message;
}

async function loadProxyCandidates() {
  try {
    const candidates = await invoke<ProxyCandidate[]>(
      "get_system_proxy_candidates",
    );
    return candidates.filter((candidate) => candidate.url.trim());
  } catch {
    return [];
  }
}

async function checkAppUpdateWithFallback() {
  const attempts: Array<{
    label: string;
    options?: CheckOptions;
    proxy?: string;
  }> = [
    { label: "直连" },
  ];
  for (const candidate of await loadProxyCandidates()) {
    attempts.push({
      label: candidate.label,
      options: { proxy: candidate.url },
      proxy: candidate.url,
    });
  }

  const errors: string[] = [];
  for (const attempt of attempts) {
    try {
      const update = await check(attempt.options);
      return { attemptLabel: attempt.label, proxy: attempt.proxy ?? null, update };
    } catch (err) {
      errors.push(`${attempt.label}：${formatError(err)}`);
    }
  }

  throw new Error(
    [
      "无法连接更新源，已尝试直连和本机代理。",
      "如果当前电脑已开启 VPN，请确认 VPN 代理允许桌面应用访问 GitHub Releases。",
      errors[errors.length - 1] ?? "",
    ]
      .filter(Boolean)
      .join("\n"),
  );
}

function editorTitle(
  editor: EditorKind,
  mode: EditorMode,
  documentType?: "inbound" | "outbound",
) {
  if (editor === "stockDocumentDetail") {
    return documentType === "outbound" ? "出库/领用单详情" : "入库单详情";
  }
  if (editor === "stockBatchDetail") {
    return "批次库存";
  }
  if (editor === "stocktakeDetail") {
    return "盘点详情";
  }
  if (editor === "stockDocument") {
    return documentType === "outbound" ? "新建出库/领用单" : "新建入库单";
  }
  const labels: Record<
    Exclude<
      EditorKind,
      | "stockDocument"
      | "stockDocumentDetail"
      | "stockBatchDetail"
      | "stocktakeDetail"
    >,
    string
  > = {
    adjustment: "库存调整",
    budget: "预算规则",
    businessSettings: "业务与目录设置",
    category: "分类",
    changePassword: "修改密码",
    passwordReset: "找回密码",
    clientConnection: "客户端连接",
    clientPairing: "客户端配对",
    connectionWizard: "多电脑连接",
    department: "部门",
    item: "物品",
    restoreBackup: "恢复备份",
    secondBackupDir: "第二备份目录",
    softwareUpdate: "软件更新",
    stocktakeCounts: "盘点实盘",
    stocktakeCreate: "创建盘点单",
    supplier: "供应商",
    unit: "单位",
    user: "用户",
  };
  const action = mode === "edit" ? "编辑" : "新增";
  if (editor === "stocktakeCounts") return "录入盘点实盘";
  if (editor === "stocktakeCreate") return "创建盘点单";
  if (editor === "adjustment") return "新建调整单";
  if (
    editor === "changePassword" ||
    editor === "passwordReset" ||
    editor === "businessSettings" ||
    editor === "softwareUpdate" ||
    editor === "clientConnection" ||
    editor === "clientPairing" ||
    editor === "connectionWizard" ||
    editor === "secondBackupDir" ||
    editor === "restoreBackup"
  ) {
    return labels[editor];
  }
  return `${action}${labels[editor]}`;
}

function editorUrl(params: Record<string, string | undefined>) {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value) search.set(key, value);
  }
  return `${window.location.pathname}?${search.toString()}`;
}

function editorWindowSize(editor: EditorKind) {
  const compactEditors: EditorKind[] = [
    "department",
    "category",
    "unit",
    "supplier",
    "budget",
    "changePassword",
    "secondBackupDir",
  ];
  if (compactEditors.includes(editor)) {
    return { width: 620, height: 380, minWidth: 520, minHeight: 320 };
  }
  if (editor === "passwordReset") {
    return { width: 460, height: 420, minWidth: 420, minHeight: 360 };
  }
  if (editor === "item" || editor === "user" || editor === "businessSettings") {
    return { width: 760, height: 560, minWidth: 640, minHeight: 420 };
  }
  if (
    editor === "stockDocumentDetail" ||
    editor === "stockBatchDetail" ||
    editor === "stocktakeDetail"
  ) {
    return { width: 980, height: 680, minWidth: 760, minHeight: 520 };
  }
  if (editor === "clientConnection" || editor === "restoreBackup") {
    return { width: 720, height: 560, minWidth: 620, minHeight: 420 };
  }
  if (editor === "clientPairing") {
    return { width: 620, height: 420, minWidth: 520, minHeight: 340 };
  }
  if (editor === "connectionWizard") {
    return { width: 680, height: 560, minWidth: 560, minHeight: 460 };
  }
  if (editor === "softwareUpdate") {
    return { width: 760, height: 620, minWidth: 620, minHeight: 480 };
  }
  return { width: 860, height: 720, minWidth: 680, minHeight: 560 };
}

async function bringEditorWindowToFront(windowRef: WebviewWindow) {
  if (await windowRef.isMinimized()) {
    await windowRef.unminimize();
  }
  if (!(await windowRef.isVisible())) {
    await windowRef.show();
  }
  await windowRef.setFocus();
}

async function openEditorWindow(
  editor: EditorKind,
  options: {
    extra?: Record<string, string | undefined>;
    mode?: EditorMode;
    id?: string;
    documentType?: "inbound" | "outbound";
    width?: number;
    height?: number;
  } = {},
) {
  const mode = options.mode ?? "create";
  const stableContext =
    options.id ?? options.documentType ?? options.extra?.periodMonth ?? "new";
  const labelParts = ["editor", editor, mode, stableContext]
    .filter(Boolean)
    .map((part) => String(part).replace(/[^a-zA-Z0-9_-]/g, "-"));
  const label = labelParts.join("-");
  const title = editorTitle(editor, mode, options.documentType);
  const existing = await WebviewWindow.getByLabel(label);
  if (existing) {
    await bringEditorWindowToFront(existing);
    return;
  }
  if (openingEditorWindows.has(label)) {
    window.setTimeout(() => {
      void WebviewWindow.getByLabel(label).then((windowRef) => {
        if (windowRef) void bringEditorWindowToFront(windowRef);
      });
    }, 120);
    return;
  }
  openingEditorWindows.add(label);
  try {
    const size = editorWindowSize(editor);
    const windowRef = new WebviewWindow(label, {
      center: true,
      height: options.height ?? size.height,
      minHeight: size.minHeight,
      minWidth: size.minWidth,
      resizable: true,
      title,
      url: editorUrl({
        documentType: options.documentType,
        editor,
        id: options.id,
        mode,
        ...options.extra,
      }),
      width: options.width ?? size.width,
    });
    await new Promise<void>((resolve) => {
      windowRef.once("tauri://created", () => resolve());
      windowRef.once("tauri://error", () => resolve());
    });
  } finally {
    openingEditorWindows.delete(label);
  }
}

async function closeCurrentEditorWindow() {
  await WebviewWindow.getCurrent().close();
}

async function notifyEditorSaved(payload: EditorSavedPayload) {
  await emitTo(MAIN_WINDOW_LABEL, "editor:saved", payload);
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
  const [activeNav, setActiveNav] = useState<NavKey>("dashboard");
  const [appearanceSettings, setAppearanceSettings] =
    useState<AppearanceSettings>(() => loadAppearanceSettings());
  const i18n = useMemo(
    () => createI18n(appearanceSettings.locale),
    [appearanceSettings.locale],
  );
  useSyncedAppearanceSettings(appearanceSettings);
  useBroadcastAppearanceSettings(appearanceSettings);
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [currentUser, setCurrentUser] = useState<CurrentUser | null>(null);
  const [userAccounts, setUserAccounts] = useState<UserAccount[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [isLoginPending, setIsLoginPending] = useState(false);
  const [passwordChangeRequired, setPasswordChangeRequired] = useState(false);
  const [isSavingMode, setIsSavingMode] = useState(false);
  const [categories, setCategories] = useState<Category[]>([]);
  const [units, setUnits] = useState<Unit[]>([]);
  const [departments, setDepartments] = useState<Department[]>([]);
  const [suppliers, setSuppliers] = useState<Supplier[]>([]);
  const [supplierPurchaseRecords, setSupplierPurchaseRecords] = useState<
    SupplierPurchaseRecord[]
  >([]);
  const [activeSupplier, setActiveSupplier] = useState<Supplier | null>(null);
  const [budgetRules, setBudgetRules] = useState<BudgetRule[]>([]);
  const [approvalRequests, setApprovalRequests] = useState<ApprovalRequest[]>(
    [],
  );
  const [items, setItems] = useState<Item[]>([]);
  const [itemSearch, setItemSearch] = useState("");
  const [inboundDocuments, setInboundDocuments] = useState<StockDocument[]>([]);
  const [outboundDocuments, setOutboundDocuments] = useState<StockDocument[]>(
    [],
  );
  const [adjustmentDocuments, setAdjustmentDocuments] = useState<
    StockDocument[]
  >([]);
  const [inboundDocumentQuery, setInboundDocumentQuery] =
    useState<StockDocumentQuery>({
      documentType: "inbound",
      month: currentMonthString(),
    });
  const [outboundDocumentQuery, setOutboundDocumentQuery] =
    useState<StockDocumentQuery>({
      documentType: "outbound",
      month: currentMonthString(),
    });
  const [adjustmentDocumentQuery, setAdjustmentDocumentQuery] =
    useState<StockDocumentQuery>({
      documentType: "adjustment",
      month: currentMonthString(),
    });
  const [stockBalances, setStockBalances] = useState<StockBalanceRow[]>([]);
  const [stockMovements, setStockMovements] = useState<StockMovementRow[]>([]);
  const [stockBalanceQuery, setStockBalanceQuery] = useState<StockBalanceQuery>(
    {},
  );
  const [stockMovementQuery, setStockMovementQuery] =
    useState<StockMovementQuery>({});
  const [stocktakes, setStocktakes] = useState<StocktakeDocument[]>([]);
  const [reportMonth, setReportMonth] = useState(currentMonthString());
  const [reportQuery, setReportQuery] = useState<ReportQuery>({
    month: currentMonthString(),
  });
  const [hasManualReportMonth, setHasManualReportMonth] = useState(false);
  const [reportBundle, setReportBundle] = useState<ReportBundle | null>(null);
  const [lastExportPath, setLastExportPath] = useState<string | null>(null);
  const [importPreview, setImportPreview] = useState<ImportPreview | null>(
    null,
  );
  const [importResult, setImportResult] = useState<ImportResult | null>(null);
  const [isImporting, setIsImporting] = useState(false);
  const [backupRecords, setBackupRecords] = useState<BackupRecord[]>([]);
  const [auditLogs, setAuditLogs] = useState<AuditLogRow[]>([]);
  const [lastBackup, setLastBackup] = useState<BackupSummary | null>(null);
  const [isBackupWorking, setIsBackupWorking] = useState(false);
  const [systemSettings, setSystemSettings] = useState<SystemSettings | null>(
    null,
  );
  const [hostStatus, setHostStatus] = useState<HostServiceStatus | null>(null);
  const [clientConnections, setClientConnections] = useState<
    ClientConnectionInfo[]
  >([]);
  const [hostTestResult, setHostTestResult] =
    useState<HostConnectionTestResult | null>(null);
  const [clientConnectionCheckedAt, setClientConnectionCheckedAt] = useState<
    string | null
  >(null);
  const [updateState, setUpdateState] = useState<AppUpdateState>({
    ...initialUpdateState,
  });
  const hasCheckedUpdateOnStartupRef = useRef(false);

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

  async function runAction(message: string, action: () => Promise<unknown>) {
    try {
      setError(null);
      setNotice(null);
      await action();
      await refreshAll();
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
    });
  }

  async function exportItems(search = itemSearch) {
    try {
      setError(null);
      setNotice(null);
      const result = await invoke<{ path: string }>("export_items", {
        search,
      });
      await refreshAll();
      setNotice(`物品档案已导出：${result.path}`);
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
      await refreshAll();
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
      await refreshAll();
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
    });
  }

  async function startHostRuntime() {
    await runAction("主机服务已启动", async () => {
      const status = await invoke<HostServiceStatus>("start_host_service");
      setHostStatus(status);
      await loadHostRuntime();
    });
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
    });
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
    );
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
    );
  }

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
      }
      scheduleRefreshAll();
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
  const isClientPaired = Boolean(status?.runtime.clientToken);
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
    <div className="app-shell" data-platform={desktopPlatform}>
      <aside className="sidebar">
        <div className="brand">
          <PantsLogo />
          <div>
            <strong>Aster</strong>
            <span>{i18n.t("app.productTagline")}</span>
          </div>
        </div>
        <nav className="nav-list">
          {navGroups.map((group) => {
            const groupItems = group.keys
              .map((key) => visibleNavItems.find((item) => item.key === key))
              .filter((item): item is NavItem => Boolean(item));

            if (groupItems.length === 0) return null;

            return (
              <div className="nav-section" key={group.titleKey}>
                <span className="nav-section-title">
                  {i18n.t(group.titleKey)}
                </span>
                {groupItems.map((item) => (
                  <button
                    className={
                      activeNav === item.key ? "nav-item active" : "nav-item"
                    }
                    key={item.key}
                    onClick={() => setActiveNav(item.key)}
                  >
                    <NavIcon name={item.key} />
                    <span className="nav-item-label">
                      {i18n.t(item.labelKey)}
                    </span>
                  </button>
                ))}
              </div>
            );
          })}
        </nav>
        {settingsNavItem ? (
          <div className="sidebar-footer">
            <button
              className={
                activeNav === settingsNavItem.key
                  ? "nav-item active"
                  : "nav-item"
              }
              onClick={() => setActiveNav(settingsNavItem.key)}
            >
              <NavIcon name={settingsNavItem.key} />
              <span className="nav-item-label">
                {i18n.t(settingsNavItem.labelKey)}
              </span>
            </button>
            <button
              className={`sidebar-connection sidebar-connection-${sidebarConnectionKind}`}
              onClick={() => setActiveNav(settingsNavItem.key)}
              title={connectionStatusHint(
                status,
                hostStatus,
                hostTestResult,
                i18n,
              )}
            >
              <span className="sidebar-connection-dot" />
              <span className="sidebar-connection-copy">
                <strong>
                  {connectionStatusLabel(status, hostTestResult, i18n)}
                </strong>
                <em>
                  {connectionStatusHint(
                    status,
                    hostStatus,
                    hostTestResult,
                    i18n,
                  )}
                </em>
              </span>
            </button>
          </div>
        ) : null}
      </aside>

      <main className="content">
        <header className="topbar">
          <div>
            <h1>
              {i18n.t(
                visibleNavItems.find((item) => item.key === activeNav)
                  ?.labelKey ?? "app.home",
              )}
            </h1>
          </div>
          <div className="topbar-actions">
            <button
              aria-label={i18n.t("app.githubAria")}
              className="ghost-button icon-button"
              onClick={() => void openUrl("https://github.com/westng")}
              title="GitHub"
            >
              <GitHubIcon />
            </button>
            <button className="ghost-button" onClick={() => refreshAll()}>
              {i18n.t("app.refreshStatus")}
            </button>
            <button className="ghost-button" onClick={logoutUser}>
              {i18n.t("app.logout")}
            </button>
          </div>
        </header>

        <div className="content-body">
          {activeNav === "dashboard" ? (
            <Dashboard
              changeMode={changeMode}
              i18n={i18n}
              isSavingMode={isSavingMode}
              metricCards={metricCards}
              modeLabel={(mode) => modeLabel(mode, i18n)}
              movementTypeLabel={(type) => movementTypeLabel(type, i18n)}
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
            statusLabel={approvalStatusLabel}
            typeLabel={approvalTypeLabel}
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
            currentUser={currentUser}
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
        </div>

        <footer className={`app-statusbar app-statusbar-${footerStatus.kind}`}>
          <span className="app-statusbar-indicator" />
          <span className="app-statusbar-message">{footerStatus.text}</span>
          <span className="app-statusbar-meta">
            {status
              ? `${modeLabel(status.runtime.mode, i18n)} · Schema v${status.schemaVersion}`
              : i18n.t("app.initializing")}
          </span>
        </footer>
      </main>
    </div>
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

  let content: React.ReactNode = null;
  if (editor === "item") {
    content = (
      <ItemEditor
        categories={enabledCategories}
        disabled={isSaving || isLoading}
        item={items.find((item) => item.id === id)}
        mode={mode}
        onSave={(request) =>
          runEditorAction({ editor, message: "物品已保存" }, () =>
            invoke("save_item", { request }),
          )
        }
        suppliers={enabledSuppliers}
        units={enabledUnits}
      />
    );
  } else if (editor === "department") {
    content = (
      <DepartmentEditor
        departments={departments}
        disabled={isSaving || isLoading}
        item={departments.find((item) => item.id === id)}
        mode={mode}
        onSave={(request) =>
          runEditorAction({ editor, message: "部门已保存" }, () =>
            invoke("save_department", { request }),
          )
        }
      />
    );
  } else if (editor === "category") {
    content = (
      <CategoryEditor
        categories={categories}
        disabled={isSaving || isLoading}
        item={categories.find((item) => item.id === id)}
        mode={mode}
        onSave={(request) =>
          runEditorAction({ editor, message: "分类已保存" }, () =>
            invoke("save_category", { request }),
          )
        }
      />
    );
  } else if (editor === "unit") {
    content = (
      <SimpleNameEditor
        disabled={isSaving || isLoading}
        fallbackSortOrder={units.length + 1}
        item={units.find((item) => item.id === id)}
        label="单位"
        mode={mode}
        onSave={(request) =>
          runEditorAction({ editor, message: "单位已保存" }, () =>
            invoke("save_unit", { request }),
          )
        }
      />
    );
  } else if (editor === "supplier") {
    content = (
      <SupplierEditor
        disabled={isSaving || isLoading}
        item={suppliers.find((item) => item.id === id)}
        mode={mode}
        onSave={(request) =>
          runEditorAction({ editor, message: "供应商已保存" }, () =>
            invoke("save_supplier", { request }),
          )
        }
      />
    );
  } else if (editor === "budget") {
    content = (
      <BudgetRuleEditor
        categories={enabledCategories}
        departments={enabledDepartments}
        disabled={isSaving || isLoading}
        mode={mode}
        onSave={(request) =>
          runEditorAction({ editor, message: "预算规则已保存" }, () =>
            invoke("save_budget_rule", { request }),
          )
        }
        periodMonth={periodMonth}
        rule={budgetRules.find((item) => item.id === id)}
      />
    );
  } else if (editor === "user") {
    content = (
      <UserEditor
        departments={departments}
        disabled={isSaving || isLoading}
        mode={mode}
        onSave={(request) =>
          runEditorAction({ editor, message: "用户已保存" }, () =>
            invoke("save_user_account", { request }),
          )
        }
        roles={roles}
        user={users.find((item) => item.id === id)}
      />
    );
  } else if (editor === "changePassword") {
    content = (
      <ChangePasswordEditor
        disabled={isSaving || isLoading}
        onSave={(request) =>
          runSettingsEditorAction("密码已修改", () =>
            invoke("change_password", { request }),
          )
        }
      />
    );
  } else if (editor === "passwordReset") {
    content = (
      <PasswordResetEditor
        disabled={isSaving || isLoading}
        i18n={createI18n(loadAppearanceSettings().locale)}
        onRequestCode={async (username) => {
          setIsSaving(true);
          try {
            setError(null);
            const result = await invoke<{
              maskedEmail: string;
              expiresMinutes: number;
            }>("request_password_reset_code", {
              request: { username },
            });
            setNotice(
              `验证码已发送至 ${result.maskedEmail}，${result.expiresMinutes} 分钟内有效。`,
            );
          } catch (err) {
            setError(formatError(err));
            throw err;
          } finally {
            setIsSaving(false);
          }
        }}
        onReset={(request) =>
          runEditorAction({ editor, message: "密码已重置，请使用新密码登录" }, () =>
            invoke("reset_password_with_code", { request }),
          )
        }
      />
    );
  } else if (editor === "businessSettings") {
    content = (
      <BusinessSettingsEditor
        disabled={isSaving || isLoading || !systemSettings}
        localDirectoriesOnly={status?.runtime.mode === "client"}
        settings={systemSettings}
        onSave={(request) =>
          runSettingsEditorAction("系统设置已保存", () =>
            invoke("save_system_settings", { request }),
          )
        }
      />
    );
  } else if (editor === "softwareUpdate") {
    content = (
      <SoftwareUpdateWindow
        disabled={isSaving || isLoading}
        i18n={createI18n(loadAppearanceSettings().locale)}
        status={status}
        updateState={updateState}
        onCheck={() => checkEditorUpdate()}
        onInstall={downloadAndInstallEditorUpdate}
        onRestart={() => relaunch()}
      />
    );
  } else if (editor === "clientConnection") {
    content = (
      <ClientConnectionEditor
        disabled={isSaving || isLoading || !status}
        status={status}
        onDiscover={(hostPort) => invoke("discover_hosts", { hostPort })}
        onSave={(hostAddress, hostPort) =>
          runSettingsEditorAction("客户端连接配置已保存", () =>
            invoke("save_client_config", {
              request: { hostAddress, hostPort },
            }),
          )
        }
        onTest={(hostAddress, hostPort) =>
          invoke("test_host_connection", {
            request: { hostAddress, hostPort },
          })
        }
      />
    );
  } else if (editor === "clientPairing") {
    content = (
      <ClientPairingEditor
        disabled={isSaving || isLoading || !status}
        status={status}
        onSave={(request) =>
          runSettingsEditorAction("客户端已完成主机配对", () =>
            invoke("pair_with_host", { request }),
          )
        }
      />
    );
  } else if (editor === "connectionWizard") {
    content = (
      <ConnectionWizard
        clientOnly={params.get("clientOnly") === "1"}
        clientConnections={clientConnections}
        disabled={isSaving || isLoading || !status}
        hostStatus={hostStatus}
        status={status}
        onDiscover={(hostPort) => invoke("discover_hosts", { hostPort })}
        onEnableHost={async () => {
          setIsSaving(true);
          try {
            setError(null);
            await invoke<RuntimeConfig>("set_runtime_mode", { mode: "host" });
            const nextHostStatus =
              await invoke<HostServiceStatus>("start_host_service");
            const [nextStatus, nextClients] = await Promise.all([
              invoke<AppStatus>("get_app_status"),
              invoke<ClientConnectionInfo[]>("list_client_connections"),
            ]);
            setStatus(nextStatus);
            setHostStatus(nextHostStatus);
            setClientConnections(nextClients);
            return nextHostStatus;
          } finally {
            setIsSaving(false);
          }
        }}
        onFinish={(message) =>
          runSettingsEditorAction(
            params.get("clientOnly") === "1"
              ? "已连接主电脑，请使用主机账号登录"
              : message,
            () => Promise.resolve(),
          )
        }
        onPair={async (request) => {
          setIsSaving(true);
          try {
            setError(null);
            await invoke<RuntimeConfig>("save_client_config", {
              request: {
                hostAddress: request.hostAddress,
                hostPort: request.hostPort,
              },
            });
            const nextRuntime = await invoke<RuntimeConfig>("pair_with_host", {
              request: {
                pairCode: request.pairCode,
                clientName: request.clientName,
                clientDeviceId: request.clientDeviceId,
              },
            });
            const nextStatus = await invoke<AppStatus>("get_app_status");
            setStatus(nextStatus);
            return nextRuntime;
          } finally {
            setIsSaving(false);
          }
        }}
        onRefreshHost={async () => {
          const [nextStatus, nextHostStatus] = await Promise.all([
            invoke<AppStatus>("get_app_status"),
            invoke<HostServiceStatus>("get_host_service_status"),
          ]);
          setStatus(nextStatus);
          setHostStatus(nextHostStatus);
          if (nextStatus.runtime.mode === "host") {
            setClientConnections(
              await invoke<ClientConnectionInfo[]>("list_client_connections"),
            );
          } else {
            setClientConnections([]);
          }
        }}
        onTest={(hostAddress, hostPort) =>
          invoke("test_host_connection", {
            request: { hostAddress, hostPort },
          })
        }
      />
    );
  } else if (editor === "secondBackupDir") {
    content = (
      <SecondBackupDirEditor
        disabled={isSaving || isLoading || !status}
        status={status}
        onSave={(path) =>
          runSettingsEditorAction("第二备份目录已保存", () =>
            invoke("set_second_backup_dir", { request: { path } }),
          )
        }
      />
    );
  } else if (editor === "restoreBackup") {
    content = (
      <RestoreBackupEditor
        disabled={isSaving || isLoading || !status}
        status={status}
        onPreview={(backupFile) =>
          invoke("preview_restore_backup", { backupFile })
        }
        onRestore={(request) =>
          runSettingsEditorAction("备份已恢复，数据库健康检查通过", () =>
            invoke("restore_backup", { request }),
          )
        }
      />
    );
  } else if (editor === "stockDocument" && documentType) {
    content = (
      <StockDocumentEditor
        balances={stockBalances}
        currentUser={currentUser}
        departments={enabledDepartments}
        disabled={isSaving || isLoading}
        documentType={documentType}
        items={enabledItems}
        onCreateApproval={(request) =>
          runEditorAction(
            { editor, documentType, message: "审批申请已提交" },
            () => invoke("create_approval_request", { request }),
          )
        }
        onSaveDraft={(request) =>
          runEditorAction(
            {
              editor,
              documentType,
              message:
                documentType === "outbound"
                  ? "出库/领用草稿已保存"
                  : "入库草稿已保存",
            },
            () => invoke("save_stock_document_draft", { request }),
          )
        }
        onSubmit={(request) =>
          runEditorAction(
            {
              editor,
              documentType,
              message:
                documentType === "outbound"
                  ? "出库/领用单已确认，库存已更新"
                  : "入库单已确认，库存已更新",
            },
            () => invoke("submit_stock_document", { request }),
          )
        }
        suppliers={enabledSuppliers}
      />
    );
  } else if (editor === "stockDocumentDetail") {
    content = (
      <StockDocumentDetailViewer
        detail={stockDocumentDetail}
        isLoading={isLoading}
      />
    );
  } else if (editor === "stockBatchDetail") {
    content = (
      <StockBatchDetailViewer batches={stockBatches} isLoading={isLoading} />
    );
  } else if (editor === "stocktakeDetail") {
    content = (
      <StocktakeDetailViewer
        detail={stocktakeDetail}
        disabled={isSaving || isLoading}
        isLoading={isLoading}
        onConfirm={(stocktakeId, handler, remark) =>
          runEditorAction(
            { editor, message: "盘点单已确认，差异流水已生成", stocktakeId },
            () =>
              invoke("confirm_stocktake", {
                request: { stocktakeId, handler, remark },
              }),
          )
        }
        onExport={async (stocktakeId) => {
          try {
            setIsSaving(true);
            setError(null);
            setNotice(null);
            const result = await invoke<{ path: string }>(
              "export_stocktake_sheet",
              {
                request: { stocktakeId },
              },
            );
            setNotice(`盘点表已导出：${result.path}`);
          } catch (err) {
            setError(formatError(err));
          } finally {
            setIsSaving(false);
          }
        }}
        onVoid={(documentId, stocktakeId, reason, handler) =>
          runEditorAction(
            { editor, message: "盘点单已作废，冲正流水已生成", stocktakeId },
            () =>
              invoke("void_stock_document", {
                request: { documentId, reason, handler },
              }),
          )
        }
      />
    );
  } else if (editor === "adjustment") {
    content = (
      <AdjustmentEditor
        currentUser={currentUser}
        disabled={isSaving || isLoading}
        items={enabledItems}
        onSubmit={(request) =>
          runEditorAction(
            { editor, message: "调整单已确认，库存流水已生成" },
            () => invoke("submit_adjustment", { request }),
          )
        }
      />
    );
  } else if (editor === "stocktakeCreate") {
    content = (
      <StocktakeCreateEditor
        categories={enabledCategories}
        disabled={isSaving || isLoading}
        items={enabledItems}
        onCreate={(request) =>
          runEditorAction({ editor, message: "盘点单已创建" }, () =>
            invoke("create_stocktake", { request }),
          )
        }
      />
    );
  } else if (editor === "stocktakeCounts") {
    content = (
      <StocktakeCountsEditor
        detail={stocktakeDetail}
        disabled={isSaving || isLoading}
        onSelect={async (stocktakeId) => {
          try {
            setError(null);
            setStocktakeDetail(
              await invoke<StocktakeDetail>("get_stocktake_detail", {
                stocktakeId,
              }),
            );
          } catch (err) {
            setError(formatError(err));
          }
        }}
        onSaveCounts={(stocktakeId, lines) =>
          runEditorAction(
            { editor, message: "实盘数量已保存", stocktakeId },
            () =>
              invoke("update_stocktake_counts", {
                request: { stocktakeId, lines },
              }),
          )
        }
        stocktakes={stocktakes}
      />
    );
  }

  return (
    <main className="editor-shell">
      <div className="editor-messages">
        {error ? <div className="error-banner">{error}</div> : null}
        {notice ? <div className="notice-banner">{notice}</div> : null}
      </div>
      <section className="editor-body">
        {content ?? (
          <div className="placeholder-panel">
            <h2>暂不支持的编辑窗口</h2>
          </div>
        )}
      </section>
    </main>
  );
}

function ItemEditor({
  categories,
  disabled,
  item,
  mode,
  onSave,
  suppliers,
  units,
}: {
  categories: OptionRecord[];
  disabled: boolean;
  item?: Item;
  mode: EditorMode;
  onSave: (request: typeof emptyItem) => Promise<void>;
  suppliers: OptionRecord[];
  units: OptionRecord[];
}) {
  const [draft, setDraft] = useState(emptyItem);
  useEffect(() => {
    if (mode === "edit" && item) {
      setDraft({
        id: item.id,
        expectedUpdatedAt: item.updatedAt,
        code: item.code,
        barcode: item.barcode ?? "",
        name: item.name,
        categoryId: item.categoryId ?? "",
        spec: item.spec ?? "",
        unitId: item.unitId ?? "",
        defaultPrice: item.defaultPrice,
        salePrice: item.salePrice,
        supplierId: item.supplierId ?? "",
        warningQuantity: item.warningQuantity,
        enabled: item.enabled,
        remark: item.remark ?? "",
      });
    }
  }, [item, mode]);
  return (
    <EditorForm
      disabled={disabled}
      saveLabel="保存物品"
      onSave={() => onSave(draft)}
    >
      {mode === "edit" ? (
        <Field label="编码">
          <input value={draft.code || "系统生成"} readOnly />
        </Field>
      ) : null}
      <Field label="条码">
        <input
          value={draft.barcode}
          onChange={(e) => setDraft({ ...draft, barcode: e.target.value })}
        />
      </Field>
      <Field label="名称">
        <input
          value={draft.name}
          onChange={(e) => setDraft({ ...draft, name: e.target.value })}
        />
      </Field>
      <Field label="分类">
        <select
          value={draft.categoryId}
          onChange={(e) => setDraft({ ...draft, categoryId: e.target.value })}
        >
          <option value="">未分类</option>
          {categories.map((record) => (
            <option key={record.id} value={record.id}>
              {record.name}
            </option>
          ))}
        </select>
      </Field>
      <Field label="规格">
        <input
          value={draft.spec}
          onChange={(e) => setDraft({ ...draft, spec: e.target.value })}
        />
      </Field>
      <Field label="单位">
        <select
          value={draft.unitId}
          onChange={(e) => setDraft({ ...draft, unitId: e.target.value })}
        >
          <option value="">未设置</option>
          {units.map((record) => (
            <option key={record.id} value={record.id}>
              {record.name}
            </option>
          ))}
        </select>
      </Field>
      <Field label="参考进价">
        <input
          min="0"
          type="number"
          value={draft.defaultPrice}
          onChange={(e) =>
            setDraft({ ...draft, defaultPrice: Number(e.target.value) })
          }
        />
      </Field>
      <Field label="参考售价">
        <input
          min="0"
          type="number"
          value={draft.salePrice}
          onChange={(e) =>
            setDraft({ ...draft, salePrice: Number(e.target.value) })
          }
        />
      </Field>
      <Field label="供应商">
        <select
          value={draft.supplierId}
          onChange={(e) => setDraft({ ...draft, supplierId: e.target.value })}
        >
          <option value="">未设置</option>
          {suppliers.map((record) => (
            <option key={record.id} value={record.id}>
              {record.name}
            </option>
          ))}
        </select>
      </Field>
      <Field label="预警线">
        <input
          min="0"
          type="number"
          value={draft.warningQuantity}
          onChange={(e) =>
            setDraft({ ...draft, warningQuantity: Number(e.target.value) })
          }
        />
      </Field>
      <Field label="备注">
        <input
          value={draft.remark}
          onChange={(e) => setDraft({ ...draft, remark: e.target.value })}
        />
      </Field>
      <label className="checkbox-field">
        <input
          checked={draft.enabled}
          onChange={(e) => setDraft({ ...draft, enabled: e.target.checked })}
          type="checkbox"
        />
        启用
      </label>
    </EditorForm>
  );
}

function DepartmentEditor({
  departments,
  disabled,
  item,
  mode,
  onSave,
}: {
  departments: Department[];
  disabled: boolean;
  item?: Department;
  mode: EditorMode;
  onSave: (request: DepartmentDraft) => Promise<void>;
}) {
  const [draft, setDraft] = useState<DepartmentDraft>({
    code: "",
    name: "",
    manager: "",
    enabled: true,
    sortOrder: departments.length + 1,
    remark: "",
  });
  useEffect(() => {
    if (mode === "edit" && item) {
      setDraft({
        ...item,
        id: item.id,
        expectedUpdatedAt: item.updatedAt,
        manager: item.manager ?? "",
        remark: item.remark ?? "",
      });
    } else {
      setDraft((current) => ({
        ...current,
        sortOrder: departments.length + 1,
      }));
    }
  }, [departments.length, item, mode]);
  return (
    <EditorForm
      disabled={disabled}
      saveLabel="保存部门"
      onSave={() => onSave(draft)}
    >
      <Field label="编码">
        <input
          value={draft.code}
          onChange={(e) => setDraft({ ...draft, code: e.target.value })}
        />
      </Field>
      <Field label="名称">
        <input
          value={draft.name}
          onChange={(e) => setDraft({ ...draft, name: e.target.value })}
        />
      </Field>
      <Field label="负责人">
        <input
          value={draft.manager ?? ""}
          onChange={(e) => setDraft({ ...draft, manager: e.target.value })}
        />
      </Field>
      <Field label="排序">
        <input
          type="number"
          value={draft.sortOrder}
          onChange={(e) =>
            setDraft({ ...draft, sortOrder: Number(e.target.value) })
          }
        />
      </Field>
      <Field label="备注">
        <input
          value={draft.remark ?? ""}
          onChange={(e) => setDraft({ ...draft, remark: e.target.value })}
        />
      </Field>
      <label className="checkbox-field">
        <input
          checked={draft.enabled}
          onChange={(e) => setDraft({ ...draft, enabled: e.target.checked })}
          type="checkbox"
        />
        启用
      </label>
    </EditorForm>
  );
}

function CategoryEditor({
  categories,
  disabled,
  item,
  mode,
  onSave,
}: {
  categories: Category[];
  disabled: boolean;
  item?: Category;
  mode: EditorMode;
  onSave: (request: CategoryDraft) => Promise<void>;
}) {
  const [draft, setDraft] = useState<CategoryDraft>({
    parentId: "",
    name: "",
    enabled: true,
    sortOrder: categories.length + 1,
  });
  useEffect(() => {
    if (mode === "edit" && item) {
      setDraft({
        id: item.id,
        expectedUpdatedAt: item.updatedAt,
        parentId: item.parentId ?? "",
        name: item.name,
        enabled: item.enabled,
        sortOrder: item.sortOrder,
      });
    } else {
      setDraft((current) => ({ ...current, sortOrder: categories.length + 1 }));
    }
  }, [categories.length, item, mode]);
  const parentOptions = categories.filter(
    (record) => !record.parentId && record.id !== draft.id,
  );
  return (
    <EditorForm
      disabled={disabled}
      saveLabel="保存分类"
      onSave={() => onSave({ ...draft, parentId: draft.parentId || "" })}
    >
      <Field label="上级分类">
        <select
          value={draft.parentId}
          onChange={(e) => setDraft({ ...draft, parentId: e.target.value })}
        >
          <option value="">作为大类</option>
          {parentOptions.map((record) => (
            <option key={record.id} value={record.id}>
              {record.name}
            </option>
          ))}
        </select>
      </Field>
      <Field label="名称">
        <input
          value={draft.name}
          onChange={(e) => setDraft({ ...draft, name: e.target.value })}
        />
      </Field>
      <Field label="排序">
        <input
          type="number"
          value={draft.sortOrder}
          onChange={(e) =>
            setDraft({ ...draft, sortOrder: Number(e.target.value) })
          }
        />
      </Field>
      <label className="checkbox-field">
        <input
          checked={draft.enabled}
          onChange={(e) => setDraft({ ...draft, enabled: e.target.checked })}
          type="checkbox"
        />
        启用
      </label>
    </EditorForm>
  );
}

function SimpleNameEditor({
  disabled,
  fallbackSortOrder,
  item,
  label,
  mode,
  onSave,
}: {
  disabled: boolean;
  fallbackSortOrder: number;
  item?: Unit | Category;
  label: string;
  mode: EditorMode;
  onSave: (request: SimpleNameDraft) => Promise<void>;
}) {
  const [draft, setDraft] = useState<SimpleNameDraft>({
    name: "",
    enabled: true,
    sortOrder: fallbackSortOrder,
  });
  useEffect(() => {
    if (mode === "edit" && item) {
      setDraft({
        id: item.id,
        expectedUpdatedAt: item.updatedAt,
        name: item.name,
        enabled: item.enabled,
        sortOrder: item.sortOrder,
      });
    } else {
      setDraft((current) => ({ ...current, sortOrder: fallbackSortOrder }));
    }
  }, [fallbackSortOrder, item, mode]);
  return (
    <EditorForm
      disabled={disabled}
      saveLabel={`保存${label}`}
      onSave={() => onSave(draft)}
    >
      <Field label="名称">
        <input
          value={draft.name}
          onChange={(e) => setDraft({ ...draft, name: e.target.value })}
        />
      </Field>
      <Field label="排序">
        <input
          type="number"
          value={draft.sortOrder}
          onChange={(e) =>
            setDraft({ ...draft, sortOrder: Number(e.target.value) })
          }
        />
      </Field>
      <label className="checkbox-field">
        <input
          checked={draft.enabled}
          onChange={(e) => setDraft({ ...draft, enabled: e.target.checked })}
          type="checkbox"
        />
        启用
      </label>
    </EditorForm>
  );
}

function SupplierEditor({
  disabled,
  item,
  mode,
  onSave,
}: {
  disabled: boolean;
  item?: Supplier;
  mode: EditorMode;
  onSave: (request: SupplierDraft) => Promise<void>;
}) {
  const [draft, setDraft] = useState<SupplierDraft>({
    name: "",
    contact: "",
    phone: "",
    address: "",
    enabled: true,
    remark: "",
  });
  useEffect(() => {
    if (mode === "edit" && item) {
      setDraft({
        ...item,
        id: item.id,
        expectedUpdatedAt: item.updatedAt,
        contact: item.contact ?? "",
        phone: item.phone ?? "",
        address: item.address ?? "",
        remark: item.remark ?? "",
      });
    }
  }, [item, mode]);
  return (
    <EditorForm
      disabled={disabled}
      saveLabel="保存供应商"
      onSave={() => onSave(draft)}
    >
      <Field label="名称">
        <input
          value={draft.name}
          onChange={(e) => setDraft({ ...draft, name: e.target.value })}
        />
      </Field>
      <Field label="联系人">
        <input
          value={draft.contact ?? ""}
          onChange={(e) => setDraft({ ...draft, contact: e.target.value })}
        />
      </Field>
      <Field label="电话">
        <input
          value={draft.phone ?? ""}
          onChange={(e) => setDraft({ ...draft, phone: e.target.value })}
        />
      </Field>
      <Field label="地址">
        <input
          value={draft.address ?? ""}
          onChange={(e) => setDraft({ ...draft, address: e.target.value })}
        />
      </Field>
      <Field label="备注">
        <input
          value={draft.remark ?? ""}
          onChange={(e) => setDraft({ ...draft, remark: e.target.value })}
        />
      </Field>
      <label className="checkbox-field">
        <input
          checked={draft.enabled}
          onChange={(e) => setDraft({ ...draft, enabled: e.target.checked })}
          type="checkbox"
        />
        启用
      </label>
    </EditorForm>
  );
}

function BudgetRuleEditor({
  categories,
  departments,
  disabled,
  mode,
  onSave,
  periodMonth,
  rule,
}: {
  categories: Category[];
  departments: Department[];
  disabled: boolean;
  mode: EditorMode;
  onSave: (request: BudgetRuleDraft) => Promise<void>;
  periodMonth: string;
  rule?: BudgetRule;
}) {
  const [draft, setDraft] = useState<BudgetRuleDraft>({
    departmentId: departments[0]?.id ?? "",
    categoryId: null,
    periodMonth,
    amountLimit: 0,
    enabled: true,
  });
  useEffect(() => {
    if (mode === "edit" && rule) {
      setDraft({
        id: rule.id,
        expectedUpdatedAt: rule.updatedAt,
        departmentId: rule.departmentId,
        categoryId: rule.categoryId ?? null,
        periodMonth: rule.periodMonth,
        amountLimit: rule.amountLimit,
        enabled: rule.enabled,
      });
    } else {
      setDraft((current) => ({
        ...current,
        departmentId: current.departmentId || departments[0]?.id || "",
        periodMonth: current.periodMonth || periodMonth,
      }));
    }
  }, [categories, departments, mode, periodMonth, rule]);
  return (
    <EditorForm
      disabled={
        disabled ||
        !draft.departmentId ||
        !draft.periodMonth
      }
      saveLabel="保存预算"
      onSave={() => onSave(draft)}
    >
      <Field label="月份">
        <MonthSelect
          value={draft.periodMonth}
          onChange={(periodMonth) => setDraft({ ...draft, periodMonth })}
        />
      </Field>
      <Field label="部门">
        <select
          value={draft.departmentId}
          onChange={(e) => setDraft({ ...draft, departmentId: e.target.value })}
        >
          {departments.map((record) => (
            <option key={record.id} value={record.id}>
              {record.name}
            </option>
          ))}
        </select>
      </Field>
      <Field label="分类">
        <select
          value={draft.categoryId ?? ""}
          onChange={(e) =>
            setDraft({ ...draft, categoryId: e.target.value || null })
          }
        >
          <option value="">全部分类</option>
          {categories.map((record) => (
            <option key={record.id} value={record.id}>
              {record.name}
            </option>
          ))}
        </select>
      </Field>
      <Field label="预算金额">
        <input
          min="0"
          step="0.01"
          type="number"
          value={draft.amountLimit}
          onChange={(e) =>
            setDraft({ ...draft, amountLimit: Number(e.target.value) })
          }
        />
      </Field>
      <label className="checkbox-field">
        <input
          checked={draft.enabled}
          onChange={(e) => setDraft({ ...draft, enabled: e.target.checked })}
          type="checkbox"
        />
        启用
      </label>
    </EditorForm>
  );
}

function UserEditor({
  departments,
  disabled,
  mode,
  onSave,
  roles,
  user,
}: {
  departments: Department[];
  disabled: boolean;
  mode: EditorMode;
  onSave: (request: {
    id?: string;
    username: string;
    displayName: string;
    email?: string | null;
    password?: string | null;
    departmentId?: string | null;
    enabled: boolean;
    roleCodes: string[];
  }) => Promise<void>;
  roles: Role[];
  user?: UserAccount;
}) {
  const empty = {
    id: undefined as string | undefined,
    username: "",
    displayName: "",
    email: "",
    password: "",
    departmentId: "",
    enabled: true,
    roleCodes: ["warehouse"] as string[],
  };
  const [draft, setDraft] = useState(empty);
  useEffect(() => {
    if (mode === "edit" && user) {
      setDraft({
        id: user.id,
        username: user.username,
        displayName: user.displayName,
        email: user.email ?? "",
        password: "",
        departmentId: user.departmentId ?? "",
        enabled: user.enabled,
        roleCodes: user.roles.map((role) => role.code),
      });
    }
  }, [mode, user]);
  function toggleRole(code: string) {
    setDraft((current) => ({
      ...current,
      roleCodes: current.roleCodes.includes(code)
        ? current.roleCodes.filter((item) => item !== code)
        : [...current.roleCodes, code],
    }));
  }
  return (
    <EditorForm
      disabled={disabled}
      saveLabel="保存用户"
      onSave={() =>
        onSave({
          ...draft,
          email: draft.email.trim() ? draft.email.trim() : null,
          departmentId: draft.departmentId || null,
          password: draft.password.trim() ? draft.password : null,
        })
      }
    >
      <Field label="用户名">
        <input
          value={draft.username}
          onChange={(e) => setDraft({ ...draft, username: e.target.value })}
        />
      </Field>
      <Field label="显示名称">
        <input
          value={draft.displayName}
          onChange={(e) => setDraft({ ...draft, displayName: e.target.value })}
        />
      </Field>
      <Field label="邮箱">
        <input
          autoComplete="email"
          value={draft.email}
          onChange={(e) => setDraft({ ...draft, email: e.target.value })}
        />
      </Field>
      <Field label={draft.id ? "新密码" : "初始密码"}>
        <input
          value={draft.password}
          onChange={(e) => setDraft({ ...draft, password: e.target.value })}
          type="password"
        />
      </Field>
      <Field label="所属部门">
        <select
          value={draft.departmentId}
          onChange={(e) => setDraft({ ...draft, departmentId: e.target.value })}
        >
          <option value="">不绑定部门</option>
          {departments.map((department) => (
            <option key={department.id} value={department.id}>
              {department.name}
            </option>
          ))}
        </select>
      </Field>
      <div className="role-checks">
        {roles.map((role) => (
          <label key={role.code}>
            <input
              checked={draft.roleCodes.includes(role.code)}
              onChange={() => toggleRole(role.code)}
              type="checkbox"
            />
            <span>{role.name}</span>
          </label>
        ))}
      </div>
      <label className="checkbox-field">
        <input
          checked={draft.enabled}
          onChange={(e) => setDraft({ ...draft, enabled: e.target.checked })}
          type="checkbox"
        />
        启用
      </label>
    </EditorForm>
  );
}

function ChangePasswordEditor({
  disabled,
  onSave,
}: {
  disabled: boolean;
  onSave: (request: { oldPassword: string; newPassword: string }) => Promise<void>;
}) {
  const [oldPassword, setOldPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  return (
    <EditorForm
      disabled={disabled || !oldPassword || !newPassword}
      saveLabel="修改密码"
      onSave={() => onSave({ oldPassword, newPassword })}
    >
      <Field label="旧密码">
        <input
          autoFocus
          type="password"
          value={oldPassword}
          onChange={(event) => setOldPassword(event.target.value)}
        />
      </Field>
      <Field label="新密码">
        <input
          type="password"
          value={newPassword}
          onChange={(event) => setNewPassword(event.target.value)}
        />
      </Field>
    </EditorForm>
  );
}

function PasswordResetEditor({
  disabled,
  i18n,
  onRequestCode,
  onReset,
}: {
  disabled: boolean;
  i18n: I18n;
  onRequestCode: (username: string) => Promise<void>;
  onReset: (request: {
    username: string;
    code: string;
    newPassword: string;
  }) => Promise<void>;
}) {
  const [username, setUsername] = useState("");
  const [code, setCode] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const canRequestCode = username.trim().length > 0;
  const canReset =
    username.trim().length > 0 && code.length === 6 && newPassword.length >= 6;

  return (
    <EditorForm
      contentClassName="password-reset-grid"
      disabled={disabled || !canReset}
      saveLabel={i18n.t("login.resetPassword")}
      onSave={() =>
        onReset({
          username: username.trim(),
          code,
          newPassword,
        })
      }
    >
      <p className="editor-form-note">{i18n.t("login.resetHint")}</p>
      <Field label={i18n.t("login.username")}>
        <input
          autoComplete="username"
          autoFocus
          disabled={disabled}
          value={username}
          onChange={(event) => setUsername(event.target.value)}
        />
      </Field>
      <div className="editor-inline-actions">
        <button
          className="ghost-button"
          disabled={disabled || !canRequestCode}
          onClick={() => void onRequestCode(username.trim())}
          type="button"
        >
          {i18n.t("login.sendCode")}
        </button>
      </div>
      <Field label={i18n.t("login.resetCode")}>
        <input
          autoComplete="one-time-code"
          disabled={disabled}
          inputMode="numeric"
          maxLength={12}
          value={code}
          onChange={(event) =>
            setCode(event.target.value.replace(/\D/g, "").slice(0, 6))
          }
        />
      </Field>
      <Field label={i18n.t("login.newPassword")}>
        <input
          autoComplete="new-password"
          disabled={disabled}
          type="password"
          value={newPassword}
          onChange={(event) => setNewPassword(event.target.value)}
        />
      </Field>
    </EditorForm>
  );
}

function BusinessSettingsEditor({
  disabled,
  localDirectoriesOnly = false,
  onSave,
  settings,
}: {
  disabled: boolean;
  localDirectoriesOnly?: boolean;
  onSave: (request: SystemSettings) => Promise<void>;
  settings: SystemSettings | null;
}) {
  const [draft, setDraft] = useState<SystemSettings>(
    settings ?? {
      hotelName: "",
      currentPeriod: currentMonthString(),
      defaultMonth: currentMonthString(),
      allowNegativeStock: false,
      quantityDecimals: 2,
      amountDecimals: 2,
      defaultExportDir: "",
      defaultBackupDir: "",
      autoBackupEnabled: true,
      intervalBackupEnabled: false,
      intervalBackupHours: 24,
      smtpEnabled: false,
      smtpHost: "",
      smtpPort: 465,
      smtpUsername: "",
      smtpPassword: "",
      smtpFromEmail: "",
      smtpFromName: "Aster",
      smtpPasswordConfigured: false,
    },
  );

  useEffect(() => {
    if (settings) setDraft(settings);
  }, [settings]);

  async function selectDirectory(
    title: string,
    key: "defaultExportDir" | "defaultBackupDir",
  ) {
    const selected = await chooseSinglePath({
      title,
      directory: true,
      defaultPath: draft[key] || undefined,
      canCreateDirectories: true,
    });
    if (selected) setDraft({ ...draft, [key]: selected });
  }

  return (
    <EditorForm
      disabled={disabled || (!localDirectoriesOnly && !draft.hotelName.trim())}
      saveLabel={localDirectoriesOnly ? "保存本机目录" : "保存系统设置"}
      onSave={() => onSave(draft)}
    >
      {localDirectoriesOnly ? (
        <div className="notice-banner">
          客户端模式下业务参数由主机统一控制，这里只保存当前电脑的导出和备份目录。
        </div>
      ) : null}
      <Field label="酒店名称">
        <input
          autoFocus
          disabled={localDirectoriesOnly}
          value={draft.hotelName}
          onChange={(event) => setDraft({ ...draft, hotelName: event.target.value })}
        />
      </Field>
      <Field label="当前账期">
        <MonthSelect
          disabled={localDirectoriesOnly}
          value={draft.currentPeriod}
          onChange={(currentPeriod) => setDraft({ ...draft, currentPeriod })}
        />
      </Field>
      <Field label="默认月份">
        <MonthSelect
          disabled={localDirectoriesOnly}
          value={draft.defaultMonth}
          onChange={(defaultMonth) => setDraft({ ...draft, defaultMonth })}
        />
      </Field>
      <Field label="数量小数位">
        <input
          max="6"
          min="0"
          disabled={localDirectoriesOnly}
          type="number"
          value={draft.quantityDecimals}
          onChange={(event) =>
            setDraft({ ...draft, quantityDecimals: Number(event.target.value) })
          }
        />
      </Field>
      <Field label="金额小数位">
        <input
          max="6"
          min="0"
          disabled={localDirectoriesOnly}
          type="number"
          value={draft.amountDecimals}
          onChange={(event) =>
            setDraft({ ...draft, amountDecimals: Number(event.target.value) })
          }
        />
      </Field>
      <Field label="定时备份小时">
        <input
          max="168"
          min="1"
          disabled={localDirectoriesOnly}
          type="number"
          value={draft.intervalBackupHours}
          onChange={(event) =>
            setDraft({ ...draft, intervalBackupHours: Number(event.target.value) })
          }
        />
      </Field>
      <Field label="默认导出目录">
        <PathPickerField
          value={draft.defaultExportDir}
          placeholder="请选择默认导出目录"
          buttonLabel="选择"
          onChoose={() => selectDirectory("选择默认导出目录", "defaultExportDir")}
        />
      </Field>
      <Field label="默认备份目录">
        <PathPickerField
          value={draft.defaultBackupDir}
          placeholder="请选择默认备份目录"
          buttonLabel="选择"
          onChoose={() => selectDirectory("选择默认备份目录", "defaultBackupDir")}
        />
      </Field>
      <label className="checkbox-field">
        <input
          checked={draft.allowNegativeStock}
          disabled={localDirectoriesOnly}
          onChange={(event) =>
            setDraft({ ...draft, allowNegativeStock: event.target.checked })
          }
          type="checkbox"
        />
        允许负库存
      </label>
      <label className="checkbox-field">
        <input
          checked={draft.autoBackupEnabled}
          disabled={localDirectoriesOnly}
          onChange={(event) =>
            setDraft({ ...draft, autoBackupEnabled: event.target.checked })
          }
          type="checkbox"
        />
        启动自动备份
      </label>
      <label className="checkbox-field">
        <input
          checked={draft.intervalBackupEnabled}
          disabled={localDirectoriesOnly}
          onChange={(event) =>
            setDraft({ ...draft, intervalBackupEnabled: event.target.checked })
          }
          type="checkbox"
        />
        运行中定时备份
      </label>
      <label className="checkbox-field">
        <input
          checked={draft.smtpEnabled}
          disabled={localDirectoriesOnly}
          onChange={(event) =>
            setDraft({ ...draft, smtpEnabled: event.target.checked })
          }
          type="checkbox"
        />
        启用邮箱验证码找回密码
      </label>
      <Field label="SMTP 主机">
        <input
          disabled={localDirectoriesOnly}
          placeholder="smtp.example.com"
          value={draft.smtpHost}
          onChange={(event) => setDraft({ ...draft, smtpHost: event.target.value })}
        />
      </Field>
      <Field label="SMTP 端口">
        <input
          max="65535"
          min="1"
          disabled={localDirectoriesOnly}
          type="number"
          value={draft.smtpPort}
          onChange={(event) =>
            setDraft({ ...draft, smtpPort: Number(event.target.value) })
          }
        />
      </Field>
      <Field label="SMTP 账号">
        <input
          autoComplete="username"
          disabled={localDirectoriesOnly}
          value={draft.smtpUsername}
          onChange={(event) =>
            setDraft({ ...draft, smtpUsername: event.target.value })
          }
        />
      </Field>
      <Field label="SMTP 授权码">
        <input
          autoComplete="new-password"
          disabled={localDirectoriesOnly}
          placeholder={draft.smtpPasswordConfigured ? "已配置，留空不修改" : ""}
          type="password"
          value={draft.smtpPassword ?? ""}
          onChange={(event) =>
            setDraft({ ...draft, smtpPassword: event.target.value })
          }
        />
      </Field>
      <Field label="发件邮箱">
        <input
          autoComplete="email"
          disabled={localDirectoriesOnly}
          value={draft.smtpFromEmail}
          onChange={(event) =>
            setDraft({ ...draft, smtpFromEmail: event.target.value })
          }
        />
      </Field>
      <Field label="发件名称">
        <input
          disabled={localDirectoriesOnly}
          value={draft.smtpFromName}
          onChange={(event) =>
            setDraft({ ...draft, smtpFromName: event.target.value })
          }
        />
      </Field>
    </EditorForm>
  );
}

function ClientConnectionEditor({
  disabled,
  onDiscover,
  onSave,
  onTest,
  status,
}: {
  disabled: boolean;
  onDiscover: (hostPort: number) => Promise<HostDiscoveryResult[]>;
  onSave: (hostAddress: string, hostPort: number) => Promise<void>;
  onTest: (
    hostAddress: string,
    hostPort: number,
  ) => Promise<HostConnectionTestResult>;
  status: AppStatus | null;
}) {
  const [hostAddress, setHostAddress] = useState(
    status?.runtime.hostAddress ?? "127.0.0.1",
  );
  const [hostPort, setHostPort] = useState(status?.runtime.hostPort ?? 17871);
  const [testResult, setTestResult] = useState<HostConnectionTestResult | null>(null);
  const [hosts, setHosts] = useState<HostDiscoveryResult[]>([]);

  return (
    <EditorForm
      disabled={disabled || !hostAddress.trim()}
      saveLabel="保存客户端连接"
      onSave={() => onSave(hostAddress, hostPort)}
    >
      <Field label="主机地址">
        <input
          autoFocus
          value={hostAddress}
          onChange={(event) => setHostAddress(event.target.value)}
          placeholder="主机 IP 或主机名"
        />
      </Field>
      <Field label="主机端口">
        <input
          max="65535"
          min="1024"
          type="number"
          value={hostPort}
          onChange={(event) => setHostPort(Number(event.target.value))}
        />
      </Field>
      <button
        className="ghost-button"
        disabled={disabled}
        type="button"
        onClick={async () => setTestResult(await onTest(hostAddress, hostPort))}
      >
        测试连接
      </button>
      <button
        className="ghost-button"
        disabled={disabled}
        type="button"
        onClick={async () => setHosts(await onDiscover(hostPort))}
      >
        发现主机
      </button>
      {testResult ? (
        <div className={testResult.ok ? "settings-result success" : "settings-result warning"}>
          <strong>{testResult.message}</strong>
          <span>{testResult.appName ?? "-"} {testResult.appVersion ?? ""}</span>
          <span>Schema：{testResult.schemaVersion ?? "-"}</span>
        </div>
      ) : null}
      {hosts.length > 0 ? (
        <div className="discovery-list">
          {hosts.map((host) => (
            <button
              className="discovery-item"
              key={`${host.hostAddress}:${host.hostPort}`}
              type="button"
              onClick={() => {
                setHostAddress(host.hostAddress);
                setHostPort(host.hostPort);
              }}
            >
              <strong>{host.hostAddress}:{host.hostPort}</strong>
              <span>{host.appName} {host.appVersion} · Schema {host.schemaVersion}</span>
            </button>
          ))}
        </div>
      ) : null}
    </EditorForm>
  );
}

function ClientPairingEditor({
  disabled,
  onSave,
  status,
}: {
  disabled: boolean;
  onSave: (request: {
    pairCode: string;
    clientName: string;
    clientDeviceId: string;
  }) => Promise<void>;
  status: AppStatus | null;
}) {
  const [pairCode, setPairCode] = useState("");
  const [clientName, setClientName] = useState("Aster 客户端");
  const [clientDeviceId, setClientDeviceId] = useState(
    status?.runtime.clientDeviceId ?? "",
  );
  return (
    <EditorForm
      disabled={disabled || pairCode.length !== 12 || !clientName.trim()}
      saveLabel="完成配对"
      onSave={() => onSave({ pairCode, clientName, clientDeviceId })}
    >
      <Field label="配对码">
        <input
          autoFocus
          inputMode="numeric"
          maxLength={12}
          value={pairCode}
          onChange={(event) => setPairCode(event.target.value)}
        />
      </Field>
      <Field label="客户端名称">
        <input
          value={clientName}
          onChange={(event) => setClientName(event.target.value)}
        />
      </Field>
      <Field label="设备 ID">
        <input
          value={clientDeviceId}
          onChange={(event) => setClientDeviceId(event.target.value)}
        />
      </Field>
    </EditorForm>
  );
}

function SecondBackupDirEditor({
  disabled,
  onSave,
  status,
}: {
  disabled: boolean;
  onSave: (path: string) => Promise<void>;
  status: AppStatus | null;
}) {
  const [path, setPath] = useState(status?.runtime.backupDir ?? "");
  async function selectDirectory() {
    const selected = await chooseSinglePath({
      title: "选择第二备份目录",
      directory: true,
      defaultPath: path || undefined,
      canCreateDirectories: true,
    });
    if (selected) setPath(selected);
  }
  return (
    <EditorForm
      disabled={disabled || !path}
      saveLabel="保存目录"
      onSave={() => onSave(path)}
    >
      <Field label="第二备份目录">
        <PathPickerField
          value={path}
          placeholder="请选择外接硬盘或同步盘目录"
          buttonLabel="选择"
          onChoose={selectDirectory}
        />
      </Field>
    </EditorForm>
  );
}

function RestoreBackupEditor({
  disabled,
  onPreview,
  onRestore,
  status,
}: {
  disabled: boolean;
  onPreview: (backupFile: string) => Promise<RestorePreview>;
  onRestore: (request: {
    backupFile: string;
    confirmation: string;
    validationToken: string;
  }) => Promise<void>;
  status: AppStatus | null;
}) {
  const [backupFile, setBackupFile] = useState("");
  const [confirmation, setConfirmation] = useState("");
  const [preview, setPreview] = useState<RestorePreview | null>(null);
  async function selectFile() {
    const selected = await chooseSinglePath({
      title: "选择备份文件",
      filters: [{ name: "Aster 备份文件", extensions: ["zip"] }],
      defaultPath: backupFile || status?.runtime.backupDir || undefined,
    });
    if (selected) {
      setBackupFile(selected);
      setPreview(null);
    }
  }
  return (
    <EditorForm
      disabled={
        disabled ||
        !preview ||
        preview.backupFile !== backupFile ||
        !preview.validationToken ||
        confirmation !== "RESTORE"
      }
      saveLabel="恢复备份"
      onSave={() =>
        onRestore({
          backupFile,
          confirmation,
          validationToken: preview?.validationToken ?? "",
        })
      }
    >
      <Field label="备份文件">
        <PathPickerField
          value={backupFile}
          placeholder="请选择 aster-backup-YYYYMMDD-HHMMSS-短ID.zip"
          buttonLabel="选择"
          onChoose={selectFile}
        />
      </Field>
      <button
        className="ghost-button"
        disabled={disabled || !backupFile}
        type="button"
        onClick={async () => setPreview(await onPreview(backupFile))}
      >
        校验备份包
      </button>
      <Field label="确认文本">
        <input
          value={confirmation}
          onChange={(event) => setConfirmation(event.target.value)}
          placeholder="输入 RESTORE"
        />
      </Field>
      {preview ? (
        <div className="settings-result">
          <strong>{preview.message}</strong>
          <span>创建时间：{formatDateTime(preview.metadata.createdAt)}</span>
          <span>Schema：v{preview.metadata.schemaVersion}</span>
          <span>来源主机：{preview.metadata.sourceHostName ?? "-"}</span>
          <span>校验：{preview.metadata.databaseSha256}</span>
        </div>
      ) : null}
    </EditorForm>
  );
}

function StockDocumentEditor({
  balances,
  currentUser,
  departments,
  disabled,
  documentType,
  items,
  onCreateApproval,
  onSaveDraft,
  onSubmit,
  suppliers,
}: {
  balances: StockBalanceRow[];
  currentUser: CurrentUser | null;
  departments: Department[];
  disabled: boolean;
  documentType: "inbound" | "outbound";
  items: Item[];
  onCreateApproval: (request: CreateApprovalRequestDraft) => Promise<void>;
  onSaveDraft: (request: StockDocumentDraft) => Promise<void>;
  onSubmit: (request: StockDocumentDraft) => Promise<void>;
  suppliers: Supplier[];
}) {
  const defaultHandler = userDisplayName(currentUser);
  const emptyLine: StockDocumentLineDraft = {
    itemId: "",
    quantity: 1,
    unitPrice: 0,
    amount: null,
    purchaseUnitPrice: null,
    purchaseAmount: null,
    saleUnitPrice: null,
    saleAmount: null,
    costUnitPrice: null,
    costAmount: null,
    remark: "",
  };
  const [draft, setDraft] = useState<StockDocumentDraft>({
    documentId: undefined,
    documentType,
    outboundKind: documentType === "outbound" ? "internal" : undefined,
    businessDate: currentDateTimeString(),
    departmentId: "",
    supplierId: "",
    handler: defaultHandler,
    purpose: "",
    remark: "",
    approvalRequestId: "",
    lines: [emptyLine],
  });
  const [scanCode, setScanCode] = useState("");
  const isOutbound = documentType === "outbound";
  const isInternalOutbound =
    isOutbound && (draft.outboundKind ?? "internal") === "internal";
  const currentOutboundKind = draft.outboundKind ?? "internal";
  const totalAmount = draft.lines.reduce(
    (sum, line) =>
      sum + effectiveDraftAmount(line, documentType, currentOutboundKind),
    0,
  );
  const balanceByItemId = useMemo(
    () => new Map(balances.map((balance) => [balance.itemId, balance])),
    [balances],
  );

  useEffect(() => {
    if (!defaultHandler) return;
    setDraft((current) =>
      current.handler?.trim() ? current : { ...current, handler: defaultHandler },
    );
  }, [defaultHandler]);

  function availableStockInfo(itemId: string) {
    if (!itemId) return { label: "-", empty: true };
    const balance = balanceByItemId.get(itemId);
    if (!balance) return { label: "0", empty: true };
    return {
      label: `${balance.quantity} ${balance.unitName ?? ""}`.trim(),
      empty: balance.quantity <= 0,
    };
  }

  function updateLine(
    index: number,
    nextLine: Partial<StockDocumentLineDraft>,
  ) {
    setDraft((current) => ({
      ...current,
      lines: current.lines.map((line, lineIndex) => {
        if (lineIndex !== index) return line;
        const updated = { ...line, ...nextLine };
        if (nextLine.itemId) {
          const item = items.find((record) => record.id === nextLine.itemId);
          const nextPrice =
            documentType === "inbound"
              ? (item?.defaultPrice ?? updated.unitPrice)
              : currentOutboundKind === "guest_sale"
                ? (item?.salePrice ?? updated.unitPrice)
                : 0;
          updated.unitPrice = nextPrice;
          updated.purchaseUnitPrice =
            documentType === "inbound" ? nextPrice : null;
          updated.saleUnitPrice =
            documentType === "outbound" && currentOutboundKind === "guest_sale"
              ? nextPrice
              : null;
          updated.amount = null;
          updated.purchaseAmount = null;
          updated.saleAmount = null;
          updated.costUnitPrice = null;
          updated.costAmount = null;
        }
        return updated;
      }),
    }));
  }

  function addLine(line: StockDocumentLineDraft = emptyLine) {
    setDraft((current) => ({ ...current, lines: [...current.lines, line] }));
  }

  function removeLine(index: number) {
    setDraft((current) => {
      const lines = current.lines.filter((_, lineIndex) => lineIndex !== index);
      return { ...current, lines: lines.length ? lines : [emptyLine] };
    });
  }

  function applyScannedCode(rawCode: string) {
    const code = rawCode.trim();
    if (!code) return;
    const item = items.find(
      (record) =>
        record.barcode === code || record.code === code || record.name === code,
    );
    if (!item) return;
    const nextLine = {
      itemId: item.id,
      quantity: 1,
      unitPrice:
        documentType === "inbound"
          ? item.defaultPrice
          : currentOutboundKind === "guest_sale"
            ? item.salePrice
            : 0,
      amount: null,
      purchaseUnitPrice: documentType === "inbound" ? item.defaultPrice : null,
      purchaseAmount: null,
      saleUnitPrice:
        documentType === "outbound" && currentOutboundKind === "guest_sale"
          ? item.salePrice
          : null,
      saleAmount: null,
      costUnitPrice: null,
      costAmount: null,
      remark: "",
    };
    setDraft((current) => {
      const emptyIndex = current.lines.findIndex((line) => !line.itemId);
      if (emptyIndex >= 0) {
        return {
          ...current,
          lines: current.lines.map((line, index) =>
            index === emptyIndex ? nextLine : line,
          ),
        };
      }
      return { ...current, lines: [...current.lines, nextLine] };
    });
    setScanCode("");
  }

  return (
    <div className="editor-document document-entry-editor">
      <div className="editor-form-grid">
        <Field label="业务日期">
          <input
            type="datetime-local"
            step={1}
            value={draft.businessDate}
            onChange={(e) =>
              setDraft({ ...draft, businessDate: e.target.value })
            }
          />
        </Field>
        {isOutbound ? (
          <Field label="出库类型">
            <select
              value={draft.outboundKind ?? "internal"}
              onChange={(e) => {
                const nextOutboundKind = e.target.value as
                  | "internal"
                  | "guest_sale";
                setDraft({
                  ...draft,
                  outboundKind: nextOutboundKind,
                  departmentId:
                    nextOutboundKind === "guest_sale" ? "" : draft.departmentId,
                  approvalRequestId:
                    nextOutboundKind === "guest_sale"
                      ? ""
                      : draft.approvalRequestId,
                  lines: draft.lines.map((line) => {
                    const item = items.find((record) => record.id === line.itemId);
                    if (nextOutboundKind === "guest_sale") {
                      const saleUnitPrice = item?.salePrice ?? line.saleUnitPrice ?? 0;
                      return {
                        ...line,
                        unitPrice: saleUnitPrice,
                        amount: null,
                        saleUnitPrice,
                        saleAmount: null,
                        costUnitPrice: null,
                        costAmount: null,
                      };
                    }
                    return {
                      ...line,
                      unitPrice: 0,
                      amount: null,
                      saleUnitPrice: null,
                      saleAmount: null,
                      costUnitPrice: null,
                      costAmount: null,
                    };
                  }),
                });
              }}
            >
              <option value="internal">内部员工领用</option>
              <option value="guest_sale">酒店客人销售</option>
            </select>
          </Field>
        ) : null}
        {isInternalOutbound ? (
          <Field label="领用部门">
            <select
              value={draft.departmentId}
              onChange={(e) =>
                setDraft({ ...draft, departmentId: e.target.value })
              }
            >
              <option value="">请选择部门</option>
              {departments.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.name}
                </option>
              ))}
            </select>
          </Field>
        ) : isOutbound ? (
          <Field label="销售对象">
            <input disabled value="酒店客人" />
          </Field>
        ) : (
          <Field label="供应商">
            <select
              value={draft.supplierId}
              onChange={(e) =>
                setDraft({ ...draft, supplierId: e.target.value })
              }
            >
              <option value="">未设置</option>
              {suppliers.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.name}
                </option>
              ))}
            </select>
          </Field>
        )}
        <Field label="经办人">
          <input
            readOnly
            disabled
            value={draft.handler}
          />
        </Field>
        <Field label={isOutbound ? "用途" : "备注"}>
          <input
            value={isOutbound ? draft.purpose : draft.remark}
            onChange={(e) =>
              isOutbound
                ? setDraft({ ...draft, purpose: e.target.value })
                : setDraft({ ...draft, remark: e.target.value })
            }
          />
        </Field>
        {isInternalOutbound ? (
          <Field label="审批单 ID">
            <input
              value={draft.approvalRequestId}
              onChange={(e) =>
                setDraft({ ...draft, approvalRequestId: e.target.value })
              }
              placeholder="超预算审批通过后填写"
            />
          </Field>
        ) : null}
      </div>

      <div className="editor-toolbar">
        <form
          onSubmit={(e) => {
            e.preventDefault();
            applyScannedCode(scanCode);
          }}
        >
          <input
            placeholder="扫码或输入条码/编码"
            value={scanCode}
            onChange={(e) => setScanCode(e.target.value)}
          />
          <button className="ghost-button" disabled={disabled}>
            加入
          </button>
        </form>
        <button
          className="primary-button"
          disabled={disabled}
          onClick={() => addLine()}
        >
          新增一行
        </button>
      </div>

      <div className="editor-table-scroll">
        <table>
          <thead>
            <tr>
              <th>物品</th>
              {isOutbound ? <th>可用库存</th> : null}
              <th>数量</th>
              {isInternalOutbound ? null : (
                <>
                  <th>{isOutbound ? "销售单价" : "本次进价"}</th>
                  <th>{isOutbound ? "销售金额" : "采购金额"}</th>
                </>
              )}
              {isInternalOutbound ? <th>成本核算</th> : null}
              <th>备注</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            {draft.lines.map((line, index) => (
              <tr key={index}>
                <td>
                  <ItemSearchSelect
                    disabled={disabled}
                    items={items}
                    value={line.itemId}
                    onChange={(itemId) => updateLine(index, { itemId })}
                  />
                </td>
                {isOutbound ? (
                  <td>
                    <span
                      className={
                        availableStockInfo(line.itemId).empty
                          ? "available-stock empty"
                          : "available-stock"
                      }
                    >
                      {availableStockInfo(line.itemId).label}
                    </span>
                  </td>
                ) : null}
                <td>
                  <input
                    className="table-input"
                    min="0"
                    type="number"
                    value={line.quantity}
                    onChange={(e) =>
                      updateLine(index, { quantity: Number(e.target.value) })
                    }
                  />
                </td>
                {isInternalOutbound ? (
                  <td>
                    <span className="muted-inline">提交后按 FIFO 批次成本计算</span>
                  </td>
                ) : (
                  <>
                    <td>
                      <input
                        className="table-input"
                        min="0"
                        type="number"
                        value={
                          isOutbound
                            ? (line.saleUnitPrice ?? line.unitPrice)
                            : (line.purchaseUnitPrice ?? line.unitPrice)
                        }
                        onChange={(e) => {
                          const value = Number(e.target.value);
                          updateLine(
                            index,
                            isOutbound
                              ? { unitPrice: value, saleUnitPrice: value }
                              : { unitPrice: value, purchaseUnitPrice: value },
                          );
                        }}
                      />
                    </td>
                    <td>
                      <input
                        className="table-input"
                        min="0"
                        placeholder={formatMoney(
                          line.quantity *
                            (isOutbound
                              ? (line.saleUnitPrice ?? line.unitPrice)
                              : (line.purchaseUnitPrice ?? line.unitPrice)),
                        )}
                        type="number"
                        value={
                          isOutbound
                            ? (line.saleAmount ?? "")
                            : (line.purchaseAmount ?? line.amount ?? "")
                        }
                        onChange={(e) => {
                          const value =
                            e.target.value === "" ? null : Number(e.target.value);
                          updateLine(
                            index,
                            isOutbound
                              ? { amount: value, saleAmount: value }
                              : { amount: value, purchaseAmount: value },
                          );
                        }}
                      />
                    </td>
                  </>
                )}
                <td>
                  <input
                    className="table-input"
                    value={line.remark}
                    onChange={(e) =>
                      updateLine(index, { remark: e.target.value })
                    }
                  />
                </td>
                <td className="row-actions">
                  <button disabled={disabled} onClick={() => removeLine(index)}>
                    删除
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="editor-actions">
        <strong>
          {isOutbound
            ? currentOutboundKind === "guest_sale"
              ? "销售合计"
              : "预计成本"
            : "采购合计"}
          ：{formatMoney(totalAmount)} 元
        </strong>
        {isInternalOutbound ? (
          <button
            className="ghost-button"
            disabled={disabled || !draft.departmentId || !draft.businessDate}
            onClick={() =>
              onCreateApproval({
                entityType: "budget_override",
                entityId: `${draft.departmentId}:${draft.businessDate.slice(0, 7)}`,
                reason: `申请 ${draft.businessDate.slice(0, 7)} ${optionName(departments, draft.departmentId)} 超预算领用，预计金额 ${formatMoney(totalAmount)} 元`,
              })
            }
          >
            申请超预算审批
          </button>
        ) : null}
        <button
          className="ghost-button"
          disabled={disabled}
          onClick={() => onSaveDraft(draft)}
        >
          保存草稿
        </button>
        <button
          className="primary-button"
          disabled={disabled}
          onClick={() => onSubmit(draft)}
        >
          确认提交
        </button>
      </div>
    </div>
  );
}

function AdjustmentEditor({
  currentUser,
  disabled,
  items,
  onSubmit,
}: {
  currentUser: CurrentUser | null;
  disabled: boolean;
  items: Item[];
  onSubmit: (request: AdjustmentDraft) => Promise<void>;
}) {
  const defaultHandler = userDisplayName(currentUser);
  const emptyLine: AdjustmentLineDraft = {
    itemId: "",
    direction: "out",
    quantity: 1,
    unitPrice: 0,
    amount: null,
    remark: "",
  };
  const [draft, setDraft] = useState<AdjustmentDraft>({
    businessDate: currentDateTimeString(),
    adjustmentType: "damage",
    handler: defaultHandler,
    reason: "",
    lines: [emptyLine],
  });
  const totalAmount = draft.lines.reduce(
    (sum, line) => sum + effectiveLineAmount(line),
    0,
  );
  const correction = draft.adjustmentType === "correction";

  useEffect(() => {
    if (!defaultHandler) return;
    setDraft((current) =>
      current.handler?.trim() ? current : { ...current, handler: defaultHandler },
    );
  }, [defaultHandler]);

  function updateAdjustmentType(type: AdjustmentDraft["adjustmentType"]) {
    const direction = type === "gain" ? "in" : "out";
    setDraft((current) => ({
      ...current,
      adjustmentType: type,
      lines: current.lines.map((line) => ({
        ...line,
        direction: type === "correction" ? line.direction : direction,
      })),
    }));
  }

  function updateLine(index: number, nextLine: Partial<AdjustmentLineDraft>) {
    setDraft((current) => ({
      ...current,
      lines: current.lines.map((line, lineIndex) => {
        if (lineIndex !== index) return line;
        const updated = { ...line, ...nextLine };
        if (nextLine.itemId) {
          const item = items.find((record) => record.id === nextLine.itemId);
          updated.unitPrice = item?.defaultPrice ?? updated.unitPrice;
          updated.amount = null;
        }
        return updated;
      }),
    }));
  }

  function removeLine(index: number) {
    setDraft((current) => {
      const lines = current.lines.filter((_, lineIndex) => lineIndex !== index);
      return { ...current, lines: lines.length ? lines : [emptyLine] };
    });
  }

  return (
    <div className="editor-document document-entry-editor">
      <div className="editor-form-grid">
        <Field label="调整日期">
          <input
            type="datetime-local"
            step={1}
            value={draft.businessDate}
            onChange={(e) =>
              setDraft({ ...draft, businessDate: e.target.value })
            }
          />
        </Field>
        <Field label="调整类型">
          <select
            value={draft.adjustmentType}
            onChange={(e) =>
              updateAdjustmentType(
                e.target.value as AdjustmentDraft["adjustmentType"],
              )
            }
          >
            <option value="gain">盘盈调整</option>
            <option value="loss">盘亏调整</option>
            <option value="damage">损耗调整</option>
            <option value="correction">数据修正</option>
          </select>
        </Field>
        <Field label="经办人">
          <input
            readOnly
            disabled
            value={draft.handler}
          />
        </Field>
        <Field label="调整原因">
          <input
            value={draft.reason}
            onChange={(e) => setDraft({ ...draft, reason: e.target.value })}
          />
        </Field>
      </div>
      <div className="editor-toolbar">
        <h2>调整明细</h2>
        <button
          className="primary-button"
          disabled={disabled}
          onClick={() =>
            setDraft({ ...draft, lines: [...draft.lines, emptyLine] })
          }
        >
          新增一行
        </button>
      </div>
      <div className="editor-table-scroll">
        <table>
          <thead>
            <tr>
              <th>物品</th>
              <th>方向</th>
              <th>数量</th>
              <th>成本单价</th>
              <th>金额</th>
              <th>备注</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            {draft.lines.map((line, index) => (
              <tr key={index}>
                <td>
                  <ItemSearchSelect
                    disabled={disabled}
                    items={items}
                    value={line.itemId}
                    onChange={(itemId) => updateLine(index, { itemId })}
                  />
                </td>
                <td>
                  <select
                    className="table-input compact-input"
                    disabled={!correction}
                    value={line.direction}
                    onChange={(e) =>
                      updateLine(index, {
                        direction: e.target.value as "in" | "out",
                      })
                    }
                  >
                    <option value="in">增加</option>
                    <option value="out">减少</option>
                  </select>
                </td>
                <td>
                  <input
                    className="table-input compact-input"
                    min="0"
                    type="number"
                    value={line.quantity}
                    onChange={(e) =>
                      updateLine(index, { quantity: Number(e.target.value) })
                    }
                  />
                </td>
                <td>
                  <input
                    className="table-input compact-input"
                    min="0"
                    type="number"
                    value={line.unitPrice}
                    onChange={(e) =>
                      updateLine(index, { unitPrice: Number(e.target.value) })
                    }
                  />
                </td>
                <td>
                  <input
                    className="table-input compact-input"
                    min="0"
                    placeholder={formatMoney(line.quantity * line.unitPrice)}
                    type="number"
                    value={line.amount ?? ""}
                    onChange={(e) =>
                      updateLine(index, {
                        amount:
                          e.target.value === "" ? null : Number(e.target.value),
                      })
                    }
                  />
                </td>
                <td>
                  <input
                    className="table-input"
                    value={line.remark}
                    onChange={(e) =>
                      updateLine(index, { remark: e.target.value })
                    }
                  />
                </td>
                <td className="row-actions">
                  <button disabled={disabled} onClick={() => removeLine(index)}>
                    删除
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="editor-actions">
        <strong>合计金额：{formatMoney(totalAmount)} 元</strong>
        <button
          className="primary-button"
          disabled={disabled}
          onClick={() => onSubmit(draft)}
        >
          确认调整
        </button>
      </div>
    </div>
  );
}

function StocktakeCreateEditor({
  categories,
  disabled,
  items,
  onCreate,
}: {
  categories: OptionRecord[];
  disabled: boolean;
  items: Item[];
  onCreate: (request: {
    businessDate: string;
    scopeType: string;
    categoryId?: string | null;
    itemIds: string[];
    handler?: string | null;
    remark?: string | null;
  }) => Promise<void>;
}) {
  const [businessDate, setBusinessDate] = useState(currentDateTimeString());
  const [scopeType, setScopeType] = useState<"all" | "category" | "custom">(
    "all",
  );
  const [categoryId, setCategoryId] = useState("");
  const [selectedItemId, setSelectedItemId] = useState("");
  const [customItemIds, setCustomItemIds] = useState<string[]>([]);
  const [handler, setHandler] = useState("");
  const [remark, setRemark] = useState("");

  function addCustomItem() {
    if (selectedItemId && !customItemIds.includes(selectedItemId)) {
      setCustomItemIds([...customItemIds, selectedItemId]);
    }
    setSelectedItemId("");
  }

  return (
    <EditorForm
      disabled={disabled}
      saveLabel="创建盘点单"
      onSave={() =>
        onCreate({
          businessDate,
          scopeType,
          categoryId,
          itemIds: customItemIds,
          handler,
          remark,
        })
      }
    >
      <Field label="盘点日期">
        <input
          type="datetime-local"
          step={1}
          value={businessDate}
          onChange={(e) => setBusinessDate(e.target.value)}
        />
      </Field>
      <Field label="盘点范围">
        <select
          value={scopeType}
          onChange={(e) =>
            setScopeType(e.target.value as "all" | "category" | "custom")
          }
        >
          <option value="all">全部物品</option>
          <option value="category">按分类</option>
          <option value="custom">自定义物品</option>
        </select>
      </Field>
      {scopeType === "category" ? (
        <Field label="分类">
          <select
            value={categoryId}
            onChange={(e) => setCategoryId(e.target.value)}
          >
            <option value="">请选择分类</option>
            {categories.map((item) => (
              <option key={item.id} value={item.id}>
                {item.name}
              </option>
            ))}
          </select>
        </Field>
      ) : null}
      {scopeType === "custom" ? (
        <div className="custom-picker">
          <Field label="物品">
            <select
              value={selectedItemId}
              onChange={(e) => setSelectedItemId(e.target.value)}
            >
              <option value="">请选择物品</option>
              {items.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.code} · {item.name}
                </option>
              ))}
            </select>
          </Field>
          <button
            className="ghost-button"
            disabled={disabled}
            onClick={addCustomItem}
          >
            加入
          </button>
          <div className="selected-tags">
            {customItemIds.map((selectedId) => {
              const item = items.find((record) => record.id === selectedId);
              return (
                <button
                  key={selectedId}
                  onClick={() =>
                    setCustomItemIds(
                      customItemIds.filter((itemId) => itemId !== selectedId),
                    )
                  }
                >
                  {item?.name ?? selectedId} x
                </button>
              );
            })}
          </div>
        </div>
      ) : null}
      <Field label="经办人">
        <input value={handler} onChange={(e) => setHandler(e.target.value)} />
      </Field>
      <Field label="备注">
        <input value={remark} onChange={(e) => setRemark(e.target.value)} />
      </Field>
    </EditorForm>
  );
}

function StocktakeCountsEditor({
  detail,
  disabled,
  onSaveCounts,
  onSelect,
  stocktakes,
}: {
  detail: StocktakeDetail | null;
  disabled: boolean;
  onSaveCounts: (
    stocktakeId: string,
    lines: {
      lineId: string;
      countedQuantity?: number | null;
      remark?: string | null;
    }[],
  ) => Promise<void>;
  onSelect: (stocktakeId: string) => Promise<void>;
  stocktakes: StocktakeDocument[];
}) {
  const [lineDrafts, setLineDrafts] = useState<
    Record<string, { countedQuantity: string; remark: string }>
  >({});

  useEffect(() => {
    const nextDrafts: Record<
      string,
      { countedQuantity: string; remark: string }
    > = {};
    for (const line of detail?.lines ?? []) {
      nextDrafts[line.id] = {
        countedQuantity:
          line.countedQuantity == null ? "" : String(line.countedQuantity),
        remark: line.remark ?? "",
      };
    }
    setLineDrafts(nextDrafts);
  }, [detail?.document.id]);

  function saveCounts() {
    if (!detail) return Promise.resolve();
    const lines = Object.entries(lineDrafts).map(([lineId, draft]) => ({
      lineId,
      countedQuantity:
        draft.countedQuantity.trim() === ""
          ? null
          : Number(draft.countedQuantity),
      remark: draft.remark,
    }));
    return onSaveCounts(detail.document.id, lines);
  }

  const canEdit =
    detail &&
    detail.document.status !== "confirmed" &&
    detail.document.status !== "voided";
  return (
    <div className="editor-document stocktake-counts-editor">
      <div className="editor-toolbar">
        <Field label="盘点单">
          <select
            value={detail?.document.id ?? ""}
            onChange={(e) => onSelect(e.target.value)}
          >
            {stocktakes.map((stocktake) => (
              <option key={stocktake.id} value={stocktake.id}>
                {stocktake.documentNo} · {formatDateTime(stocktake.businessDate)} ·{" "}
                {stocktakeStatusLabel(stocktake.status)}
              </option>
            ))}
          </select>
        </Field>
      </div>
      {detail ? (
        <section className="metrics-grid stocktake-metrics">
          <div className="metric-card">
            <span>盘点行数</span>
            <strong>{detail.document.lineCount}</strong>
            <em>行</em>
          </div>
          <div className="metric-card">
            <span>已录入</span>
            <strong>{detail.document.countedCount}</strong>
            <em>行</em>
          </div>
          <div className="metric-card">
            <span>状态</span>
            <strong>{stocktakeStatusLabel(detail.document.status)}</strong>
            <em>{stocktakeScopeLabel(detail.document.scopeType)}</em>
          </div>
          <div className="metric-card">
            <span>盘盈/盘亏</span>
            <strong>
              {formatMoney(detail.document.gainAmount)} /{" "}
              {formatMoney(detail.document.lossAmount)}
            </strong>
            <em>元</em>
          </div>
        </section>
      ) : null}
      <div className="editor-table-scroll">
        <table>
          <thead>
            <tr>
              <th>编码</th>
              <th>物品</th>
              <th>规格</th>
              <th>单位</th>
              <th>账面</th>
              <th>实盘</th>
              <th>差异</th>
              <th>差异金额</th>
              <th>备注</th>
            </tr>
          </thead>
          <tbody>
            {(detail?.lines ?? []).map((line) => {
              const draft = lineDrafts[line.id] ?? {
                countedQuantity: "",
                remark: "",
              };
              return (
                <tr key={line.id}>
                  <td>{line.itemCode}</td>
                  <td>{line.itemName}</td>
                  <td>{line.spec ?? "-"}</td>
                  <td>{line.unitName ?? "-"}</td>
                  <td>{line.bookQuantity}</td>
                  <td>
                    <input
                      className="table-input compact-input"
                      disabled={!canEdit || disabled}
                      min="0"
                      type="number"
                      value={draft.countedQuantity}
                      onChange={(e) =>
                        setLineDrafts({
                          ...lineDrafts,
                          [line.id]: {
                            ...draft,
                            countedQuantity: e.target.value,
                          },
                        })
                      }
                    />
                  </td>
                  <td
                    className={
                      line.differenceQuantity === 0
                        ? ""
                        : line.differenceQuantity > 0
                          ? "gain-text"
                          : "loss-text"
                    }
                  >
                    {line.differenceQuantity}
                  </td>
                  <td>{formatMoney(line.differenceAmount)}</td>
                  <td>
                    <input
                      className="table-input"
                      disabled={!canEdit || disabled}
                      value={draft.remark}
                      onChange={(e) =>
                        setLineDrafts({
                          ...lineDrafts,
                          [line.id]: { ...draft, remark: e.target.value },
                        })
                      }
                    />
                  </td>
                </tr>
              );
            })}
            {!detail || detail.lines.length === 0 ? (
              <EmptyRow colSpan={9} />
            ) : null}
          </tbody>
        </table>
      </div>
      <div className="editor-actions">
        <button
          className="primary-button"
          disabled={disabled || !canEdit}
          onClick={saveCounts}
        >
          保存实盘
        </button>
      </div>
    </div>
  );
}

function EditorForm({
  children,
  contentClassName,
  disabled,
  onSave,
  saveLabel,
}: {
  children: React.ReactNode;
  contentClassName?: string;
  disabled: boolean;
  onSave: () => Promise<void>;
  saveLabel: string;
}) {
  return (
    <div className="editor-form">
      <div
        className={
          contentClassName
            ? `editor-form-grid ${contentClassName}`
            : "editor-form-grid"
        }
      >
        {children}
      </div>
      <div className="editor-actions">
        <button
          className="ghost-button"
          disabled={disabled}
          onClick={() => void closeCurrentEditorWindow()}
          type="button"
        >
          取消
        </button>
        <button className="primary-button" disabled={disabled} onClick={onSave}>
          {saveLabel}
        </button>
      </div>
    </div>
  );
}

function StockDocumentPage({
  canWrite,
  departments,
  documentType,
  documents,
  handlerOptions,
  items,
  onConfirmDraft,
  onQueryChange,
  onVoid,
  query,
  suppliers,
}: {
  canWrite: boolean;
  departments: Department[];
  documentType: "inbound" | "outbound";
  documents: StockDocument[];
  handlerOptions: string[];
  items: Item[];
  onConfirmDraft: (
    documentId: string,
    approvalRequestId?: string | null,
  ) => Promise<void>;
  onQueryChange: (query: StockDocumentQuery) => Promise<void>;
  onVoid: (
    documentId: string,
    reason: string,
    handler: string,
  ) => Promise<void>;
  query: StockDocumentQuery;
  suppliers: Supplier[];
}) {
  const isOutbound = documentType === "outbound";
  const [approvalRequestId, setApprovalRequestId] = useState("");
  const [voidReason, setVoidReason] = useState("");
  const [voidHandler, setVoidHandler] = useState("");

  return (
    <section className="table-panel">
      <div className="table-toolbar document-action-toolbar">
        <DocumentVoidControls
          approvalRequestId={approvalRequestId}
          isOutbound={isOutbound}
          setApprovalRequestId={setApprovalRequestId}
          setVoidHandler={setVoidHandler}
          setVoidReason={setVoidReason}
          voidHandler={voidHandler}
          voidReason={voidReason}
        />
        <button
          className="primary-button"
          disabled={!canWrite}
          onClick={() =>
            openEditorWindow("stockDocument", {
              documentType,
              width: 980,
              height: 760,
            })
          }
        >
          {isOutbound ? "新建出库/领用单" : "新建入库单"}
        </button>
      </div>
      <DocumentList
        departments={departments}
        documents={documents}
        handlerOptions={handlerOptions}
        items={items}
        isOutbound={isOutbound}
        canVoid={canWrite}
        approvalRequestId={approvalRequestId}
        onConfirmDraft={onConfirmDraft}
        onQueryChange={onQueryChange}
        onVoid={onVoid}
        query={query}
        voidHandler={voidHandler}
        voidReason={voidReason}
        suppliers={suppliers}
      />
    </section>
  );
}

function DocumentVoidControls({
  approvalRequestId,
  isOutbound,
  setApprovalRequestId,
  setVoidHandler,
  setVoidReason,
  voidHandler,
  voidReason,
}: {
  approvalRequestId: string;
  isOutbound: boolean;
  setApprovalRequestId: (value: string) => void;
  setVoidHandler: (value: string) => void;
  setVoidReason: (value: string) => void;
  voidHandler: string;
  voidReason: string;
}) {
  return (
    <div className="void-controls">
      {isOutbound ? (
        <input
          placeholder="审批单 ID"
          value={approvalRequestId}
          onChange={(e) => setApprovalRequestId(e.target.value)}
        />
      ) : null}
      <input
        placeholder="作废原因"
        value={voidReason}
        onChange={(e) => setVoidReason(e.target.value)}
      />
      <input
        placeholder="经办人"
        value={voidHandler}
        onChange={(e) => setVoidHandler(e.target.value)}
      />
    </div>
  );
}

function DocumentList({
  approvalRequestId = "",
  canVoid = true,
  departments = [],
  documents,
  handlerOptions = [],
  items = [],
  isOutbound,
  onConfirmDraft,
  onQueryChange,
  onVoid,
  query,
  suppliers = [],
  title,
  voidHandler = "",
  voidReason = "",
}: {
  approvalRequestId?: string;
  canVoid?: boolean;
  departments?: Department[];
  documents: StockDocument[];
  handlerOptions?: string[];
  items?: Item[];
  isOutbound: boolean;
  onConfirmDraft?: (
    documentId: string,
    approvalRequestId?: string | null,
  ) => Promise<void>;
  onQueryChange?: (query: StockDocumentQuery) => Promise<void>;
  onVoid?: (
    documentId: string,
    reason: string,
    handler: string,
  ) => Promise<void>;
  query?: StockDocumentQuery;
  suppliers?: Supplier[];
  title?: string;
  voidHandler?: string;
  voidReason?: string;
}) {
  const [filterDraft, setFilterDraft] = useState<StockDocumentQuery>(
    query ?? {
      documentType: isOutbound ? "outbound" : "inbound",
      month: currentMonthString(),
    },
  );
  useEffect(() => {
    if (query) {
      setFilterDraft({
        ...query,
        month: query.month || currentMonthString(),
      });
    }
  }, [query]);

  const partyLabel = isOutbound ? "领用部门" : "供应商";
  const isAdjustment = filterDraft.documentType === "adjustment";
  const partyValue = isOutbound
    ? (filterDraft.departmentId ?? "")
    : (filterDraft.supplierId ?? "");
  const partyOptions = isOutbound ? departments : suppliers;
  const effectiveHandlerOptions = useMemo(
    () =>
      uniqueTextOptions([
        ...handlerOptions,
        ...documents.map((document) => document.handler),
      ]),
    [documents, handlerOptions],
  );

  function updateFilter(next: Partial<StockDocumentQuery>) {
    setFilterDraft({ ...filterDraft, ...next });
  }

  function applyFilters() {
    if (!query || !onQueryChange) return;
    applyFiltersWithDraft(filterDraft);
  }

  function applyFiltersWithDraft(draft: StockDocumentQuery) {
    if (!query || !onQueryChange) return;
    onQueryChange({
      ...draft,
      documentType: query.documentType,
      outboundKind: isOutbound ? draft.outboundKind || null : null,
      month: draft.month || currentMonthString(),
      departmentId: isOutbound ? draft.departmentId || null : null,
      supplierId: !isOutbound && !isAdjustment ? draft.supplierId || null : null,
      itemId: draft.itemId || null,
      handler: draft.handler || null,
      search: draft.search?.trim() || null,
    });
  }

  function updateItemFilter(itemId: string) {
    const nextDraft = { ...filterDraft, itemId };
    setFilterDraft(nextDraft);
    applyFiltersWithDraft(nextDraft);
  }

  function resetFilters() {
    if (!query || !onQueryChange) return;
    const nextQuery: StockDocumentQuery = {
      documentType: query.documentType,
      month: currentMonthString(),
    };
    setFilterDraft(nextQuery);
    onQueryChange(nextQuery);
  }

  return (
    <div className="subtable">
      {title ? (
        <div className="subtable-heading">
          <h3>{title}</h3>
        </div>
      ) : null}
      {query && onQueryChange ? (
        <div
          className="document-filters"
          onKeyDown={(event) => submitOnEnter(event, applyFilters)}
        >
          <div className="filter-fields">
            <Field label="月份">
              <MonthSelect
                value={filterDraft.month ?? currentMonthString()}
                onChange={(month) => updateFilter({ month })}
              />
            </Field>
            {isAdjustment ? null : (
              <Field label={partyLabel}>
                <select
                  value={partyValue}
                  onChange={(e) =>
                    isOutbound
                      ? updateFilter({ departmentId: e.target.value })
                      : updateFilter({ supplierId: e.target.value })
                  }
                >
                  <option value="">全部</option>
                  {partyOptions.map((item) => (
                    <option key={item.id} value={item.id}>
                      {item.name}
                    </option>
                  ))}
                </select>
              </Field>
            )}
            {isAdjustment ? null : isOutbound ? (
              <Field label="出库类型">
                <select
                  value={filterDraft.outboundKind ?? ""}
                  onChange={(e) =>
                    updateFilter({
                      outboundKind:
                        e.target.value === ""
                          ? null
                          : (e.target.value as "internal" | "guest_sale"),
                    })
                  }
                >
                  <option value="">全部</option>
                  <option value="internal">内部员工领用</option>
                  <option value="guest_sale">酒店客人销售</option>
                </select>
              </Field>
            ) : null}
            <Field label="经办人">
              <select
                value={filterDraft.handler ?? ""}
                onChange={(e) => updateFilter({ handler: e.target.value })}
              >
                <option value="">全部</option>
                {effectiveHandlerOptions.map((handler) => (
                  <option key={handler} value={handler}>
                    {handler}
                  </option>
                ))}
              </select>
            </Field>
            <Field label="物品">
              <ItemSearchSelect
                allowEmpty
                disabled={false}
                emptyLabel="全部物品"
                items={items}
                value={filterDraft.itemId ?? ""}
                onChange={(itemId) => updateFilter({ itemId })}
                onCommit={updateItemFilter}
              />
            </Field>
            <Field label="关键字">
              <input
                placeholder="单号/物品/备注"
                value={filterDraft.search ?? ""}
                onChange={(e) => updateFilter({ search: e.target.value })}
              />
            </Field>
          </div>
          <div className="filter-actions document-filter-actions">
            <button className="ghost-button" onClick={resetFilters}>
              清空
            </button>
            <button className="primary-button" onClick={applyFilters}>
              筛选
            </button>
          </div>
        </div>
      ) : null}
      <table>
        <thead>
          <tr>
            <th>单号</th>
            <th>日期</th>
            {isOutbound ? <th>类型</th> : null}
            <th>对象</th>
            <th>物品</th>
            <th>审批单</th>
            <th>数量</th>
            <th>金额</th>
            <th>状态</th>
            <th>操作</th>
          </tr>
        </thead>
        <PaginatedTable
          colSpan={isOutbound ? 10 : 9}
          getRowKey={(doc) => doc.id}
          rows={documents}
        >
          {(doc) => (
            <>
              <td>{doc.documentNo}</td>
              <td>{formatDateTime(doc.businessDate)}</td>
              {isOutbound ? (
                <td>{outboundKindLabel(doc.outboundKind)}</td>
              ) : null}
              <td>
                {doc.documentType === "outbound"
                  ? doc.outboundKind === "guest_sale"
                    ? "酒店客人"
                    : (doc.departmentName ?? "-")
                  : (doc.supplierName ?? "-")}
              </td>
              <td className="item-summary-cell" title={doc.itemSummary ?? ""}>
                {doc.itemSummary ?? "-"}
              </td>
              <td>{doc.approvalRequestId ?? "-"}</td>
              <td>{doc.totalQuantity}</td>
              <td>{formatMoney(doc.totalAmount)}</td>
              <td>
                <span
                  className={
                    doc.status === "voided"
                      ? "status disabled"
                      : "status enabled"
                  }
                >
                  {doc.status === "confirmed"
                    ? "已确认"
                    : doc.status === "voided"
                      ? "已作废"
                      : doc.status === "draft"
                        ? "草稿"
                        : doc.status}
                </span>
              </td>
              <td className="row-actions">
                <button
                  onClick={() =>
                    openEditorWindow("stockDocumentDetail", {
                      documentType: doc.documentType as "inbound" | "outbound",
                      id: doc.id,
                      mode: "edit",
                      width: 980,
                      height: 680,
                    })
                  }
                >
                  详情
                </button>
                {doc.status === "draft" && onConfirmDraft ? (
                  <button
                    disabled={!canVoid}
                    onClick={() =>
                      onConfirmDraft(doc.id, approvalRequestId || null)
                    }
                  >
                    确认草稿
                  </button>
                ) : null}
                {onVoid && doc.status === "confirmed" ? (
                  <button
                    disabled={!canVoid}
                    onClick={() => onVoid(doc.id, voidReason, voidHandler)}
                  >
                    作废
                  </button>
                ) : null}
                {doc.status !== "draft" && doc.status !== "confirmed"
                  ? "-"
                  : null}
              </td>
            </>
          )}
        </PaginatedTable>
      </table>
    </div>
  );
}

function StockDocumentDetailViewer({
  detail,
  isLoading,
}: {
  detail: StockDocumentDetail | null;
  isLoading: boolean;
}) {
  if (isLoading) {
    return (
      <ReadOnlyEditorWindow>
        <div className="placeholder-panel">正在加载单据明细...</div>
      </ReadOnlyEditorWindow>
    );
  }
  if (!detail) {
    return (
      <ReadOnlyEditorWindow>
        <div className="placeholder-panel">未找到单据明细</div>
      </ReadOnlyEditorWindow>
    );
  }

  const { document, lines, batchLines } = detail;
  const isOutbound = document.documentType === "outbound";
  const partyLabel = isOutbound ? "对象" : "供应商";
  const partyValue = isOutbound
    ? document.outboundKind === "guest_sale"
      ? "酒店客人"
      : (document.departmentName ?? "-")
    : (document.supplierName ?? "-");

  return (
    <ReadOnlyEditorWindow>
      <div
        className={`document-detail-viewer ${
          batchLines.length > 0 ? "has-batches" : "single-lines"
        }`}
      >
        <div className="detail-summary-grid">
          <InfoTile label="单号" value={document.documentNo} />
          <InfoTile label="日期" value={formatDateTime(document.businessDate)} />
          <InfoTile
            label="类型"
            value={
              isOutbound
                ? outboundKindLabel(document.outboundKind)
                : document.documentType === "inbound"
                  ? "入库"
                  : document.documentType
            }
          />
          <InfoTile label={partyLabel} value={partyValue} />
          <InfoTile label="数量" value={String(document.totalQuantity)} />
          <InfoTile
            label={
              isOutbound
                ? document.outboundKind === "guest_sale"
                  ? "销售金额"
                  : "成本金额"
                : "采购金额"
            }
            value={formatMoney(document.totalAmount)}
          />
          {isOutbound && document.outboundKind === "guest_sale" ? (
            <>
              <InfoTile
                label="销售成本"
                value={formatMoney(document.totalCostAmount)}
              />
              <InfoTile
                label="毛利"
                value={formatMoney(document.totalGrossProfit)}
              />
            </>
          ) : null}
          <InfoTile label="经办人" value={document.handler ?? "-"} />
          <InfoTile label="用途" value={document.purpose ?? "-"} />
        </div>

        <div className="subtable document-detail-lines">
          <div className="subtable-heading">
            <h3>商品明细</h3>
            <span>{lines.length} 项</span>
          </div>
          <div className="document-detail-scroll">
            <table>
              <thead>
                <tr>
                  <th>商品</th>
                  <th>规格</th>
                  <th>单位</th>
                  <th>数量</th>
                  <th>{isOutbound ? "成本单价" : "采购单价"}</th>
                  <th>{isOutbound ? "成本金额" : "采购金额"}</th>
                  {isOutbound && document.outboundKind === "guest_sale" ? (
                    <>
                      <th>销售单价</th>
                      <th>销售金额</th>
                      <th>毛利</th>
                    </>
                  ) : null}
                  <th>备注</th>
                </tr>
              </thead>
              <tbody>
                {lines.map((line) => (
                  <tr key={line.id}>
                    <td>
                      <strong>{line.itemName}</strong>
                      <span className="muted-inline">{line.itemCode}</span>
                    </td>
                    <td>{line.spec ?? "-"}</td>
                    <td>{line.unitName ?? "-"}</td>
                    <td>{line.quantity}</td>
                    <td>
                      {formatMoney(
                        isOutbound
                          ? (line.costUnitPrice ?? line.unitPrice)
                          : (line.purchaseUnitPrice ?? line.unitPrice),
                      )}
                    </td>
                    <td>
                      {formatMoney(
                        isOutbound
                          ? (line.costAmount ?? line.amount)
                          : (line.purchaseAmount ?? line.amount),
                      )}
                    </td>
                    {isOutbound && document.outboundKind === "guest_sale" ? (
                      <>
                        <td>{formatMoney(line.saleUnitPrice ?? 0)}</td>
                        <td>{formatMoney(line.saleAmount ?? 0)}</td>
                        <td>{formatMoney(line.grossProfit ?? 0)}</td>
                      </>
                    ) : null}
                    <td>{line.remark ?? "-"}</td>
                  </tr>
                ))}
                {lines.length === 0 ? (
                  <tr>
                    <td
                      colSpan={
                        isOutbound && document.outboundKind === "guest_sale"
                          ? 10
                          : 7
                      }
                    >
                      暂无商品明细
                    </td>
                  </tr>
                ) : null}
              </tbody>
            </table>
          </div>
        </div>

        {batchLines.length > 0 ? (
          <div className="subtable document-detail-lines">
            <div className="subtable-heading">
              <h3>批次成本明细</h3>
              <span>{batchLines.length} 条</span>
            </div>
            <div className="document-detail-scroll">
              <table>
                <thead>
                  <tr>
                    <th>商品</th>
                    <th>批次号</th>
                    <th>入库日期</th>
                    <th>供应商</th>
                    <th>方向</th>
                    <th>数量</th>
                    <th>批次单价</th>
                    <th>批次金额</th>
                  </tr>
                </thead>
                <tbody>
                  {batchLines.map((line) => (
                    <tr key={line.id}>
                      <td>
                        <strong>{line.itemName}</strong>
                        <span className="muted-inline">{line.itemCode}</span>
                      </td>
                      <td>{line.batchNo}</td>
                      <td>{formatDateTime(line.inboundDate)}</td>
                      <td>{line.supplierName ?? "-"}</td>
                      <td>{line.direction === "in" ? "入库" : "出库"}</td>
                      <td>{line.quantity}</td>
                      <td>{formatMoney(line.unitPrice)}</td>
                      <td>{formatMoney(line.amount)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        ) : null}
      </div>
    </ReadOnlyEditorWindow>
  );
}

function StockBatchDetailViewer({
  batches,
  isLoading,
}: {
  batches: StockBatchRow[];
  isLoading: boolean;
}) {
  if (isLoading) {
    return (
      <ReadOnlyEditorWindow>
        <div className="placeholder-panel">正在加载批次库存...</div>
      </ReadOnlyEditorWindow>
    );
  }
  if (batches.length === 0) {
    return (
      <ReadOnlyEditorWindow>
        <div className="placeholder-panel">暂无批次库存</div>
      </ReadOnlyEditorWindow>
    );
  }

  const first = batches[0];
  const availableBatches = batches.filter(
    (batch) => batch.status !== "voided" && batch.remainingQuantity > 0,
  );
  const totalRemainingQuantity = availableBatches.reduce(
    (sum, batch) => sum + batch.remainingQuantity,
    0,
  );
  const totalRemainingAmount = availableBatches.reduce(
    (sum, batch) => sum + batch.remainingAmount,
    0,
  );

  return (
    <ReadOnlyEditorWindow>
      <div className="document-detail-viewer single-lines">
        <div className="detail-summary-grid">
          <InfoTile label="物品" value={`${first.itemCode} · ${first.itemName}`} />
          <InfoTile label="批次数" value={String(batches.length)} />
          <InfoTile label="可用批次" value={String(availableBatches.length)} />
          <InfoTile
            label="剩余数量"
            value={String(Number(totalRemainingQuantity.toFixed(6)))}
          />
          <InfoTile label="剩余金额" value={formatMoney(totalRemainingAmount)} />
        </div>

        <div className="subtable document-detail-lines">
          <div className="subtable-heading">
            <h3>批次余额</h3>
            <span>{batches.length} 条</span>
          </div>
          <div className="document-detail-scroll">
            <table>
              <thead>
                <tr>
                  <th>批次号</th>
                  <th>入库日期</th>
                  <th>来源单据</th>
                  <th>供应商</th>
                  <th>原始数量</th>
                  <th>剩余数量</th>
                  <th>批次单价</th>
                  <th>剩余金额</th>
                  <th>状态</th>
                </tr>
              </thead>
              <tbody>
                {batches.map((batch) => (
                  <tr key={batch.id}>
                    <td>{batch.batchNo}</td>
                    <td>{formatDateTime(batch.inboundDate)}</td>
                    <td>{batch.sourceDocumentNo ?? "-"}</td>
                    <td>{batch.supplierName ?? "-"}</td>
                    <td>{batch.originalQuantity}</td>
                    <td>{batch.remainingQuantity}</td>
                    <td>{formatMoney(batch.unitPrice)}</td>
                    <td>{formatMoney(batch.remainingAmount)}</td>
                    <td>{stockBatchStatusLabel(batch.status)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </ReadOnlyEditorWindow>
  );
}

function ReadOnlyEditorWindow({ children }: { children: React.ReactNode }) {
  return (
    <div className="editor-document readonly-editor-document">
      <div className="readonly-editor-scroll">{children}</div>
      <div className="editor-actions">
        <button
          className="primary-button"
          type="button"
          onClick={() => void closeCurrentEditorWindow()}
        >
          关闭
        </button>
      </div>
    </div>
  );
}

function stockBatchStatusLabel(status: string) {
  if (status === "available") return "可用";
  if (status === "depleted") return "已耗尽";
  if (status === "voided") return "已作废";
  if (status === "adjustment") return "调整批次";
  return status;
}

function InfoTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="info-tile">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function outboundKindLabel(kind?: "internal" | "guest_sale" | null) {
  return kind === "guest_sale" ? "酒店客人销售" : "内部员工领用";
}

function StocktakePage({
  canWrite,
  stocktakes,
}: {
  canWrite: boolean;
  stocktakes: StocktakeDocument[];
}) {
  return (
    <MasterTablePanel
      actions={
        <div className="supplier-toolbar">
          <button
            className="primary-button"
            disabled={!canWrite}
            onClick={() =>
              openEditorWindow("stocktakeCreate", { width: 780, height: 620 })
            }
          >
            创建盘点单
          </button>
        </div>
      }
      description="盘点记录创建与实盘录入在独立窗口中完成。"
      hideHeading
      title="库存盘点"
    >
      <table>
        <thead>
          <tr>
            <th>单号</th>
            <th>日期</th>
            <th>范围</th>
            <th>状态</th>
            <th>录入进度</th>
            <th>盘盈</th>
            <th>盘亏</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          {stocktakes.map((stocktake) => (
            <tr key={stocktake.id}>
              <td>{stocktake.documentNo}</td>
              <td>{formatDateTime(stocktake.businessDate)}</td>
              <td>{stocktakeScopeLabel(stocktake.scopeType)}</td>
              <td>{stocktakeStatusLabel(stocktake.status)}</td>
              <td>
                {stocktake.countedCount}/{stocktake.lineCount} 行
              </td>
              <td>{formatMoney(stocktake.gainAmount)}</td>
              <td>{formatMoney(stocktake.lossAmount)}</td>
              <td className="row-actions">
                <button
                  onClick={() =>
                    openEditorWindow("stocktakeDetail", {
                      mode: "edit",
                      id: stocktake.id,
                    })
                  }
                >
                  详情
                </button>
                <button
                  disabled={
                    !canWrite ||
                    stocktake.status === "confirmed" ||
                    stocktake.status === "voided"
                  }
                  onClick={() =>
                    openEditorWindow("stocktakeCounts", {
                      mode: "edit",
                      id: stocktake.id,
                      width: 1120,
                      height: 760,
                    })
                  }
                >
                  录入实盘
                </button>
              </td>
            </tr>
          ))}
          {stocktakes.length === 0 ? <EmptyRow colSpan={8} /> : null}
        </tbody>
      </table>
    </MasterTablePanel>
  );
}

function StocktakeDetailViewer({
  detail,
  disabled,
  isLoading,
  onConfirm,
  onExport,
  onVoid,
}: {
  detail: StocktakeDetail | null;
  disabled: boolean;
  isLoading: boolean;
  onConfirm: (
    stocktakeId: string,
    handler: string,
    remark: string,
  ) => Promise<void>;
  onExport: (stocktakeId: string) => Promise<void>;
  onVoid: (
    documentId: string,
    stocktakeId: string,
    reason: string,
    handler: string,
  ) => Promise<void>;
}) {
  const [handler, setHandler] = useState("");
  const [remark, setRemark] = useState("");
  const canEdit =
    detail &&
    detail.document.status !== "confirmed" &&
    detail.document.status !== "voided";
  const canVoid = detail?.document.status === "confirmed";
  const differenceLines =
    detail?.lines.filter(
      (line) => Math.abs(line.differenceQuantity) > 0.000001,
    ) ?? [];

  if (isLoading) {
    return (
      <ReadOnlyEditorWindow>
        <div className="placeholder-panel">正在加载盘点详情...</div>
      </ReadOnlyEditorWindow>
    );
  }
  if (!detail) {
    return (
      <ReadOnlyEditorWindow>
        <div className="placeholder-panel">盘点单不存在或已被删除。</div>
      </ReadOnlyEditorWindow>
    );
  }

  return (
    <div className="editor-document stocktake-detail-document">
      <div className="stocktake-detail-scroll">
        <div className="stocktake-detail-panel">
          <div className="document-detail-header">
            <div>
              <h2>{detail.document.documentNo}</h2>
              <span>
                {formatDateTime(detail.document.businessDate)} ·{" "}
                {stocktakeScopeLabel(detail.document.scopeType)} ·{" "}
                {stocktakeStatusLabel(detail.document.status)}
              </span>
            </div>
            <div className="report-actions">
              <input
                placeholder="经办人"
                value={handler}
                onChange={(e) => setHandler(e.target.value)}
              />
              <input
                placeholder="备注/作废原因"
                value={remark}
                onChange={(e) => setRemark(e.target.value)}
              />
            </div>
          </div>

          <section className="metrics-grid stocktake-metrics">
            <div className="metric-card">
              <span>盘点行数</span>
              <strong>{detail.document.lineCount}</strong>
              <em>行</em>
            </div>
            <div className="metric-card">
              <span>已录入</span>
              <strong>{detail.document.countedCount}</strong>
              <em>行</em>
            </div>
            <div className="metric-card">
              <span>差异项</span>
              <strong>{differenceLines.length}</strong>
              <em>项</em>
            </div>
            <div className="metric-card">
              <span>盘盈/盘亏</span>
              <strong>
                {formatMoney(detail.document.gainAmount)} /{" "}
                {formatMoney(detail.document.lossAmount)}
              </strong>
              <em>元</em>
            </div>
          </section>

          <div className="subtable document-detail-lines">
            <div className="subtable-heading">
              <h3>盘点商品</h3>
              <span>{detail.lines.length} 行</span>
            </div>
            <div className="document-detail-scroll">
              <table>
                <thead>
                  <tr>
                    <th>编码</th>
                    <th>物品</th>
                    <th>规格</th>
                    <th>单位</th>
                    <th>账面</th>
                    <th>实盘</th>
                    <th>差异</th>
                    <th>差异金额</th>
                    <th>备注</th>
                  </tr>
                </thead>
                <tbody>
                  {detail.lines.map((line) => (
                    <tr key={line.id}>
                      <td>{line.itemCode}</td>
                      <td>{line.itemName}</td>
                      <td>{line.spec ?? "-"}</td>
                      <td>{line.unitName ?? "-"}</td>
                      <td>{line.bookQuantity}</td>
                      <td>{line.countedQuantity ?? "-"}</td>
                      <td
                        className={
                          line.differenceQuantity === 0
                            ? ""
                            : line.differenceQuantity > 0
                              ? "gain-text"
                              : "loss-text"
                        }
                      >
                        {line.differenceQuantity}
                      </td>
                      <td>{formatMoney(line.differenceAmount)}</td>
                      <td>{line.remark ?? "-"}</td>
                    </tr>
                  ))}
                  {detail.lines.length === 0 ? <EmptyRow colSpan={9} /> : null}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </div>
      <div className="editor-actions">
        <button
          className="ghost-button"
          disabled={disabled}
          onClick={() => onExport(detail.document.id)}
        >
          导出盘点表
        </button>
        <button
          className="ghost-button"
          disabled={!canEdit || disabled}
          onClick={() =>
            openEditorWindow("stocktakeCounts", {
              mode: "edit",
              id: detail.document.id,
              width: 1120,
              height: 760,
            })
          }
        >
          录入实盘
        </button>
        <button
          className="ghost-button"
          disabled={!canVoid || disabled || !remark.trim()}
          onClick={() =>
            onVoid(detail.document.documentId, detail.document.id, remark, handler)
          }
        >
          作废盘点
        </button>
        <button
          className="primary-button"
          disabled={!canEdit || disabled}
          onClick={() => onConfirm(detail.document.id, handler, remark)}
        >
          确认盘点
        </button>
      </div>
    </div>
  );
}

function AdjustmentPage({
  canWrite,
  documents,
  handlerOptions,
  items,
  onQueryChange,
  onVoid,
  query,
}: {
  canWrite: boolean;
  documents: StockDocument[];
  handlerOptions: string[];
  items: Item[];
  onQueryChange: (query: StockDocumentQuery) => Promise<void>;
  onVoid: (
    documentId: string,
    reason: string,
    handler: string,
  ) => Promise<void>;
  query: StockDocumentQuery;
}) {
  const [voidReason, setVoidReason] = useState("");
  const [voidHandler, setVoidHandler] = useState("");

  return (
    <section className="table-panel">
      <div className="table-toolbar document-action-toolbar">
        <DocumentVoidControls
          approvalRequestId=""
          isOutbound={false}
          setApprovalRequestId={() => undefined}
          setVoidHandler={setVoidHandler}
          setVoidReason={setVoidReason}
          voidHandler={voidHandler}
          voidReason={voidReason}
        />
        <button
          className="primary-button"
          disabled={!canWrite}
          onClick={() =>
            openEditorWindow("adjustment", { width: 980, height: 760 })
          }
        >
          新建调整单
        </button>
      </div>
      <DocumentList
        canVoid={canWrite}
        documents={documents}
        handlerOptions={handlerOptions}
        items={items}
        isOutbound={false}
        onQueryChange={onQueryChange}
        onVoid={onVoid}
        query={query}
        voidHandler={voidHandler}
        voidReason={voidReason}
      />
    </section>
  );
}

export default App;
