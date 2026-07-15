import type { LocaleCode } from "../i18n";

export type RuntimeMode = "standalone" | "host" | "client";
export type ThemeMode = "auto" | "light" | "dark";
export type LiquidGlassStyle = "transparent" | "tinted";

export type AppearanceSettings = {
  themeMode: ThemeMode;
  liquidGlassStyle: LiquidGlassStyle;
  accentColor: string;
  locale: LocaleCode;
};

export type RuntimeConfig = {
  mode: RuntimeMode;
  hostAddress?: string | null;
  hostPort: number;
  clientPaired: boolean;
  clientDeviceId: string;
  dataDir: string;
  databasePath: string;
  backupDir: string;
  importReportDir: string;
};

export type HostServiceStatus = {
  running: boolean;
  bindAddress: string;
  port: number;
  pairCode?: string | null;
  clientCount: number;
  message: string;
};

export type ClientConnectionInfo = {
  id: string;
  clientName: string;
  clientDeviceId: string;
  clientIp: string;
  appVersion: string;
  status: string;
  lastSeenAt: string;
};

export type HostConnectionTestResult = {
  ok: boolean;
  message: string;
  appName?: string | null;
  appVersion?: string | null;
  schemaVersion?: number | null;
};

export type HostDiscoveryResult = {
  hostAddress: string;
  hostPort: number;
  appName: string;
  appVersion: string;
  schemaVersion: number;
  message: string;
};

export type AppUpdateState = {
  status: "idle" | "checking" | "available" | "notAvailable" | "downloading" | "installed" | "error";
  currentVersion?: string | null;
  latestVersion?: string | null;
  notes?: string | null;
  downloadedBytes: number;
  totalBytes?: number | null;
  error?: string | null;
  checkedAt?: string | null;
  sourceLabel?: string | null;
};

export type ProxyCandidate = { label: string; url: string };

export type DashboardMetrics = {
  itemCount: number;
  departmentCount: number;
  supplierCount: number;
  currentStockAmount: number;
  lowStockCount: number;
  negativeStockCount: number;
  thisMonthInboundAmount: number;
  thisMonthOutboundAmount: number;
};

export type RecentOperation = {
  id: string;
  occurredAt: string;
  businessType: string;
  itemName: string;
  quantity: number;
  departmentName?: string | null;
  supplierName?: string | null;
};

export type HealthStatus = {
  databaseOk: boolean;
  stockBalanceConsistencyOk: boolean;
  stockBalanceIssueCount: number;
  latestBackupAt?: string | null;
  latestIntervalBackupAt?: string | null;
  autoBackupEnabled: boolean;
  intervalBackupEnabled: boolean;
  intervalBackupHours: number;
  secondBackupOk: boolean;
  message: string;
};

export type AppStatus = {
  appName: string;
  appVersion: string;
  schemaVersion: number;
  runtime: RuntimeConfig;
  latestMovementMonth?: string | null;
  metrics: DashboardMetrics;
  recentOperations: RecentOperation[];
  health: HealthStatus;
};

export type SystemSettings = {
  hotelName: string;
  currentPeriod: string;
  defaultMonth: string;
  allowNegativeStock: boolean;
  quantityDecimals: number;
  amountDecimals: number;
  defaultExportDir: string;
  defaultBackupDir: string;
  autoBackupEnabled: boolean;
  intervalBackupEnabled: boolean;
  intervalBackupHours: number;
  smtpEnabled: boolean;
  smtpHost: string;
  smtpPort: number;
  smtpUsername: string;
  smtpPassword?: string | null;
  smtpFromEmail: string;
  smtpFromName: string;
  smtpPasswordConfigured: boolean;
};
