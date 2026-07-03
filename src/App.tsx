import { type CSSProperties, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit, emitTo, listen } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { open, type OpenDialogOptions } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ColorBendsBackground } from "./components/ColorBendsBackground";
import { createI18n, type I18n, type LocaleCode } from "./i18n";
import "./App.css";
import pantsFrame01 from "./assets/images/pants/pants_01.png";
import pantsFrame02 from "./assets/images/pants/pants_02.png";
import pantsFrame03 from "./assets/images/pants/pants_03.png";
import pantsFrame04 from "./assets/images/pants/pants_04.png";
import pantsFrame05 from "./assets/images/pants/pants_05.png";
import pantsFrame06 from "./assets/images/pants/pants_06.png";
import pantsFrame07 from "./assets/images/pants/pants_07.png";
import pantsFrame08 from "./assets/images/pants/pants_08.png";
import pantsFrame09 from "./assets/images/pants/pants_09.png";

const animatedExpressionModules = import.meta.glob("./assets/images/**/*.png", {
  eager: true,
  import: "default",
}) as Record<string, string>;

type RuntimeMode = "standalone" | "host" | "client";
type ThemeMode = "auto" | "light" | "dark";
type LiquidGlassStyle = "transparent" | "tinted";

type AppearanceSettings = {
  themeMode: ThemeMode;
  liquidGlassStyle: LiquidGlassStyle;
  accentColor: string;
  locale: LocaleCode;
};

type NavKey =
  | "dashboard"
  | "items"
  | "inbound"
  | "outbound"
  | "stock"
  | "movements"
  | "stocktake"
  | "adjustments"
  | "reports"
  | "import"
  | "departments"
  | "categories"
  | "units"
  | "suppliers"
  | "budgets"
  | "approvals"
  | "backups"
  | "logs"
  | "settings"
  | "users";

type RuntimeConfig = {
  mode: RuntimeMode;
  hostAddress?: string | null;
  hostPort: number;
  clientToken?: string | null;
  clientDeviceId: string;
  dataDir: string;
  databasePath: string;
  backupDir: string;
  importReportDir: string;
};

type HostServiceStatus = {
  running: boolean;
  bindAddress: string;
  port: number;
  pairCode?: string | null;
  clientCount: number;
  message: string;
};

type ClientConnectionInfo = {
  id: string;
  clientName: string;
  clientDeviceId: string;
  clientIp: string;
  appVersion: string;
  status: string;
  lastSeenAt: string;
};

type HostConnectionTestResult = {
  ok: boolean;
  message: string;
  appName?: string | null;
  appVersion?: string | null;
  schemaVersion?: number | null;
};

type HostDiscoveryResult = {
  hostAddress: string;
  hostPort: number;
  appName: string;
  appVersion: string;
  schemaVersion: number;
  message: string;
};

type DashboardMetrics = {
  itemCount: number;
  departmentCount: number;
  supplierCount: number;
  currentStockAmount: number;
  lowStockCount: number;
  negativeStockCount: number;
  thisMonthInboundAmount: number;
  thisMonthOutboundAmount: number;
};

type RecentOperation = {
  id: string;
  occurredAt: string;
  businessType: string;
  itemName: string;
  quantity: number;
  departmentName?: string | null;
  supplierName?: string | null;
};

type HealthStatus = {
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

type Role = {
  id: string;
  code: string;
  name: string;
};

type UserAccount = {
  id: string;
  username: string;
  displayName: string;
  email?: string | null;
  departmentId?: string | null;
  departmentName?: string | null;
  enabled: boolean;
  roles: Role[];
  createdAt: string;
  updatedAt: string;
};

type CurrentUser = {
  id: string;
  username: string;
  displayName: string;
  departmentId?: string | null;
  departmentName?: string | null;
  roles: Role[];
  permissions: string[];
};

type AppStatus = {
  appName: string;
  appVersion: string;
  schemaVersion: number;
  runtime: RuntimeConfig;
  latestMovementMonth?: string | null;
  metrics: DashboardMetrics;
  recentOperations: RecentOperation[];
  health: HealthStatus;
};

type SystemSettings = {
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

type Category = {
  id: string;
  parentId?: string | null;
  name: string;
  enabled: boolean;
  sortOrder: number;
  updatedAt: string;
};

type Unit = {
  id: string;
  name: string;
  enabled: boolean;
  sortOrder: number;
  updatedAt: string;
};

type Department = {
  id: string;
  code: string;
  name: string;
  manager?: string | null;
  enabled: boolean;
  sortOrder: number;
  remark?: string | null;
  updatedAt: string;
};

type Supplier = {
  id: string;
  name: string;
  contact?: string | null;
  phone?: string | null;
  address?: string | null;
  enabled: boolean;
  remark?: string | null;
  updatedAt: string;
};

type SupplierPurchaseRecord = {
  movementDate: string;
  documentNo?: string | null;
  itemCode: string;
  itemName: string;
  spec?: string | null;
  unitName?: string | null;
  quantity: number;
  unitPrice: number;
  amount: number;
  remark?: string | null;
};

type Item = {
  id: string;
  code: string;
  barcode?: string | null;
  name: string;
  categoryId?: string | null;
  categoryName?: string | null;
  spec?: string | null;
  unitId?: string | null;
  unitName?: string | null;
  defaultPrice: number;
  supplierId?: string | null;
  supplierName?: string | null;
  warningQuantity: number;
  enabled: boolean;
  remark?: string | null;
  updatedAt: string;
};

type StockDocument = {
  id: string;
  documentNo: string;
  documentType: "inbound" | "outbound" | "adjustment" | "stocktake";
  businessDate: string;
  departmentName?: string | null;
  supplierName?: string | null;
  handler?: string | null;
  purpose?: string | null;
  approvalRequestId?: string | null;
  status: string;
  totalQuantity: number;
  totalAmount: number;
  createdAt: string;
};

type StockDocumentQuery = {
  documentType: "inbound" | "outbound" | "adjustment" | "stocktake";
  month?: string | null;
  departmentId?: string | null;
  supplierId?: string | null;
  itemId?: string | null;
  search?: string | null;
};

type StockBalanceQuery = {
  search?: string | null;
  categoryId?: string | null;
  itemId?: string | null;
  stockStatus?: "normal" | "low" | "negative" | null;
};

type StockMovementQuery = {
  search?: string | null;
  itemId?: string | null;
  departmentId?: string | null;
  direction?: "in" | "out" | null;
  movementType?: string | null;
};

type StockBalanceRow = {
  itemId: string;
  itemCode: string;
  itemName: string;
  spec?: string | null;
  unitName?: string | null;
  quantity: number;
  amount: number;
  averagePrice: number;
  lastInboundPrice: number;
  warningQuantity: number;
  stockStatus: "normal" | "low" | "negative";
};

type StockMovementRow = {
  id: string;
  movementDate: string;
  itemCode: string;
  itemName: string;
  direction: "in" | "out";
  quantity: number;
  unitPrice: number;
  amount: number;
  documentNo?: string | null;
  departmentName?: string | null;
  supplierName?: string | null;
  movementType: string;
  operator?: string | null;
  remark?: string | null;
  createdAt: string;
};

type StocktakeDocument = {
  id: string;
  documentId: string;
  documentNo: string;
  businessDate: string;
  scopeType: "all" | "category" | "custom";
  status: string;
  handler?: string | null;
  remark?: string | null;
  lineCount: number;
  countedCount: number;
  differenceCount: number;
  gainAmount: number;
  lossAmount: number;
  createdAt: string;
  confirmedAt?: string | null;
};

type StocktakeLine = {
  id: string;
  stocktakeId: string;
  itemId: string;
  itemCode: string;
  itemName: string;
  spec?: string | null;
  unitName?: string | null;
  bookQuantity: number;
  countedQuantity?: number | null;
  differenceQuantity: number;
  averagePrice: number;
  differenceAmount: number;
  remark?: string | null;
};

type StocktakeDetail = {
  document: StocktakeDocument;
  lines: StocktakeLine[];
};

type MonthlyInventoryRow = {
  itemId: string;
  itemCode: string;
  itemName: string;
  spec?: string | null;
  unitName?: string | null;
  inboundQuantity: number;
  inboundAmount: number;
  outboundQuantity: number;
  outboundAmount: number;
  endingQuantity: number;
  endingAmount: number;
};

type DepartmentIssueSummaryRow = {
  departmentId: string;
  departmentName: string;
  quantity: number;
  amount: number;
};

type DepartmentIssueDetailRow = {
  movementDate: string;
  departmentName: string;
  itemCode: string;
  itemName: string;
  spec?: string | null;
  unitName?: string | null;
  quantity: number;
  unitPrice: number;
  amount: number;
  documentNo?: string | null;
  purpose?: string | null;
  remark?: string | null;
};

type CategoryConsumptionRow = {
  categoryId?: string | null;
  categoryName: string;
  quantity: number;
  amount: number;
};

type ItemConsumptionRow = {
  itemId: string;
  itemCode: string;
  itemName: string;
  spec?: string | null;
  unitName?: string | null;
  quantity: number;
  amount: number;
};

type InboundDetailRow = {
  movementDate: string;
  supplierName: string;
  itemCode: string;
  itemName: string;
  spec?: string | null;
  unitName?: string | null;
  quantity: number;
  unitPrice: number;
  amount: number;
  documentNo?: string | null;
  remark?: string | null;
};

type StockWarningRow = {
  itemId: string;
  itemCode: string;
  itemName: string;
  spec?: string | null;
  unitName?: string | null;
  quantity: number;
  warningQuantity: number;
  shortageQuantity: number;
  amount: number;
};

type StockBalanceReportRow = {
  itemId: string;
  itemCode: string;
  itemName: string;
  spec?: string | null;
  unitName?: string | null;
  quantity: number;
  amount: number;
  averagePrice: number;
  lastInboundPrice: number;
  warningQuantity: number;
  stockStatus: string;
};

type StocktakeDifferenceReportRow = {
  businessDate: string;
  documentNo: string;
  scopeType: string;
  status: string;
  itemCode: string;
  itemName: string;
  spec?: string | null;
  unitName?: string | null;
  bookQuantity: number;
  countedQuantity: number;
  differenceQuantity: number;
  averagePrice: number;
  differenceAmount: number;
  remark?: string | null;
};

type ReportBundle = {
  month: string;
  monthlyInventory: MonthlyInventoryRow[];
  departmentSummary: DepartmentIssueSummaryRow[];
  departmentDetails: DepartmentIssueDetailRow[];
  categoryConsumption: CategoryConsumptionRow[];
  itemConsumptionRanking: ItemConsumptionRow[];
  inboundDetails: InboundDetailRow[];
  outboundDetails: DepartmentIssueDetailRow[];
  stockBalances: StockBalanceReportRow[];
  stockWarnings: StockWarningRow[];
  stocktakeDifferences: StocktakeDifferenceReportRow[];
};

type ReportQuery = {
  month: string;
  startDate?: string | null;
  endDate?: string | null;
  departmentId?: string | null;
  categoryId?: string | null;
  itemId?: string | null;
  supplierId?: string | null;
};

type ImportMessage = {
  level: string;
  sheet: string;
  row: number;
  column?: string | null;
  message: string;
};

type ImportItemPreview = {
  name: string;
  categoryName?: string | null;
  spec?: string | null;
  unitName?: string | null;
  defaultPrice: number;
  openingQuantity: number;
  inboundQuantity: number;
  outboundQuantity: number;
  existing: boolean;
};

type ImportMonthPreview = {
  month: string;
  rowCount: number;
  openingQuantity: number;
  inboundQuantity: number;
  outboundQuantity: number;
  outboundAmount: number;
};

type ImportPreview = {
  sourceFile: string;
  sheetCount: number;
  rowCount: number;
  itemCount: number;
  newItemCount: number;
  existingItemCount: number;
  openingQuantity: number;
  openingAmount: number;
  inboundQuantity: number;
  inboundAmount: number;
  outboundQuantity: number;
  outboundAmount: number;
  documentCount: number;
  warnings: ImportMessage[];
  errors: ImportMessage[];
  items: ImportItemPreview[];
  months: ImportMonthPreview[];
};

type ImportResult = {
  jobId: string;
  sourceFile: string;
  importedItems: number;
  matchedItems: number;
  documentCount: number;
  movementCount: number;
  warningCount: number;
  errorCount: number;
  reportPath?: string | null;
  sourceCopyPath?: string | null;
};

type BackupRecord = {
  id: string;
  backupFile: string;
  backupType: string;
  appVersion: string;
  schemaVersion: number;
  hostName?: string | null;
  os?: string | null;
  databaseSize: number;
  sha256?: string | null;
  status: string;
  errorMessage?: string | null;
  createdAt: string;
};

type AuditLogRow = {
  id: string;
  action: string;
  entityType: string;
  entityId: string;
  summary: string;
  operator: string;
  createdAt: string;
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

type BudgetRule = {
  id: string;
  departmentId: string;
  departmentName: string;
  categoryId: string;
  categoryName: string;
  periodMonth: string;
  amountLimit: number;
  usedAmount: number;
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
};

type BudgetRuleDraft = {
  id?: string;
  expectedUpdatedAt?: string;
  departmentId: string;
  categoryId: string;
  periodMonth: string;
  amountLimit: number;
  enabled: boolean;
};

type ApprovalRequest = {
  id: string;
  entityType: string;
  entityId: string;
  status: string;
  requestedBy?: string | null;
  decidedBy?: string | null;
  reason?: string | null;
  decisionNote?: string | null;
  createdAt: string;
  decidedAt?: string | null;
};

type CreateApprovalRequestDraft = {
  entityType: string;
  entityId: string;
  reason: string;
};

type OptionRecord = {
  id: string;
  name: string;
  enabled: boolean;
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
  | "businessSettings"
  | "clientConnection"
  | "clientPairing"
  | "connectionWizard"
  | "secondBackupDir"
  | "restoreBackup"
  | "stockDocument"
  | "adjustment"
  | "stocktakeCreate"
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

const pantsFrames = [
  pantsFrame01,
  pantsFrame02,
  pantsFrame03,
  pantsFrame04,
  pantsFrame05,
  pantsFrame06,
  pantsFrame07,
  pantsFrame08,
  pantsFrame09,
];

const animatedExpressionGroups = Object.entries(animatedExpressionModules)
  .filter(([path]) => !path.includes("/._"))
  .reduce(
    (groups, [path, src]) => {
      const parts = path.split("/");
      const groupName = parts[parts.length - 2];
      const fileName = parts[parts.length - 1] ?? "";
      if (!groupName || !/_[0-9]+\.png$/i.test(fileName)) return groups;

      const existing = groups.get(groupName) ?? [];
      existing.push({ path, src });
      groups.set(groupName, existing);
      return groups;
    },
    new Map<string, Array<{ path: string; src: string }>>(),
  );

const animatedExpressions = Array.from(animatedExpressionGroups.entries())
  .map(([name, frames]) => ({
    frames: frames
      .sort((left, right) => left.path.localeCompare(right.path))
      .map((frame) => frame.src),
    name,
  }))
  .filter((group) => group.frames.length > 0)
  .sort((left, right) => left.name.localeCompare(right.name));

const loginExpressionItems = [
  ...animatedExpressions,
  {
    frames: pantsFrames,
    name: "pants-logo",
  },
];

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
const accentColors = [
  "#8f96a3",
  "#2f6dff",
  "#a65dd9",
  "#f062a8",
  "#ff6a57",
  "#ffb020",
  "#f5dd00",
  "#33c96f",
] as const;

const accentColorLabelKeys: Record<string, string> = {
  "#8f96a3": "color.graphite",
  "#2f6dff": "color.blue",
  "#a65dd9": "color.purple",
  "#f062a8": "color.pink",
  "#ff6a57": "color.coral",
  "#ffb020": "color.amber",
  "#f5dd00": "color.lemon",
  "#33c96f": "color.green",
};

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
  supplierId: "",
  warningQuantity: 0,
  enabled: true,
  remark: "",
};

function todayString() {
  return new Date().toISOString().slice(0, 10);
}

function currentMonthString() {
  return new Date().toISOString().slice(0, 7);
}

function formatMoney(value: number) {
  return defaultI18n.formatMoney(value);
}

function accentColorLabel(color: string, i18n = defaultI18n) {
  return i18n.t(accentColorLabelKeys[color] ?? color);
}

const DEFAULT_TABLE_PAGE_SIZE = 50;

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

function normalizeSearchText(value?: string | number | null) {
  return String(value ?? "")
    .trim()
    .toLowerCase();
}

function itemSearchText(item: Item) {
  return normalizeSearchText(
    [
      item.code,
      item.barcode,
      item.name,
      item.spec,
      item.unitName,
      item.categoryName,
      item.supplierName,
    ]
      .filter(Boolean)
      .join(" "),
  );
}

function itemDisplayName(item: Item) {
  return [item.code, item.name].filter(Boolean).join(" · ");
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

function formatFileSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

function backupTypeLabel(type: string, i18n = defaultI18n) {
  return i18n.backupTypeLabel(type);
}

function auditActionLabel(action: string, i18n = defaultI18n) {
  return i18n.auditActionLabel(action);
}

function auditEntityLabel(type: string, i18n = defaultI18n) {
  return i18n.auditEntityLabel(type);
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

function editorTitle(
  editor: EditorKind,
  mode: EditorMode,
  documentType?: "inbound" | "outbound",
) {
  if (editor === "stockDocument") {
    return documentType === "outbound" ? "新建出库/领用单" : "新建入库单";
  }
  const labels: Record<Exclude<EditorKind, "stockDocument">, string> = {
    adjustment: "库存调整",
    budget: "预算规则",
    businessSettings: "业务与目录设置",
    category: "分类",
    changePassword: "修改密码",
    clientConnection: "客户端连接",
    clientPairing: "客户端配对",
    connectionWizard: "多电脑连接",
    department: "部门",
    item: "物品",
    restoreBackup: "恢复备份",
    secondBackupDir: "第二备份目录",
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
    editor === "businessSettings" ||
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
  if (editor === "item" || editor === "user" || editor === "businessSettings") {
    return { width: 760, height: 560, minWidth: 640, minHeight: 420 };
  }
  if (editor === "clientConnection" || editor === "restoreBackup") {
    return { width: 720, height: 560, minWidth: 620, minHeight: 420 };
  }
  if (editor === "clientPairing") {
    return { width: 620, height: 420, minWidth: 520, minHeight: 340 };
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

function PantsLogo() {
  const [frameIndex, setFrameIndex] = useState(0);

  useEffect(() => {
    const timer = window.setInterval(() => {
      setFrameIndex((current) => (current + 1) % pantsFrames.length);
    }, 130);

    return () => window.clearInterval(timer);
  }, []);

  return (
    <img
      className="brand-mark"
      src={pantsFrames[frameIndex]}
      alt="Aster"
      draggable={false}
    />
  );
}

function AnimatedExpressionWall() {
  const [frameIndex, setFrameIndex] = useState(0);

  useEffect(() => {
    const timer = window.setInterval(() => {
      setFrameIndex((current) => current + 1);
    }, 130);

    return () => window.clearInterval(timer);
  }, []);

  return (
    <div className="login-expression-wall" aria-hidden="true">
      {loginExpressionItems.map((expression, index) => (
        <div className="login-expression-item" key={expression.name}>
          <img
            alt=""
            draggable={false}
            src={expression.frames[(frameIndex + index * 2) % expression.frames.length]}
          />
        </div>
      ))}
    </div>
  );
}

function App() {
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
    });
  const [outboundDocumentQuery, setOutboundDocumentQuery] =
    useState<StockDocumentQuery>({
      documentType: "outbound",
    });
  const [stockBalances, setStockBalances] = useState<StockBalanceRow[]>([]);
  const [stockMovements, setStockMovements] = useState<StockMovementRow[]>([]);
  const [stockBalanceQuery, setStockBalanceQuery] = useState<StockBalanceQuery>(
    {},
  );
  const [stockMovementQuery, setStockMovementQuery] =
    useState<StockMovementQuery>({});
  const [stocktakes, setStocktakes] = useState<StocktakeDocument[]>([]);
  const [activeStocktake, setActiveStocktake] =
    useState<StocktakeDetail | null>(null);
  const [lastStocktakeExportPath, setLastStocktakeExportPath] = useState<
    string | null
  >(null);
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
    setInboundDocumentQuery({ documentType: "inbound" });
    setOutboundDocumentQuery({ documentType: "outbound" });
    setStockBalances([]);
    setStockMovements([]);
    setStockBalanceQuery({});
    setStockMovementQuery({});
    setStocktakes([]);
    setActiveStocktake(null);
    setLastStocktakeExportPath(null);
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
        query: { documentType: "adjustment" },
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
    }
    await loadStockDocuments(query);
  }

  async function applyStockBalanceQuery(query: StockBalanceQuery) {
    const normalizedQuery: StockBalanceQuery = {
      search: query.search?.trim() || null,
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

  async function loadStocktakeDetail(stocktakeId: string) {
    try {
      setError(null);
      const detail = await invoke<StocktakeDetail>("get_stocktake_detail", {
        stocktakeId,
      });
      setActiveStocktake(detail);
    } catch (err) {
      setError(formatError(err));
    }
  }

  async function confirmActiveStocktake(handler: string, remark: string) {
    if (!activeStocktake) return;
    await runAction("盘点单已确认，差异流水已生成", async () => {
      const detail = await invoke<StocktakeDetail>("confirm_stocktake", {
        request: {
          stocktakeId: activeStocktake.document.id,
          handler,
          remark,
        },
      });
      setActiveStocktake(detail);
    });
  }

  async function voidActiveStocktake(reason: string, handler: string) {
    if (!activeStocktake) return;
    const stocktakeId = activeStocktake.document.id;
    await runAction("盘点单已作废，冲正流水已生成", async () => {
      await invoke("void_stock_document", {
        request: {
          documentId: activeStocktake.document.documentId,
          reason,
          handler,
        },
      });
      const [nextStocktakes, detail] = await Promise.all([
        invoke<StocktakeDocument[]>("list_stocktakes"),
        invoke<StocktakeDetail>("get_stocktake_detail", { stocktakeId }),
      ]);
      setStocktakes(nextStocktakes);
      setActiveStocktake(detail);
    });
  }

  async function exportActiveStocktake() {
    if (!activeStocktake) return;
    await runAction("盘点表已导出", async () => {
      const result = await invoke<{ path: string }>("export_stocktake_sheet", {
        request: { stocktakeId: activeStocktake.document.id },
      });
      setLastStocktakeExportPath(result.path);
    });
  }

  async function loginUser(username: string, password: string) {
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
      setCurrentUser(user);
      setNotice(`已登录：${user.displayName}`);
      scheduleRefreshAll();
    } catch (err) {
      setError(formatError(err));
    } finally {
      setIsLoginPending(false);
    }
  }

  async function requestPasswordResetCode(username: string) {
    try {
      setIsLoginPending(true);
      setError(null);
      setNotice(null);
      const result = await invoke<{ maskedEmail: string; expiresMinutes: number }>(
        "request_password_reset_code",
        { request: { username } },
      );
      setNotice(
        `验证码已发送至 ${result.maskedEmail}，${result.expiresMinutes} 分钟内有效。`,
      );
    } catch (err) {
      setError(formatError(err));
      throw err;
    } finally {
      setIsLoginPending(false);
    }
  }

  async function resetPasswordWithCode(
    username: string,
    code: string,
    newPassword: string,
  ) {
    try {
      setIsLoginPending(true);
      setError(null);
      setNotice(null);
      await invoke("reset_password_with_code", {
        request: { username, code, newPassword },
      });
      setNotice("密码已重置，请使用新密码登录。");
    } catch (err) {
      setError(formatError(err));
      throw err;
    } finally {
      setIsLoginPending(false);
    }
  }

  async function logoutUser() {
    try {
      await invoke("logout");
      setCurrentUser(null);
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
    let unlisten: (() => void) | undefined;
    void listen<EditorSavedPayload>("editor:saved", (event) => {
      if (event.payload.stocktakeId) {
        void loadStocktakeDetail(event.payload.stocktakeId);
      }
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
        onLogin={loginUser}
        onRequestPasswordResetCode={requestPasswordResetCode}
        onResetPasswordWithCode={resetPasswordWithCode}
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
              onNavigate={setActiveNav}
              status={status}
            />
          ) : null}

          {activeNav === "items" ? (
            <ItemsPage
              canWrite={canWriteStock}
              categories={enabledCategories}
              itemSearch={itemSearch}
              items={items}
              onSearch={async (search) => {
                setItemSearch(search);
                await refreshAll(search);
              }}
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
          <SimpleNamePage
            canWrite={canWriteStock}
            description="单位用于物品档案和单据展示。"
            fields={["sortOrder"]}
            items={units}
            onToggle={(id, enabled, expectedUpdatedAt) =>
              runAction("单位状态已更新", () =>
                invoke("set_unit_enabled", { id, enabled, expectedUpdatedAt }),
              )
            }
            title="单位管理"
          />
        ) : null}

        {activeNav === "suppliers" ? (
          <SuppliersPage
            canWrite={canWriteStock}
            activeSupplier={activeSupplier}
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
            month={reportMonth}
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
            onDecide={decideApprovalRequest}
          />
        ) : null}

        {activeNav === "inbound" ? (
          <StockDocumentPage
            canWrite={canWriteStock}
            departments={departments.filter((item) => item.enabled)}
            documentType="inbound"
            documents={inboundDocuments}
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
            canViewReports={canViewReports}
            canWrite={canWriteStock}
            detail={activeStocktake}
            exportPath={lastStocktakeExportPath}
            onConfirm={confirmActiveStocktake}
            onExport={exportActiveStocktake}
            onSelect={loadStocktakeDetail}
            onVoid={voidActiveStocktake}
            stocktakes={stocktakes}
          />
        ) : null}

        {activeNav === "adjustments" ? (
          <AdjustmentPage
            canWrite={canWriteStock}
            documents={adjustmentDocuments}
            onVoid={voidDocument}
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
            onPreview={previewImport}
            onRun={runImport}
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
            onLogout={logoutUser}
            onRemoveClientConnection={removeClientConnection}
            onStartHostService={startHostRuntime}
            clientConnections={clientConnections}
            hostStatus={hostStatus}
            hostTestResult={hostTestResult}
            i18n={i18n}
            status={status}
            systemSettings={systemSettings}
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

function Dashboard({
  changeMode,
  i18n,
  isSavingMode,
  metricCards,
  onNavigate,
  status,
}: {
  changeMode: (mode: RuntimeMode) => void;
  i18n: I18n;
  isSavingMode: boolean;
  metricCards: { label: string; value: string | number; suffix: string }[];
  onNavigate: (key: NavKey) => void;
  status: AppStatus | null;
}) {
  const quickActions: { label: string; nav: NavKey }[] = [
    { label: i18n.t("dashboard.quick.inbound"), nav: "inbound" },
    { label: i18n.t("dashboard.quick.outbound"), nav: "outbound" },
    { label: i18n.t("dashboard.quick.import"), nav: "import" },
    { label: i18n.t("dashboard.quick.reports"), nav: "reports" },
    { label: i18n.t("dashboard.quick.stocktake"), nav: "stocktake" },
  ];

  return (
    <>
      <section className="status-grid">
        <div className="status-panel">
          <span className="panel-label">{i18n.t("dashboard.runtimeMode")}</span>
          <strong>
            {status
              ? modeLabel(status.runtime.mode, i18n)
              : i18n.t("dashboard.loading")}
          </strong>
          <div className="segmented">
            {(["standalone", "host", "client"] as RuntimeMode[]).map((mode) => (
              <button
                className={status?.runtime.mode === mode ? "selected" : ""}
                disabled={isSavingMode}
                key={mode}
                onClick={() => changeMode(mode)}
              >
                {modeLabel(mode, i18n)}
              </button>
            ))}
          </div>
        </div>

        <div className="status-panel">
          <span className="panel-label">{i18n.t("dashboard.database")}</span>
          <strong>
            {status?.health.databaseOk
              ? i18n.t("dashboard.databaseHealthy")
              : i18n.t("dashboard.databasePending")}
          </strong>
          <p>
            {status?.health.message ?? i18n.t("dashboard.databaseInitializing")}
          </p>
          {status && !status.health.stockBalanceConsistencyOk ? (
            <p className="warning-text">
              {i18n.t("dashboard.stockBalanceMismatch", {
                count: status.health.stockBalanceIssueCount,
              })}
            </p>
          ) : null}
          {!status?.health.secondBackupOk ? (
            <p className="warning-text">
              {i18n.t("dashboard.secondBackupNotReady")}
            </p>
          ) : null}
        </div>

        <div className="status-panel">
          <span className="panel-label">{i18n.t("dashboard.appVersion")}</span>
          <strong>{status?.appVersion ?? "0.1.0"}</strong>
          <p>Schema v{status?.schemaVersion ?? 0}</p>
        </div>
      </section>

      <section className="metrics-grid">
        {metricCards.map((card) => (
          <div className="metric-card" key={card.label}>
            <span>{card.label}</span>
            <strong>{card.value}</strong>
            <em>{card.suffix}</em>
          </div>
        ))}
      </section>

      <section className="workspace-grid">
        <div className="module-panel">
          <div className="section-heading">
            <h2>{i18n.t("dashboard.quickActions")}</h2>
            <span>{i18n.t("dashboard.quickActionsHint")}</span>
          </div>
          <div className="quick-action-grid">
            {quickActions.map((item) => (
              <button key={item.nav} onClick={() => onNavigate(item.nav)}>
                {item.label}
              </button>
            ))}
          </div>
        </div>

        <div className="module-panel recent-panel">
          <div className="section-heading">
            <h2>{i18n.t("dashboard.recentOperations")}</h2>
            <span>{i18n.t("dashboard.recentOperationsHint")}</span>
          </div>
          <table className="compact-table">
            <thead>
              <tr>
                <th>{i18n.t("dashboard.table.time")}</th>
                <th>{i18n.t("dashboard.table.type")}</th>
                <th>{i18n.t("dashboard.table.item")}</th>
                <th>{i18n.t("dashboard.table.quantity")}</th>
                <th>{i18n.t("dashboard.table.departmentSupplier")}</th>
              </tr>
            </thead>
            <tbody>
              {(status?.recentOperations ?? []).map((row) => (
                <tr key={row.id}>
                  <td>{row.occurredAt}</td>
                  <td>{movementTypeLabel(row.businessType, i18n)}</td>
                  <td>{row.itemName}</td>
                  <td>{row.quantity}</td>
                  <td>{row.departmentName ?? row.supplierName ?? "-"}</td>
                </tr>
              ))}
              {!status || status.recentOperations.length === 0 ? (
                <EmptyRow colSpan={5} />
              ) : null}
            </tbody>
          </table>
        </div>

        <div className="module-panel">
          <div className="section-heading">
            <h2>{i18n.t("dashboard.mainline")}</h2>
            <span>{i18n.t("dashboard.mainlineHint")}</span>
          </div>
          <div className="workstream-list">
            {workstreams.map((item) => (
              <div className="workstream" key={item.titleKey}>
                <strong>{i18n.t(item.titleKey)}</strong>
                <p>{i18n.t(item.bodyKey)}</p>
              </div>
            ))}
          </div>
        </div>

        <div className="module-panel">
          <div className="section-heading">
            <h2>{i18n.t("dashboard.localData")}</h2>
            <span>{i18n.t("dashboard.localDataHint")}</span>
          </div>
          <dl className="path-list">
            <dt>{i18n.t("dashboard.dataDir")}</dt>
            <dd>{status?.runtime.dataDir ?? "-"}</dd>
            <dt>SQLite</dt>
            <dd>{status?.runtime.databasePath ?? "-"}</dd>
            <dt>{i18n.t("dashboard.backupDir")}</dt>
            <dd>{status?.runtime.backupDir ?? "-"}</dd>
            <dt>{i18n.t("dashboard.importReportDir")}</dt>
            <dd>{status?.runtime.importReportDir ?? "-"}</dd>
          </dl>
        </div>
      </section>
    </>
  );
}

function LoginScreen({
  error,
  i18n = defaultI18n,
  isLoginPending,
  notice,
  onLogin,
  onRequestPasswordResetCode,
  onResetPasswordWithCode,
}: {
  error: string | null;
  i18n?: I18n;
  isLoginPending: boolean;
  notice: string | null;
  onLogin: (username: string, password: string) => Promise<void>;
  onRequestPasswordResetCode: (username: string) => Promise<void>;
  onResetPasswordWithCode: (
    username: string,
    code: string,
    newPassword: string,
  ) => Promise<void>;
}) {
  const [username, setUsername] = useState("admin");
  const [password, setPassword] = useState("");
  const [isResetOpen, setIsResetOpen] = useState(false);
  const [resetCode, setResetCode] = useState("");
  const [resetPassword, setResetPassword] = useState("");
  const canResetPassword =
    username.trim() && resetCode.trim().length === 6 && resetPassword.length >= 6;

  function submitLogin(event: React.FormEvent) {
    event.preventDefault();
    void onLogin(username, password);
  }

  async function submitResetCode() {
    await onRequestPasswordResetCode(username);
    setResetCode("");
    setResetPassword("");
  }

  async function submitResetPassword(event: React.FormEvent) {
    event.preventDefault();
    await onResetPasswordWithCode(username, resetCode, resetPassword);
    setIsResetOpen(false);
    setPassword("");
    setResetCode("");
    setResetPassword("");
  }

  return (
    <main className="login-shell">
      <section className="login-brand-panel">
        <ColorBendsBackground
          bandWidth={6}
          className="login-color-bends-bg"
          colors={["#ff5c7a", "#8a5cff", "#00ffd1"]}
          frequency={1}
          intensity={1.5}
          iterations={1}
          mouseInfluence={1}
          noise={0.15}
          parallax={0.5}
          rotation={90}
          scale={1}
          speed={0.2}
          transparent
          warpStrength={1}
        />
        <div className="login-brand-content">
          <AnimatedExpressionWall />
          <div className="login-copy">
            <h1>{i18n.t("login.title")}</h1>
            <p>{i18n.t("login.description")}</p>
          </div>
        </div>
      </section>

      <section className="login-card">
        <div className="login-card-header">
          <h2>{i18n.t("login.accountLogin")}</h2>
        </div>

        {error ? (
          <div className="error-banner login-message">{error}</div>
        ) : null}
        {notice ? (
          <div className="notice-banner login-message">{notice}</div>
        ) : null}

        <form className="login-form" onSubmit={submitLogin}>
          <Field label={i18n.t("login.username")}>
            <input
              autoComplete="username"
              autoFocus
              disabled={isLoginPending}
              value={username}
              onChange={(event) => setUsername(event.target.value)}
            />
          </Field>
          <Field label={i18n.t("login.password")}>
            <input
              autoComplete="current-password"
              disabled={isLoginPending}
              type="password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
            />
          </Field>
          <button
            className="primary-button login-submit"
            disabled={isLoginPending}
            type="submit"
          >
            {isLoginPending ? i18n.t("login.loggingIn") : i18n.t("login.login")}
          </button>
        </form>
        <div className="login-reset-panel">
          <button
            className="link-button login-reset-toggle"
            disabled={isLoginPending}
            onClick={() => setIsResetOpen((value) => !value)}
            type="button"
          >
            {isResetOpen ? i18n.t("login.backToLogin") : i18n.t("login.forgotPassword")}
          </button>
          {isResetOpen ? (
            <form className="login-form" onSubmit={submitResetPassword}>
              <p className="login-reset-hint">{i18n.t("login.resetHint")}</p>
              <button
                className="ghost-button"
                disabled={isLoginPending || !username.trim()}
                onClick={() => void submitResetCode()}
                type="button"
              >
                {i18n.t("login.sendCode")}
              </button>
              <Field label={i18n.t("login.resetCode")}>
                <input
                  autoComplete="one-time-code"
                  disabled={isLoginPending}
                  inputMode="numeric"
                  maxLength={6}
                  value={resetCode}
                  onChange={(event) =>
                    setResetCode(event.target.value.replace(/\D/g, "").slice(0, 6))
                  }
                />
              </Field>
              <Field label={i18n.t("login.newPassword")}>
                <input
                  autoComplete="new-password"
                  disabled={isLoginPending}
                  type="password"
                  value={resetPassword}
                  onChange={(event) => setResetPassword(event.target.value)}
                />
              </Field>
              <button
                className="primary-button login-submit"
                disabled={isLoginPending || !canResetPassword}
                type="submit"
              >
                {i18n.t("login.resetPassword")}
              </button>
            </form>
          ) : null}
        </div>
      </section>
    </main>
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
  const [stockBalances, setStockBalances] = useState<StockBalanceRow[]>([]);
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
        setStockBalances(
          await invoke<StockBalanceRow[]>("list_stock_balances", {
            query: {},
          }),
        );
      }
      if (
        editor === "businessSettings" ||
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
        if (nextStatus.runtime.mode === "host") {
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

  useEffect(() => {
    document.body.classList.add("editor-window");
    return () => document.body.classList.remove("editor-window");
  }, []);

  useEffect(() => {
    void loadEditorData();
  }, []);

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
  } else if (editor === "businessSettings") {
    content = (
      <BusinessSettingsEditor
        disabled={isSaving || isLoading || !systemSettings}
        settings={systemSettings}
        onSave={(request) =>
          runSettingsEditorAction("系统设置已保存", () =>
            invoke("save_system_settings", { request }),
          )
        }
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
          runSettingsEditorAction(message, () => Promise.resolve())
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
  } else if (editor === "adjustment") {
    content = (
      <AdjustmentEditor
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
      <Field label="默认单价">
        <input
          min="0"
          type="number"
          value={draft.defaultPrice}
          onChange={(e) =>
            setDraft({ ...draft, defaultPrice: Number(e.target.value) })
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
    categoryId: categories[0]?.id ?? "",
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
        categoryId: rule.categoryId,
        periodMonth: rule.periodMonth,
        amountLimit: rule.amountLimit,
        enabled: rule.enabled,
      });
    } else {
      setDraft((current) => ({
        ...current,
        departmentId: current.departmentId || departments[0]?.id || "",
        categoryId: current.categoryId || categories[0]?.id || "",
        periodMonth: current.periodMonth || periodMonth,
      }));
    }
  }, [categories, departments, mode, periodMonth, rule]);
  return (
    <EditorForm
      disabled={
        disabled ||
        !draft.departmentId ||
        !draft.categoryId ||
        !draft.periodMonth
      }
      saveLabel="保存预算"
      onSave={() => onSave(draft)}
    >
      <Field label="月份">
        <input
          type="month"
          value={draft.periodMonth}
          onChange={(e) => setDraft({ ...draft, periodMonth: e.target.value })}
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
          value={draft.categoryId}
          onChange={(e) => setDraft({ ...draft, categoryId: e.target.value })}
        >
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

function BusinessSettingsEditor({
  disabled,
  onSave,
  settings,
}: {
  disabled: boolean;
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
      disabled={disabled || !draft.hotelName.trim()}
      saveLabel="保存系统设置"
      onSave={() => onSave(draft)}
    >
      <Field label="酒店名称">
        <input
          autoFocus
          value={draft.hotelName}
          onChange={(event) => setDraft({ ...draft, hotelName: event.target.value })}
        />
      </Field>
      <Field label="当前账期">
        <input
          type="month"
          value={draft.currentPeriod}
          onChange={(event) => setDraft({ ...draft, currentPeriod: event.target.value })}
        />
      </Field>
      <Field label="默认月份">
        <input
          type="month"
          value={draft.defaultMonth}
          onChange={(event) => setDraft({ ...draft, defaultMonth: event.target.value })}
        />
      </Field>
      <Field label="数量小数位">
        <input
          max="6"
          min="0"
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
          onChange={(event) =>
            setDraft({ ...draft, smtpEnabled: event.target.checked })
          }
          type="checkbox"
        />
        启用邮箱验证码找回密码
      </label>
      <Field label="SMTP 主机">
        <input
          placeholder="smtp.example.com"
          value={draft.smtpHost}
          onChange={(event) => setDraft({ ...draft, smtpHost: event.target.value })}
        />
      </Field>
      <Field label="SMTP 端口">
        <input
          max="65535"
          min="1"
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
          value={draft.smtpUsername}
          onChange={(event) =>
            setDraft({ ...draft, smtpUsername: event.target.value })
          }
        />
      </Field>
      <Field label="SMTP 授权码">
        <input
          autoComplete="new-password"
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
          value={draft.smtpFromEmail}
          onChange={(event) =>
            setDraft({ ...draft, smtpFromEmail: event.target.value })
          }
        />
      </Field>
      <Field label="发件名称">
        <input
          value={draft.smtpFromName}
          onChange={(event) =>
            setDraft({ ...draft, smtpFromName: event.target.value })
          }
        />
      </Field>
    </EditorForm>
  );
}

type ConnectionWizardStep =
  | "role"
  | "hostConfirm"
  | "hostReady"
  | "discover"
  | "manual"
  | "pair"
  | "clientReady";

function ConnectionWizard({
  clientConnections,
  disabled,
  hostStatus,
  onDiscover,
  onEnableHost,
  onFinish,
  onPair,
  onRefreshHost,
  onTest,
  status,
}: {
  clientConnections: ClientConnectionInfo[];
  disabled: boolean;
  hostStatus: HostServiceStatus | null;
  onDiscover: (hostPort: number) => Promise<HostDiscoveryResult[]>;
  onEnableHost: () => Promise<HostServiceStatus>;
  onFinish: (message: string) => Promise<void>;
  onPair: (request: {
    hostAddress: string;
    hostPort: number;
    pairCode: string;
    clientName: string;
    clientDeviceId: string;
  }) => Promise<RuntimeConfig>;
  onRefreshHost: () => Promise<void>;
  onTest: (
    hostAddress: string,
    hostPort: number,
  ) => Promise<HostConnectionTestResult>;
  status: AppStatus | null;
}) {
  const [step, setStep] = useState<ConnectionWizardStep>("role");
  const [hosts, setHosts] = useState<HostDiscoveryResult[]>([]);
  const [selectedHost, setSelectedHost] = useState<HostDiscoveryResult | null>(
    null,
  );
  const [hostAddress, setHostAddress] = useState(
    status?.runtime.hostAddress ?? "",
  );
  const [hostPort, setHostPort] = useState(status?.runtime.hostPort ?? 17871);
  const [pairCode, setPairCode] = useState("");
  const [clientName, setClientName] = useState(() =>
    defaultClientName(detectDesktopPlatform()),
  );
  const [clientDeviceId, setClientDeviceId] = useState(
    status?.runtime.clientDeviceId || defaultClientDeviceId(),
  );
  const [testResult, setTestResult] =
    useState<HostConnectionTestResult | null>(null);
  const [isBusy, setIsBusy] = useState(false);
  const [localError, setLocalError] = useState<string | null>(null);
  const effectiveHostStatus = hostStatus;
  const effectiveHostAddress =
    selectedHost?.hostAddress || hostAddress.trim() || "";
  const effectiveHostPort = selectedHost?.hostPort || hostPort || 17871;

  async function discover() {
    setStep("discover");
    setIsBusy(true);
    setLocalError(null);
    setTestResult(null);
    try {
      const results = await onDiscover(hostPort || 17871);
      setHosts(results);
      if (results.length === 1) {
        setSelectedHost(results[0]);
      }
    } catch (err) {
      setHosts([]);
      setLocalError(formatError(err));
    } finally {
      setIsBusy(false);
    }
  }

  async function enableHost() {
    setIsBusy(true);
    setLocalError(null);
    try {
      await onEnableHost();
      setStep("hostReady");
    } catch (err) {
      setLocalError(formatError(err));
    } finally {
      setIsBusy(false);
    }
  }

  async function testManualHost() {
    if (!hostAddress.trim()) return;
    setIsBusy(true);
    setLocalError(null);
    try {
      const result = await onTest(hostAddress.trim(), hostPort || 17871);
      setTestResult(result);
      if (result.ok) {
        setSelectedHost({
          hostAddress: hostAddress.trim(),
          hostPort: hostPort || 17871,
          appName: result.appName ?? "Aster",
          appVersion: result.appVersion ?? "-",
          schemaVersion: result.schemaVersion ?? 0,
          message: result.message,
        });
        setStep("pair");
      }
    } catch (err) {
      setTestResult({
        ok: false,
        message: formatError(err),
        appName: null,
        appVersion: null,
        schemaVersion: null,
      });
    } finally {
      setIsBusy(false);
    }
  }

  async function pairHost() {
    if (!effectiveHostAddress || pairCode.length !== 6 || !clientName.trim()) {
      return;
    }
    setIsBusy(true);
    setLocalError(null);
    try {
      await onPair({
        hostAddress: effectiveHostAddress,
        hostPort: effectiveHostPort,
        pairCode,
        clientName: clientName.trim(),
        clientDeviceId: clientDeviceId.trim() || defaultClientDeviceId(),
      });
      setStep("clientReady");
    } catch (err) {
      setLocalError(formatError(err));
    } finally {
      setIsBusy(false);
    }
  }

  return (
    <div className="connection-wizard">
      <div className="wizard-header">
        <span>多电脑连接</span>
        <h2>{wizardStepTitle(step)}</h2>
      </div>

      {localError ? <div className="error-banner">{localError}</div> : null}

      {step === "role" ? (
        <div className="wizard-choice-grid">
          <button
            className="wizard-choice"
            disabled={disabled || isBusy}
            type="button"
            onClick={() => setStep("hostConfirm")}
          >
            <strong>这台作为主电脑</strong>
            <span>正式库存数据保存在这台电脑，其他电脑连接过来一起使用。</span>
          </button>
          <button
            className="wizard-choice"
            disabled={disabled || isBusy}
            type="button"
            onClick={() => void discover()}
          >
            <strong>连接到主电脑</strong>
            <span>这台电脑连接已有主电脑，共用同一套库存数据。</span>
          </button>
        </div>
      ) : null}

      {step === "hostConfirm" ? (
        <div className="wizard-panel">
          <p>
            正式库存数据将保存在这台电脑。其他电脑需要输入这台电脑显示的配对码后才能连接。
          </p>
          <div className="wizard-actions">
            <button
              className="ghost-button"
              disabled={isBusy}
              type="button"
              onClick={() => setStep("role")}
            >
              返回
            </button>
            <button
              className="primary-button"
              disabled={disabled || isBusy}
              type="button"
              onClick={() => void enableHost()}
            >
              {isBusy ? "开启中..." : "开启共享"}
            </button>
          </div>
        </div>
      ) : null}

      {step === "hostReady" ? (
        <div className="wizard-panel">
          <div className="pair-code-card">
            <span>给其他电脑输入的配对码</span>
            <strong>{effectiveHostStatus?.pairCode ?? "------"}</strong>
          </div>
          <dl className="wizard-summary">
            <div>
              <dt>共享状态</dt>
              <dd>{effectiveHostStatus?.message ?? "主电脑共享已开启"}</dd>
            </div>
            <div>
              <dt>连接地址</dt>
              <dd>
                {effectiveHostStatus?.running
                  ? `${effectiveHostStatus.bindAddress}:${effectiveHostStatus.port}`
                  : "-"}
              </dd>
            </div>
            <div>
              <dt>已连接其他电脑</dt>
              <dd>{clientConnections.length} 台</dd>
            </div>
          </dl>
          <div className="wizard-actions">
            <button
              className="ghost-button"
              disabled={isBusy}
              type="button"
              onClick={() => void onRefreshHost()}
            >
              刷新状态
            </button>
            <button
              className="primary-button"
              disabled={disabled || isBusy}
              type="button"
              onClick={() => void onFinish("这台电脑已开启主电脑共享")}
            >
              完成
            </button>
          </div>
        </div>
      ) : null}

      {step === "discover" ? (
        <div className="wizard-panel">
          <div className="wizard-toolbar">
            <p>
              {isBusy
                ? "正在搜索局域网内的主电脑..."
                : hosts.length > 0
                  ? "选择要连接的主电脑。"
                  : "没有找到主电脑。请确认主电脑已开启共享。"}
            </p>
            <button
              className="ghost-button"
              disabled={disabled || isBusy}
              type="button"
              onClick={() => void discover()}
            >
              重新搜索
            </button>
          </div>
          {hosts.length > 0 ? (
            <div className="discovery-list">
              {hosts.map((host) => (
                <button
                  className={
                    selectedHost?.hostAddress === host.hostAddress &&
                    selectedHost?.hostPort === host.hostPort
                      ? "discovery-item selected"
                      : "discovery-item"
                  }
                  key={`${host.hostAddress}:${host.hostPort}`}
                  type="button"
                  onClick={() => {
                    setSelectedHost(host);
                    setHostAddress(host.hostAddress);
                    setHostPort(host.hostPort);
                    setStep("pair");
                  }}
                >
                  <strong>{host.hostAddress}</strong>
                  <span>
                    {host.appName} {host.appVersion} · Schema{" "}
                    {host.schemaVersion}
                  </span>
                  <span>{host.message}</span>
                </button>
              ))}
            </div>
          ) : null}
          <div className="wizard-actions">
            <button
              className="ghost-button"
              disabled={isBusy}
              type="button"
              onClick={() => setStep("role")}
            >
              返回
            </button>
            <button
              className="ghost-button"
              disabled={isBusy}
              type="button"
              onClick={() => setStep("manual")}
            >
              手动输入地址
            </button>
          </div>
        </div>
      ) : null}

      {step === "manual" ? (
        <div className="wizard-panel">
          <Field label="主电脑地址">
            <input
              autoFocus
              disabled={isBusy}
              placeholder="例如 192.168.1.20"
              value={hostAddress}
              onChange={(event) => setHostAddress(event.target.value)}
            />
          </Field>
          <Field label="端口">
            <input
              disabled={isBusy}
              max="65535"
              min="1024"
              type="number"
              value={hostPort}
              onChange={(event) => setHostPort(Number(event.target.value))}
            />
          </Field>
          {testResult ? (
            <div
              className={
                testResult.ok
                  ? "settings-result success"
                  : "settings-result warning"
              }
            >
              <strong>{testResult.message}</strong>
              <span>
                {testResult.appName ?? "-"} {testResult.appVersion ?? ""}
              </span>
            </div>
          ) : null}
          <div className="wizard-actions">
            <button
              className="ghost-button"
              disabled={isBusy}
              type="button"
              onClick={() => setStep("discover")}
            >
              返回搜索
            </button>
            <button
              className="primary-button"
              disabled={disabled || isBusy || !hostAddress.trim()}
              type="button"
              onClick={() => void testManualHost()}
            >
              测试并继续
            </button>
          </div>
        </div>
      ) : null}

      {step === "pair" ? (
        <div className="wizard-panel">
          <dl className="wizard-summary">
            <div>
              <dt>主电脑</dt>
              <dd>
                {effectiveHostAddress}:{effectiveHostPort}
              </dd>
            </div>
          </dl>
          <Field label="配对码">
            <input
              autoFocus
              disabled={isBusy}
              inputMode="numeric"
              maxLength={6}
              placeholder="输入主电脑显示的 6 位数字"
              value={pairCode}
              onChange={(event) =>
                setPairCode(event.target.value.replace(/\D/g, "").slice(0, 6))
              }
            />
          </Field>
          <Field label="这台电脑名称">
            <input
              disabled={isBusy}
              value={clientName}
              onChange={(event) => setClientName(event.target.value)}
            />
          </Field>
          <Field label="设备标识">
            <input
              disabled={isBusy}
              value={clientDeviceId}
              onChange={(event) => setClientDeviceId(event.target.value)}
            />
          </Field>
          <div className="wizard-actions">
            <button
              className="ghost-button"
              disabled={isBusy}
              type="button"
              onClick={() => setStep(hosts.length > 0 ? "discover" : "manual")}
            >
              返回
            </button>
            <button
              className="primary-button"
              disabled={
                disabled || isBusy || pairCode.length !== 6 || !clientName.trim()
              }
              type="button"
              onClick={() => void pairHost()}
            >
              {isBusy ? "连接中..." : "连接"}
            </button>
          </div>
        </div>
      ) : null}

      {step === "clientReady" ? (
        <div className="wizard-panel">
          <div className="settings-result success">
            <strong>已连接到主电脑</strong>
            <span>
              {effectiveHostAddress}:{effectiveHostPort}
            </span>
            <span>以后打开应用会自动使用主电脑上的库存数据。</span>
          </div>
          <div className="wizard-actions">
            <button
              className="primary-button"
              disabled={disabled || isBusy}
              type="button"
              onClick={() => void onFinish("已连接到主电脑")}
            >
              完成
            </button>
          </div>
        </div>
      ) : null}
    </div>
  );
}

function wizardStepTitle(step: ConnectionWizardStep) {
  if (step === "hostConfirm") return "把这台电脑设为主电脑";
  if (step === "hostReady") return "主电脑已开启";
  if (step === "discover") return "搜索主电脑";
  if (step === "manual") return "手动连接主电脑";
  if (step === "pair") return "输入配对码";
  if (step === "clientReady") return "连接完成";
  return "这台电脑要怎么使用？";
}

function defaultClientName(platform: string) {
  if (platform === "windows") return "Windows 电脑";
  if (platform === "macos") return "macOS 电脑";
  return "Aster 电脑";
}

function defaultClientDeviceId() {
  const stored = window.localStorage.getItem("aster.clientDeviceId");
  if (stored) return stored;
  const generated = `device-${Date.now().toString(36)}-${Math.random()
    .toString(36)
    .slice(2, 8)}`;
  window.localStorage.setItem("aster.clientDeviceId", generated);
  return generated;
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
      disabled={disabled || pairCode.length !== 6 || !clientName.trim()}
      saveLabel="完成配对"
      onSave={() => onSave({ pairCode, clientName, clientDeviceId })}
    >
      <Field label="配对码">
        <input
          autoFocus
          inputMode="numeric"
          maxLength={6}
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
          <span>创建时间：{preview.metadata.createdAt}</span>
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
  departments: Department[];
  disabled: boolean;
  documentType: "inbound" | "outbound";
  items: Item[];
  onCreateApproval: (request: CreateApprovalRequestDraft) => Promise<void>;
  onSaveDraft: (request: StockDocumentDraft) => Promise<void>;
  onSubmit: (request: StockDocumentDraft) => Promise<void>;
  suppliers: Supplier[];
}) {
  const emptyLine: StockDocumentLineDraft = {
    itemId: "",
    quantity: 1,
    unitPrice: 0,
    amount: null,
    remark: "",
  };
  const [draft, setDraft] = useState<StockDocumentDraft>({
    documentId: undefined,
    documentType,
    businessDate: todayString(),
    departmentId: "",
    supplierId: "",
    handler: "",
    purpose: "",
    remark: "",
    approvalRequestId: "",
    lines: [emptyLine],
  });
  const [scanCode, setScanCode] = useState("");
  const isOutbound = documentType === "outbound";
  const totalAmount = draft.lines.reduce(
    (sum, line) => sum + effectiveLineAmount(line),
    0,
  );
  const balanceByItemId = useMemo(
    () => new Map(balances.map((balance) => [balance.itemId, balance])),
    [balances],
  );

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
          updated.unitPrice = item?.defaultPrice ?? updated.unitPrice;
          updated.amount = null;
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
      unitPrice: item.defaultPrice,
      amount: null,
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
    <div className="editor-document">
      <div className="editor-form-grid">
        <Field label="业务日期">
          <input
            type="date"
            value={draft.businessDate}
            onChange={(e) =>
              setDraft({ ...draft, businessDate: e.target.value })
            }
          />
        </Field>
        {isOutbound ? (
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
            value={draft.handler}
            onChange={(e) => setDraft({ ...draft, handler: e.target.value })}
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
        {isOutbound ? (
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
              <th>单价</th>
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
                <td>
                  <input
                    className="table-input"
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
                    className="table-input"
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
        {isOutbound ? (
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

function ItemSearchSelect({
  disabled,
  items,
  onChange,
  placeholder = "搜索编码、条码或物品名称",
  value,
}: {
  disabled: boolean;
  items: Item[];
  onChange: (itemId: string) => void;
  placeholder?: string;
  value: string;
}) {
  const inputRef = useRef<HTMLInputElement | null>(null);
  const [menuStyle, setMenuStyle] = useState<CSSProperties>({});
  const selectedItem = items.find((item) => item.id === value);
  const selectedLabel = selectedItem ? itemDisplayName(selectedItem) : "";
  const [query, setQuery] = useState(selectedLabel);
  const [open, setOpen] = useState(false);
  const normalizedQuery = normalizeSearchText(query);
  const options = useMemo(() => {
    const scored = items
      .map((item, index) => {
        const haystack = itemSearchText(item);
        const code = normalizeSearchText(item.code);
        const barcode = normalizeSearchText(item.barcode);
        const name = normalizeSearchText(item.name);
        const spec = normalizeSearchText(item.spec);
        if (!normalizedQuery) return { item, index, score: index };
        if (code.startsWith(normalizedQuery)) return { item, index, score: 0 };
        if (barcode.startsWith(normalizedQuery)) {
          return { item, index, score: 1 };
        }
        if (name.startsWith(normalizedQuery)) return { item, index, score: 2 };
        if (haystack.includes(normalizedQuery)) {
          return { item, index, score: spec.includes(normalizedQuery) ? 4 : 3 };
        }
        return null;
      })
      .filter((entry): entry is { item: Item; index: number; score: number } =>
        Boolean(entry),
      )
      .sort((left, right) => left.score - right.score || left.index - right.index)
      .slice(0, 30);
    return scored.map((entry) => entry.item);
  }, [items, normalizedQuery]);

  useEffect(() => {
    if (!open) {
      setQuery(selectedLabel);
    }
  }, [open, selectedLabel]);

  useEffect(() => {
    if (!open) return;
    function updateMenuPosition() {
      const rect = inputRef.current?.getBoundingClientRect();
      if (!rect) return;
      const viewportWidth = window.innerWidth;
      setMenuStyle({
        left: Math.min(rect.left, Math.max(8, viewportWidth - 428)),
        top: rect.bottom + 4,
        width: Math.min(420, Math.max(260, viewportWidth - 16)),
      });
    }
    updateMenuPosition();
    window.addEventListener("resize", updateMenuPosition);
    window.addEventListener("scroll", updateMenuPosition, true);
    return () => {
      window.removeEventListener("resize", updateMenuPosition);
      window.removeEventListener("scroll", updateMenuPosition, true);
    };
  }, [open]);

  function selectItem(item: Item) {
    onChange(item.id);
    setQuery(itemDisplayName(item));
    setOpen(false);
  }

  function clearSelection() {
    onChange("");
    setQuery("");
    setOpen(true);
  }

  return (
    <div className="item-search-select">
      <div className="item-search-input-row">
        <input
          aria-label="搜索物品"
          className="table-input item-search-input"
          disabled={disabled}
          ref={inputRef}
          onBlur={() => {
            window.setTimeout(() => setOpen(false), 120);
          }}
          onChange={(event) => {
            setQuery(event.target.value);
            setOpen(true);
            if (!event.target.value.trim()) {
              onChange("");
            }
          }}
          onFocus={() => setOpen(true)}
          placeholder={placeholder}
          value={query}
        />
        {value ? (
          <button
            aria-label="清空物品"
            className="item-search-clear"
            disabled={disabled}
            onMouseDown={(event) => event.preventDefault()}
            onClick={clearSelection}
            type="button"
          >
            x
          </button>
        ) : null}
      </div>
      {open && !disabled ? (
        <div className="item-search-menu" style={menuStyle}>
          {options.length ? (
            options.map((item) => (
              <button
                className={item.id === value ? "selected" : ""}
                key={item.id}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => selectItem(item)}
                type="button"
              >
                <strong>{itemDisplayName(item)}</strong>
                <span>
                  {[
                    item.barcode ? `条码 ${item.barcode}` : null,
                    item.spec,
                    item.unitName,
                  ]
                    .filter(Boolean)
                    .join(" · ") || "未设置规格"}
                </span>
              </button>
            ))
          ) : (
            <div className="item-search-empty">没有匹配的物品</div>
          )}
        </div>
      ) : null}
    </div>
  );
}

function AdjustmentEditor({
  disabled,
  items,
  onSubmit,
}: {
  disabled: boolean;
  items: Item[];
  onSubmit: (request: AdjustmentDraft) => Promise<void>;
}) {
  const emptyLine: AdjustmentLineDraft = {
    itemId: "",
    direction: "out",
    quantity: 1,
    unitPrice: 0,
    amount: null,
    remark: "",
  };
  const [draft, setDraft] = useState<AdjustmentDraft>({
    businessDate: todayString(),
    adjustmentType: "damage",
    handler: "",
    reason: "",
    lines: [emptyLine],
  });
  const totalAmount = draft.lines.reduce(
    (sum, line) => sum + effectiveLineAmount(line),
    0,
  );
  const correction = draft.adjustmentType === "correction";

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
    <div className="editor-document">
      <div className="editor-form-grid">
        <Field label="调整日期">
          <input
            type="date"
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
            value={draft.handler}
            onChange={(e) => setDraft({ ...draft, handler: e.target.value })}
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
              <th>单价</th>
              <th>金额</th>
              <th>备注</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            {draft.lines.map((line, index) => (
              <tr key={index}>
                <td>
                  <select
                    className="table-input"
                    value={line.itemId}
                    onChange={(e) =>
                      updateLine(index, { itemId: e.target.value })
                    }
                  >
                    <option value="">请选择物品</option>
                    {items.map((item) => (
                      <option key={item.id} value={item.id}>
                        {item.code} · {item.name}
                      </option>
                    ))}
                  </select>
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
  const [businessDate, setBusinessDate] = useState(todayString());
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
          type="date"
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
    <div className="editor-document">
      <div className="editor-toolbar">
        <Field label="盘点单">
          <select
            value={detail?.document.id ?? ""}
            onChange={(e) => onSelect(e.target.value)}
          >
            {stocktakes.map((stocktake) => (
              <option key={stocktake.id} value={stocktake.id}>
                {stocktake.documentNo} · {stocktake.businessDate} ·{" "}
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
  disabled,
  onSave,
  saveLabel,
}: {
  children: React.ReactNode;
  disabled: boolean;
  onSave: () => Promise<void>;
  saveLabel: string;
}) {
  return (
    <div className="editor-form">
      <div className="editor-form-grid">{children}</div>
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

function UsersPage({
  currentUser,
  onToggle,
  users,
}: {
  currentUser: CurrentUser | null;
  onToggle: (userId: string, enabled: boolean) => Promise<void>;
  users: UserAccount[];
}) {
  const isAdmin =
    currentUser?.roles.some((role) => role.code === "admin") ?? false;

  if (!isAdmin) {
    return (
      <section className="module-panel placeholder-panel">
        <h2>需要管理员权限</h2>
        <p>用户管理、角色分配和危险操作控制需要管理员登录后使用。</p>
      </section>
    );
  }

  return (
    <section className="table-panel">
      <div className="table-toolbar">
        <h2>用户列表</h2>
        <button
          className="primary-button"
          onClick={() => openEditorWindow("user")}
        >
          新增用户
        </button>
      </div>
      <table>
        <thead>
          <tr>
            <th>用户名</th>
            <th>显示名称</th>
            <th>邮箱</th>
            <th>所属部门</th>
            <th>角色</th>
            <th>状态</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          {users.map((user) => (
            <tr key={user.id}>
              <td>{user.username}</td>
              <td>{user.displayName}</td>
              <td>{user.email ?? "-"}</td>
              <td>{user.departmentName ?? "-"}</td>
              <td>{user.roles.map((role) => role.name).join("、") || "-"}</td>
              <td>
                <Status enabled={user.enabled} />
              </td>
              <td className="row-actions">
                <button
                  onClick={() =>
                    openEditorWindow("user", { mode: "edit", id: user.id })
                  }
                >
                  编辑
                </button>
                <button onClick={() => onToggle(user.id, !user.enabled)}>
                  {user.enabled ? "停用" : "启用"}
                </button>
              </td>
            </tr>
          ))}
          {users.length === 0 ? <EmptyRow colSpan={7} /> : null}
        </tbody>
      </table>
    </section>
  );
}

function ItemsPage({
  canWrite,
  categories,
  itemSearch,
  items,
  onSearch,
  onToggle,
  suppliers,
  units,
}: {
  canWrite: boolean;
  categories: OptionRecord[];
  itemSearch: string;
  items: Item[];
  onSearch: (search: string) => Promise<void>;
  onToggle: (
    id: string,
    enabled: boolean,
    expectedUpdatedAt: string,
  ) => Promise<void>;
  suppliers: OptionRecord[];
  units: OptionRecord[];
}) {
  const [search, setSearch] = useState(itemSearch);

  return (
    <section className="table-panel">
      <div className="table-toolbar">
        <form
          onSubmit={(e) => {
            e.preventDefault();
            onSearch(search);
          }}
        >
          <input
            placeholder="搜索编码、名称、规格"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
          <button className="ghost-button">搜索</button>
        </form>
        <button
          className="primary-button"
          disabled={!canWrite}
          onClick={() => openEditorWindow("item")}
        >
          新增物品
        </button>
      </div>
      <table>
        <thead>
          <tr>
            <th>编码</th>
            <th>条码</th>
            <th>名称</th>
            <th>分类</th>
            <th>规格</th>
            <th>单位</th>
            <th>单价</th>
            <th>供应商</th>
            <th>状态</th>
            <th>操作</th>
          </tr>
        </thead>
        <PaginatedTable colSpan={10} getRowKey={(item) => item.id} rows={items}>
          {(item) => (
            <>
              <td>{item.code}</td>
              <td>{item.barcode ?? "-"}</td>
              <td>{item.name}</td>
              <td>
                {item.categoryName ?? optionName(categories, item.categoryId)}
              </td>
              <td>{item.spec ?? "-"}</td>
              <td>{item.unitName ?? optionName(units, item.unitId)}</td>
              <td>{formatMoney(item.defaultPrice)}</td>
              <td>
                {item.supplierName ?? optionName(suppliers, item.supplierId)}
              </td>
              <td>
                <Status enabled={item.enabled} />
              </td>
              <td className="row-actions">
                <button
                  disabled={!canWrite}
                  onClick={() =>
                    openEditorWindow("item", { mode: "edit", id: item.id })
                  }
                >
                  编辑
                </button>
                <button
                  disabled={!canWrite}
                  onClick={() =>
                    onToggle(item.id, !item.enabled, item.updatedAt)
                  }
                >
                  {item.enabled ? "停用" : "启用"}
                </button>
              </td>
            </>
          )}
        </PaginatedTable>
      </table>
    </section>
  );
}

function DepartmentsPage({
  canWrite,
  departments,
  onToggle,
}: {
  canWrite: boolean;
  departments: Department[];
  onToggle: (
    id: string,
    enabled: boolean,
    expectedUpdatedAt: string,
  ) => Promise<void>;
}) {
  return (
    <MasterTablePanel
      actions={
        <button
          className="primary-button"
          disabled={!canWrite}
          onClick={() => openEditorWindow("department")}
        >
          新增部门
        </button>
      }
      description="部门用于出库领用和部门报表统计。"
      hideHeading
      title="部门管理"
    >
      <table>
        <thead>
          <tr>
            <th>编码</th>
            <th>名称</th>
            <th>负责人</th>
            <th>排序</th>
            <th>状态</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          {departments.map((item) => (
            <tr key={item.id}>
              <td>{item.code}</td>
              <td>{item.name}</td>
              <td>{item.manager ?? "-"}</td>
              <td>{item.sortOrder}</td>
              <td>
                <Status enabled={item.enabled} />
              </td>
              <td className="row-actions">
                <button
                  disabled={!canWrite}
                  onClick={() =>
                    openEditorWindow("department", {
                      mode: "edit",
                      id: item.id,
                    })
                  }
                >
                  编辑
                </button>
                <button
                  disabled={!canWrite}
                  onClick={() =>
                    onToggle(item.id, !item.enabled, item.updatedAt)
                  }
                >
                  {item.enabled ? "停用" : "启用"}
                </button>
              </td>
            </tr>
          ))}
          {departments.length === 0 ? <EmptyRow colSpan={6} /> : null}
        </tbody>
      </table>
    </MasterTablePanel>
  );
}

function CategoriesPage({
  canWrite,
  categories,
  onToggle,
}: {
  canWrite: boolean;
  categories: Category[];
  onToggle: (
    id: string,
    enabled: boolean,
    expectedUpdatedAt: string,
  ) => Promise<void>;
}) {
  function parentName(parentId?: string | null) {
    if (!parentId) return "大类";
    return categories.find((item) => item.id === parentId)?.name ?? "-";
  }

  return (
    <MasterTablePanel
      actions={
        <button
          className="primary-button"
          disabled={!canWrite}
          onClick={() => openEditorWindow("category")}
        >
          新增分类
        </button>
      }
      description="分类支持大类和小类，用于物品筛选、预算规则和报表统计。"
      hideHeading
      title="分类管理"
    >
      <table>
        <thead>
          <tr>
            <th>名称</th>
            <th>类型</th>
            <th>上级分类</th>
            <th>排序</th>
            <th>状态</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          {categories.map((item) => (
            <tr key={item.id}>
              <td>{item.name}</td>
              <td>{item.parentId ? "小类" : "大类"}</td>
              <td>{parentName(item.parentId)}</td>
              <td>{item.sortOrder}</td>
              <td>
                <Status enabled={item.enabled} />
              </td>
              <td className="row-actions">
                <button
                  disabled={!canWrite}
                  onClick={() =>
                    openEditorWindow("category", { mode: "edit", id: item.id })
                  }
                >
                  编辑
                </button>
                <button
                  disabled={!canWrite}
                  onClick={() =>
                    onToggle(item.id, !item.enabled, item.updatedAt)
                  }
                >
                  {item.enabled ? "停用" : "启用"}
                </button>
              </td>
            </tr>
          ))}
          {categories.length === 0 ? <EmptyRow colSpan={6} /> : null}
        </tbody>
      </table>
    </MasterTablePanel>
  );
}

function SimpleNamePage({
  canWrite,
  description,
  items,
  onToggle,
  title,
}: {
  canWrite: boolean;
  description: string;
  fields: string[];
  items: (Category | Unit)[];
  onToggle: (
    id: string,
    enabled: boolean,
    expectedUpdatedAt: string,
  ) => Promise<void>;
  title: string;
}) {
  return (
    <MasterTablePanel
      actions={
        <button
          className="primary-button"
          disabled={!canWrite}
          onClick={() => openEditorWindow("unit")}
        >
          新增单位
        </button>
      }
      description={description}
      hideHeading
      title={title}
    >
      <table>
        <thead>
          <tr>
            <th>名称</th>
            <th>排序</th>
            <th>状态</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          {items.map((item) => (
            <tr key={item.id}>
              <td>{item.name}</td>
              <td>{item.sortOrder}</td>
              <td>
                <Status enabled={item.enabled} />
              </td>
              <td className="row-actions">
                <button
                  disabled={!canWrite}
                  onClick={() =>
                    openEditorWindow("unit", { mode: "edit", id: item.id })
                  }
                >
                  编辑
                </button>
                <button
                  disabled={!canWrite}
                  onClick={() =>
                    onToggle(item.id, !item.enabled, item.updatedAt)
                  }
                >
                  {item.enabled ? "停用" : "启用"}
                </button>
              </td>
            </tr>
          ))}
          {items.length === 0 ? <EmptyRow colSpan={4} /> : null}
        </tbody>
      </table>
    </MasterTablePanel>
  );
}

function SuppliersPage({
  activeSupplier,
  canWrite,
  onSelect,
  onToggle,
  purchaseRecords,
  suppliers,
}: {
  activeSupplier: Supplier | null;
  canWrite: boolean;
  onSelect: (supplier: Supplier) => Promise<void>;
  onToggle: (
    id: string,
    enabled: boolean,
    expectedUpdatedAt: string,
  ) => Promise<void>;
  purchaseRecords: SupplierPurchaseRecord[];
  suppliers: Supplier[];
}) {
  const [activeTab, setActiveTab] = useState<"suppliers" | "purchases">(
    "suppliers",
  );

  async function openPurchaseRecords(supplier: Supplier) {
    await onSelect(supplier);
    setActiveTab("purchases");
  }

  return (
    <MasterTablePanel
      actions={
        <div className="supplier-toolbar">
          <div className="segmented supplier-tabs">
            <button
              className={activeTab === "suppliers" ? "selected" : ""}
              onClick={() => setActiveTab("suppliers")}
            >
              供应商档案
            </button>
            <button
              className={activeTab === "purchases" ? "selected" : ""}
              onClick={() => setActiveTab("purchases")}
            >
              采购记录
            </button>
          </div>
          <button
            className="primary-button"
            disabled={!canWrite}
            onClick={() => openEditorWindow("supplier")}
          >
            新增供应商
          </button>
        </div>
      }
      description="供应商用于入库单和采购记录查询。"
      hideHeading
      title="供应商管理"
    >
      {activeTab === "suppliers" ? (
        <table>
          <thead>
            <tr>
              <th>名称</th>
              <th>联系人</th>
              <th>电话</th>
              <th>地址</th>
              <th>状态</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            {suppliers.map((item) => (
              <tr key={item.id}>
                <td>{item.name}</td>
                <td>{item.contact ?? "-"}</td>
                <td>{item.phone ?? "-"}</td>
                <td>{item.address ?? "-"}</td>
                <td>
                  <Status enabled={item.enabled} />
                </td>
                <td className="row-actions">
                  <button
                    disabled={!canWrite}
                    onClick={() =>
                      openEditorWindow("supplier", {
                        mode: "edit",
                        id: item.id,
                      })
                    }
                  >
                    编辑
                  </button>
                  <button
                    disabled={!canWrite}
                    onClick={() =>
                      onToggle(item.id, !item.enabled, item.updatedAt)
                    }
                  >
                    {item.enabled ? "停用" : "启用"}
                  </button>
                  <button onClick={() => void openPurchaseRecords(item)}>
                    采购记录
                  </button>
                </td>
              </tr>
            ))}
            {suppliers.length === 0 ? <EmptyRow colSpan={6} /> : null}
          </tbody>
        </table>
      ) : (
        <div className="subtable supplier-purchase-panel">
          <div className="subtable-heading">
            <h3>
              {activeSupplier
                ? `${activeSupplier.name} 采购记录`
                : "供应商采购记录"}
            </h3>
          </div>
          <table>
            <thead>
              <tr>
                <th>日期</th>
                <th>单号</th>
                <th>物品</th>
                <th>规格</th>
                <th>单位</th>
                <th>数量</th>
                <th>单价</th>
                <th>金额</th>
                <th>备注</th>
              </tr>
            </thead>
            <tbody>
              {purchaseRecords.map((record, index) => (
                <tr
                  key={`${record.documentNo ?? "doc"}-${record.itemCode}-${index}`}
                >
                  <td>{record.movementDate}</td>
                  <td>{record.documentNo ?? "-"}</td>
                  <td>
                    {record.itemCode} · {record.itemName}
                  </td>
                  <td>{record.spec ?? "-"}</td>
                  <td>{record.unitName ?? "-"}</td>
                  <td>{record.quantity}</td>
                  <td>{formatMoney(record.unitPrice)}</td>
                  <td>{formatMoney(record.amount)}</td>
                  <td>{record.remark ?? "-"}</td>
                </tr>
              ))}
              {purchaseRecords.length === 0 ? <EmptyRow colSpan={9} /> : null}
            </tbody>
          </table>
        </div>
      )}
    </MasterTablePanel>
  );
}

function BudgetRulesPage({
  canManage,
  month,
  onMonthChange,
  onToggle,
  rules,
}: {
  canManage: boolean;
  month: string;
  onMonthChange: (month: string) => Promise<void>;
  onToggle: (
    id: string,
    enabled: boolean,
    expectedUpdatedAt: string,
  ) => Promise<void>;
  rules: BudgetRule[];
}) {
  return (
    <MasterTablePanel
      actions={
        <button
          className="primary-button"
          disabled={!canManage}
          onClick={() =>
            openEditorWindow("budget", { extra: { periodMonth: month } })
          }
        >
          新增预算
        </button>
      }
      description="预算按部门、分类和月份控制出库领用金额，超出预算时出库确认会被阻止。"
      hideHeading
      title="预算控制"
    >
      <div className="table-toolbar">
        <h2>预算规则</h2>
        <input
          type="month"
          value={month}
          onChange={(e) => onMonthChange(e.target.value)}
        />
      </div>
      <table>
        <thead>
          <tr>
            <th>月份</th>
            <th>部门</th>
            <th>分类</th>
            <th>预算</th>
            <th>已用</th>
            <th>剩余</th>
            <th>状态</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          {rules.map((rule) => {
            const remaining = rule.amountLimit - rule.usedAmount;
            return (
              <tr key={rule.id}>
                <td>{rule.periodMonth}</td>
                <td>{rule.departmentName}</td>
                <td>{rule.categoryName}</td>
                <td>{formatMoney(rule.amountLimit)}</td>
                <td>{formatMoney(rule.usedAmount)}</td>
                <td className={remaining < 0 ? "danger-cell" : ""}>
                  {formatMoney(remaining)}
                </td>
                <td>
                  <Status enabled={rule.enabled} />
                </td>
                <td className="row-actions">
                  <button
                    disabled={!canManage}
                    onClick={() =>
                      openEditorWindow("budget", {
                        mode: "edit",
                        id: rule.id,
                        extra: { periodMonth: rule.periodMonth },
                      })
                    }
                  >
                    编辑
                  </button>
                  <button
                    disabled={!canManage}
                    onClick={() =>
                      onToggle(rule.id, !rule.enabled, rule.updatedAt)
                    }
                  >
                    {rule.enabled ? "停用" : "启用"}
                  </button>
                </td>
              </tr>
            );
          })}
          {rules.length === 0 ? <EmptyRow colSpan={8} /> : null}
        </tbody>
      </table>
    </MasterTablePanel>
  );
}

function ApprovalsPage({
  approvals,
  canManage,
  onDecide,
}: {
  approvals: ApprovalRequest[];
  canManage: boolean;
  onDecide: (
    approvalId: string,
    approve: boolean,
    decisionNote: string,
  ) => Promise<void>;
}) {
  const [decisionNote, setDecisionNote] = useState("");
  return (
    <section className="table-panel">
      <div className="table-toolbar">
        <input
          value={decisionNote}
          onChange={(e) => setDecisionNote(e.target.value)}
          placeholder="审批意见"
        />
      </div>
      <table>
        <thead>
          <tr>
            <th>ID</th>
            <th>类型</th>
            <th>对象</th>
            <th>原因</th>
            <th>状态</th>
            <th>申请时间</th>
            <th>审批时间</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          {approvals.map((item) => (
            <tr key={item.id}>
              <td className="path-cell">{item.id}</td>
              <td>{approvalTypeLabel(item.entityType)}</td>
              <td>{item.entityId}</td>
              <td>{item.reason ?? "-"}</td>
              <td>
                <span
                  className={
                    item.status === "approved"
                      ? "status enabled"
                      : item.status === "pending"
                        ? "status"
                        : "status disabled"
                  }
                >
                  {approvalStatusLabel(item.status)}
                </span>
              </td>
              <td>{item.createdAt}</td>
              <td>{item.decidedAt ?? "-"}</td>
              <td className="row-actions">
                {item.status === "pending" ? (
                  <>
                    <button
                      disabled={!canManage}
                      onClick={() => onDecide(item.id, true, decisionNote)}
                    >
                      通过
                    </button>
                    <button
                      disabled={!canManage}
                      onClick={() => onDecide(item.id, false, decisionNote)}
                    >
                      驳回
                    </button>
                  </>
                ) : (
                  "-"
                )}
              </td>
            </tr>
          ))}
          {approvals.length === 0 ? <EmptyRow colSpan={8} /> : null}
        </tbody>
      </table>
    </section>
  );
}

function StockDocumentPage({
  canWrite,
  departments,
  documentType,
  documents,
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
    query ?? { documentType: isOutbound ? "outbound" : "inbound" },
  );
  useEffect(() => {
    if (query) setFilterDraft(query);
  }, [query]);

  const partyLabel = isOutbound ? "领用部门" : "供应商";
  const partyValue = isOutbound
    ? (filterDraft.departmentId ?? "")
    : (filterDraft.supplierId ?? "");
  const partyOptions = isOutbound ? departments : suppliers;

  function updateFilter(next: Partial<StockDocumentQuery>) {
    setFilterDraft({ ...filterDraft, ...next });
  }

  function applyFilters() {
    if (!query || !onQueryChange) return;
    onQueryChange({
      ...filterDraft,
      documentType: query.documentType,
      month: filterDraft.month || null,
      departmentId: isOutbound ? filterDraft.departmentId || null : null,
      supplierId: isOutbound ? null : filterDraft.supplierId || null,
      itemId: filterDraft.itemId || null,
      search: filterDraft.search?.trim() || null,
    });
  }

  function resetFilters() {
    if (!query || !onQueryChange) return;
    const nextQuery: StockDocumentQuery = { documentType: query.documentType };
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
        <div className="document-filters">
          <Field label="月份">
            <input
              type="month"
              value={filterDraft.month ?? ""}
              onChange={(e) => updateFilter({ month: e.target.value })}
            />
          </Field>
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
          <Field label="物品">
            <select
              value={filterDraft.itemId ?? ""}
              onChange={(e) => updateFilter({ itemId: e.target.value })}
            >
              <option value="">全部</option>
              {items.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.code} · {item.name}
                </option>
              ))}
            </select>
          </Field>
          <Field label="关键字">
            <input
              placeholder="单号/经办人/备注"
              value={filterDraft.search ?? ""}
              onChange={(e) => updateFilter({ search: e.target.value })}
            />
          </Field>
          <div className="document-filter-actions">
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
            <th>对象</th>
            <th>审批单</th>
            <th>数量</th>
            <th>金额</th>
            <th>状态</th>
            <th>操作</th>
          </tr>
        </thead>
        <PaginatedTable
          colSpan={8}
          getRowKey={(doc) => doc.id}
          rows={documents}
        >
          {(doc) => (
            <>
              <td>{doc.documentNo}</td>
              <td>{doc.businessDate}</td>
              <td>
                {doc.documentType === "outbound"
                  ? (doc.departmentName ?? "-")
                  : (doc.supplierName ?? "-")}
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

function StockBalancePage({
  balances,
  categories,
  items,
  onQueryChange,
  onViewMovements,
  query,
}: {
  balances: StockBalanceRow[];
  categories: Category[];
  items: Item[];
  onQueryChange: (query: StockBalanceQuery) => Promise<void>;
  onViewMovements: (itemId: string) => Promise<void>;
  query: StockBalanceQuery;
}) {
  const [filterDraft, setFilterDraft] = useState<StockBalanceQuery>(query);
  useEffect(() => setFilterDraft(query), [query]);

  function updateFilter(next: Partial<StockBalanceQuery>) {
    setFilterDraft({ ...filterDraft, ...next });
  }

  function applyFilters() {
    onQueryChange(filterDraft);
  }

  function resetFilters() {
    const nextQuery: StockBalanceQuery = {};
    setFilterDraft(nextQuery);
    onQueryChange(nextQuery);
  }

  return (
    <section className="table-panel">
      <div className="document-filters">
        <Field label="关键字">
          <input
            placeholder="编码/名称/规格"
            value={filterDraft.search ?? ""}
            onChange={(e) => updateFilter({ search: e.target.value })}
          />
        </Field>
        <Field label="分类">
          <select
            value={filterDraft.categoryId ?? ""}
            onChange={(e) => updateFilter({ categoryId: e.target.value })}
          >
            <option value="">全部</option>
            {categories.map((item) => (
              <option key={item.id} value={item.id}>
                {item.name}
              </option>
            ))}
          </select>
        </Field>
        <Field label="物品">
          <select
            value={filterDraft.itemId ?? ""}
            onChange={(e) => updateFilter({ itemId: e.target.value })}
          >
            <option value="">全部</option>
            {items.map((item) => (
              <option key={item.id} value={item.id}>
                {item.code} · {item.name}
              </option>
            ))}
          </select>
        </Field>
        <Field label="库存状态">
          <select
            value={filterDraft.stockStatus ?? ""}
            onChange={(e) =>
              updateFilter({
                stockStatus: e.target.value as StockBalanceQuery["stockStatus"],
              })
            }
          >
            <option value="">全部</option>
            <option value="normal">正常</option>
            <option value="low">低库存</option>
            <option value="negative">负库存</option>
          </select>
        </Field>
        <div className="document-filter-actions">
          <button className="ghost-button" onClick={resetFilters}>
            清空
          </button>
          <button className="primary-button" onClick={applyFilters}>
            筛选
          </button>
        </div>
      </div>
      <table>
        <thead>
          <tr>
            <th>编码</th>
            <th>物品</th>
            <th>规格</th>
            <th>单位</th>
            <th>库存</th>
            <th>金额</th>
            <th>均价</th>
            <th>预警线</th>
            <th>状态</th>
            <th>操作</th>
          </tr>
        </thead>
        <PaginatedTable
          colSpan={10}
          getRowKey={(row) => row.itemId}
          rows={balances}
        >
          {(row) => (
            <>
              <td>{row.itemCode}</td>
              <td>{row.itemName}</td>
              <td>{row.spec ?? "-"}</td>
              <td>{row.unitName ?? "-"}</td>
              <td>{row.quantity}</td>
              <td>{formatMoney(row.amount)}</td>
              <td>{formatMoney(row.averagePrice)}</td>
              <td>{row.warningQuantity}</td>
              <td>
                <span
                  className={`status ${row.stockStatus === "normal" ? "enabled" : "disabled"}`}
                >
                  {row.stockStatus === "normal"
                    ? "正常"
                    : row.stockStatus === "low"
                      ? "低库存"
                      : "负库存"}
                </span>
              </td>
              <td className="row-actions">
                <button onClick={() => onViewMovements(row.itemId)}>
                  流水
                </button>
              </td>
            </>
          )}
        </PaginatedTable>
      </table>
    </section>
  );
}

function StockMovementPage({
  items,
  movements,
  onQueryChange,
  query,
}: {
  items: Item[];
  movements: StockMovementRow[];
  onQueryChange: (query: StockMovementQuery) => Promise<void>;
  query: StockMovementQuery;
}) {
  const [filterDraft, setFilterDraft] = useState<StockMovementQuery>(query);
  useEffect(() => setFilterDraft(query), [query]);

  function updateFilter(next: Partial<StockMovementQuery>) {
    setFilterDraft({ ...filterDraft, ...next });
  }

  function applyFilters() {
    onQueryChange(filterDraft);
  }

  function resetFilters() {
    const nextQuery: StockMovementQuery = {};
    setFilterDraft(nextQuery);
    onQueryChange(nextQuery);
  }

  return (
    <section className="table-panel">
      <div className="document-filters">
        <Field label="关键字">
          <input
            placeholder="编码/名称/单号"
            value={filterDraft.search ?? ""}
            onChange={(e) => updateFilter({ search: e.target.value })}
          />
        </Field>
        <Field label="物品">
          <select
            value={filterDraft.itemId ?? ""}
            onChange={(e) => updateFilter({ itemId: e.target.value })}
          >
            <option value="">全部</option>
            {items.map((item) => (
              <option key={item.id} value={item.id}>
                {item.code} · {item.name}
              </option>
            ))}
          </select>
        </Field>
        <Field label="方向">
          <select
            value={filterDraft.direction ?? ""}
            onChange={(e) =>
              updateFilter({
                direction: e.target.value as StockMovementQuery["direction"],
              })
            }
          >
            <option value="">全部</option>
            <option value="in">入</option>
            <option value="out">出</option>
          </select>
        </Field>
        <Field label="流水类型">
          <select
            value={filterDraft.movementType ?? ""}
            onChange={(e) => updateFilter({ movementType: e.target.value })}
          >
            <option value="">全部</option>
            <option value="opening">期初</option>
            <option value="inbound">入库</option>
            <option value="outbound">出库</option>
            <option value="stocktake_gain">盘盈</option>
            <option value="stocktake_loss">盘亏</option>
            <option value="adjustment">调整</option>
            <option value="reversal">冲正</option>
          </select>
        </Field>
        <div className="document-filter-actions">
          <button className="ghost-button" onClick={resetFilters}>
            清空
          </button>
          <button className="primary-button" onClick={applyFilters}>
            筛选
          </button>
        </div>
      </div>
      <table>
        <thead>
          <tr>
            <th>日期</th>
            <th>单号</th>
            <th>类型</th>
            <th>物品</th>
            <th>方向</th>
            <th>数量</th>
            <th>单价</th>
            <th>金额</th>
            <th>部门</th>
            <th>供应商</th>
            <th>操作人</th>
            <th>备注</th>
          </tr>
        </thead>
        <PaginatedTable
          colSpan={12}
          getRowKey={(row) => row.id}
          rows={movements}
        >
          {(row) => (
            <>
              <td>{row.movementDate}</td>
              <td>{row.documentNo ?? "-"}</td>
              <td>{movementTypeLabel(row.movementType)}</td>
              <td>
                {row.itemCode} · {row.itemName}
              </td>
              <td>{row.direction === "in" ? "入库" : "出库"}</td>
              <td>{row.quantity}</td>
              <td>{formatMoney(row.unitPrice)}</td>
              <td>{formatMoney(row.amount)}</td>
              <td>{row.departmentName ?? "-"}</td>
              <td>{row.supplierName ?? "-"}</td>
              <td>{row.operator ?? "-"}</td>
              <td>{row.remark ?? "-"}</td>
            </>
          )}
        </PaginatedTable>
      </table>
    </section>
  );
}

function StocktakePage({
  canViewReports,
  canWrite,
  detail,
  exportPath,
  onConfirm,
  onExport,
  onSelect,
  onVoid,
  stocktakes,
}: {
  canViewReports: boolean;
  canWrite: boolean;
  detail: StocktakeDetail | null;
  exportPath: string | null;
  onConfirm: (handler: string, remark: string) => Promise<void>;
  onExport: () => Promise<void>;
  onSelect: (stocktakeId: string) => Promise<void>;
  onVoid: (reason: string, handler: string) => Promise<void>;
  stocktakes: StocktakeDocument[];
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

  return (
    <section className="stocktake-layout">
      <div className="form-panel">
        <div className="section-heading">
          <div>
            <h2>盘点记录</h2>
            <span>创建与实盘录入在独立窗口中完成</span>
          </div>
        </div>
        <button
          className="primary-button full-width-button"
          disabled={!canWrite}
          onClick={() =>
            openEditorWindow("stocktakeCreate", { width: 780, height: 620 })
          }
        >
          创建盘点单
        </button>
        <div className="stocktake-list">
          {stocktakes.map((stocktake) => (
            <button
              className={
                detail?.document.id === stocktake.id
                  ? "stocktake-record active"
                  : "stocktake-record"
              }
              key={stocktake.id}
              onClick={() => onSelect(stocktake.id)}
            >
              <strong>{stocktake.documentNo}</strong>
              <span>
                {stocktake.businessDate} ·{" "}
                {stocktakeStatusLabel(stocktake.status)}
              </span>
              <em>
                {stocktake.countedCount}/{stocktake.lineCount} 行
              </em>
            </button>
          ))}
          {stocktakes.length === 0 ? (
            <p className="muted-text">暂无盘点单</p>
          ) : null}
        </div>
      </div>

      <div className="table-panel stocktake-detail">
        <div className="table-toolbar">
          <div>
            <h2>{detail ? detail.document.documentNo : "盘点明细"}</h2>
            {detail ? (
              <span className="table-note">
                {detail.document.businessDate} ·{" "}
                {stocktakeScopeLabel(detail.document.scopeType)} ·{" "}
                {stocktakeStatusLabel(detail.document.status)}
              </span>
            ) : null}
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
            <button
              className="ghost-button"
              disabled={!detail || !canViewReports}
              onClick={onExport}
            >
              导出盘点表
            </button>
            <button
              className="ghost-button"
              disabled={!canEdit || !canWrite}
              onClick={() =>
                openEditorWindow("stocktakeCounts", {
                  mode: "edit",
                  id: detail?.document.id,
                  width: 1120,
                  height: 760,
                })
              }
            >
              录入实盘
            </button>
            <button
              className="ghost-button"
              disabled={!canVoid || !canWrite || !remark.trim()}
              onClick={() => onVoid(remark, handler)}
            >
              作废盘点
            </button>
            <button
              className="primary-button"
              disabled={!canEdit || !canWrite}
              onClick={() => onConfirm(handler, remark)}
            >
              确认盘点
            </button>
          </div>
        </div>
        {exportPath ? (
          <div className="export-path">已导出：{exportPath}</div>
        ) : null}
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
        ) : null}
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
            {(detail?.lines ?? []).map((line) => (
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
            {!detail || detail.lines.length === 0 ? (
              <EmptyRow colSpan={9} />
            ) : null}
          </tbody>
        </table>
      </div>
    </section>
  );
}

function AdjustmentPage({
  canWrite,
  documents,
  onVoid,
}: {
  canWrite: boolean;
  documents: StockDocument[];
  onVoid: (
    documentId: string,
    reason: string,
    handler: string,
  ) => Promise<void>;
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
        isOutbound={false}
        onVoid={onVoid}
        voidHandler={voidHandler}
        voidReason={voidReason}
      />
    </section>
  );
}

function ReportsPage({
  bundle,
  categories,
  canViewReports,
  departments,
  exportPath,
  items,
  onExport,
  onQueryChange,
  query,
  suppliers,
}: {
  bundle: ReportBundle | null;
  categories: Category[];
  canViewReports: boolean;
  departments: Department[];
  exportPath: string | null;
  items: Item[];
  onExport: (query: ReportQuery) => Promise<void>;
  onQueryChange: (query: ReportQuery) => Promise<void>;
  query: ReportQuery;
  suppliers: Supplier[];
}) {
  const [filterDraft, setFilterDraft] = useState<ReportQuery>(query);
  useEffect(() => setFilterDraft(query), [query]);

  const inventory = bundle?.monthlyInventory ?? [];
  const summary = bundle?.departmentSummary ?? [];
  const details = bundle?.departmentDetails ?? [];
  const categoryConsumption = bundle?.categoryConsumption ?? [];
  const itemRanking = bundle?.itemConsumptionRanking ?? [];
  const inboundDetails = bundle?.inboundDetails ?? [];
  const outboundDetails = bundle?.outboundDetails ?? [];
  const stockBalances = bundle?.stockBalances ?? [];
  const stockWarnings = bundle?.stockWarnings ?? [];
  const stocktakeDifferences = bundle?.stocktakeDifferences ?? [];
  const totalInbound = inventory.reduce(
    (sum, row) => sum + row.inboundAmount,
    0,
  );
  const totalOutbound = inventory.reduce(
    (sum, row) => sum + row.outboundAmount,
    0,
  );
  const reportRangeLabel =
    filterDraft.startDate || filterDraft.endDate
      ? `${filterDraft.startDate || "不限"} 至 ${filterDraft.endDate || "不限"}`
      : `${filterDraft.month || currentMonthString()} 月`;

  function printReport() {
    window.print();
  }

  function updateFilter(next: Partial<ReportQuery>) {
    setFilterDraft({ ...filterDraft, ...next });
  }

  function applyFilters() {
    onQueryChange(filterDraft);
  }

  function resetFilters() {
    const nextQuery = { month: filterDraft.month || currentMonthString() };
    setFilterDraft(nextQuery);
    onQueryChange(nextQuery);
  }

  return (
    <section className="report-layout">
      <div className="module-panel report-command-center">
        <div className="report-command-main">
          <div>
            <span className="report-kicker">报表中心</span>
            <h2>{reportRangeLabel}经营报表</h2>
            <p>
              入库 {formatMoney(totalInbound)} 元 · 领用{" "}
              {formatMoney(totalOutbound)} 元 · 库存预警 {stockWarnings.length} 项
            </p>
          </div>
          <div className="report-actions">
            <button
              className="primary-button"
              disabled={!canViewReports}
              onClick={() => onExport(query)}
            >
              导出 Excel
            </button>
            <button
              className="ghost-button"
              disabled={!canViewReports || !bundle}
              onClick={printReport}
            >
              打印
            </button>
          </div>
        </div>
        <div className="report-filters">
          <Field label="月份">
            <input
              disabled={!canViewReports}
              type="month"
              value={filterDraft.month}
              onChange={(e) => updateFilter({ month: e.target.value })}
            />
          </Field>
          <Field label="开始日期">
            <input
              disabled={!canViewReports}
              type="date"
              value={filterDraft.startDate ?? ""}
              onChange={(e) => updateFilter({ startDate: e.target.value })}
            />
          </Field>
          <Field label="结束日期">
            <input
              disabled={!canViewReports}
              type="date"
              value={filterDraft.endDate ?? ""}
              onChange={(e) => updateFilter({ endDate: e.target.value })}
            />
          </Field>
          <Field label="部门">
            <select
              disabled={!canViewReports}
              value={filterDraft.departmentId ?? ""}
              onChange={(e) => updateFilter({ departmentId: e.target.value })}
            >
              <option value="">全部</option>
              {departments.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.name}
                </option>
              ))}
            </select>
          </Field>
          <Field label="分类">
            <select
              disabled={!canViewReports}
              value={filterDraft.categoryId ?? ""}
              onChange={(e) => updateFilter({ categoryId: e.target.value })}
            >
              <option value="">全部</option>
              {categories.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.name}
                </option>
              ))}
            </select>
          </Field>
          <Field label="物品">
            <select
              disabled={!canViewReports}
              value={filterDraft.itemId ?? ""}
              onChange={(e) => updateFilter({ itemId: e.target.value })}
            >
              <option value="">全部</option>
              {items.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.code} · {item.name}
                </option>
              ))}
            </select>
          </Field>
          <Field label="供应商">
            <select
              disabled={!canViewReports}
              value={filterDraft.supplierId ?? ""}
              onChange={(e) => updateFilter({ supplierId: e.target.value })}
            >
              <option value="">全部</option>
              {suppliers.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.name}
                </option>
              ))}
            </select>
          </Field>
          <div className="report-filter-actions">
            <button
              className="ghost-button"
              disabled={!canViewReports}
              onClick={resetFilters}
            >
              清空
            </button>
            <button
              className="primary-button"
              disabled={!canViewReports || !filterDraft.month}
              onClick={applyFilters}
            >
              查询
            </button>
          </div>
        </div>
        {exportPath ? (
          <div className="export-path">已导出：{exportPath}</div>
        ) : null}
      </div>

      <section className="report-metrics-grid">
        <div className="metric-card">
          <span>本月入库金额</span>
          <strong>{formatMoney(totalInbound)}</strong>
          <em>元</em>
        </div>
        <div className="metric-card">
          <span>本月领用金额</span>
          <strong>{formatMoney(totalOutbound)}</strong>
          <em>元</em>
        </div>
        <div className="metric-card">
          <span>消耗分类</span>
          <strong>{categoryConsumption.length}</strong>
          <em>类</em>
        </div>
        <div className="metric-card">
          <span>库存预警</span>
          <strong>{stockWarnings.length}</strong>
          <em>项</em>
        </div>
        <div className="metric-card">
          <span>盘点差异</span>
          <strong>{stocktakeDifferences.length}</strong>
          <em>行</em>
        </div>
      </section>

      <ReportGroupHeader
        title="经营分析"
        description="按部门、分类、物品和库存预警查看核心变化。"
      />
      <div className="workspace-grid">
        <BarChartPanel
          rows={summary
            .filter((row) => row.amount > 0)
            .map((row) => ({ label: row.departmentName, value: row.amount }))}
          title="部门领用金额"
        />
        <BarChartPanel
          rows={categoryConsumption.map((row) => ({
            label: row.categoryName,
            value: row.amount,
          }))}
          title="分类消耗金额"
        />
      </div>

      <div className="workspace-grid">
        <BarChartPanel
          rows={itemRanking
            .slice(0, 8)
            .map((row) => ({ label: row.itemName, value: row.amount }))}
          title="物品消耗排行"
        />
        <BarChartPanel
          rows={stockWarnings
            .slice(0, 8)
            .map((row) => ({
              label: row.itemName,
              value: row.shortageQuantity,
            }))}
          title="库存预警缺口"
          valueFormatter={(value) => value.toFixed(2)}
        />
      </div>

      <ReportGroupHeader
        title="库存总览"
        description="月度进销存和当前库存余额，用于核对物品流转与结存。"
      />
      <div className="table-panel report-section">
        <div className="table-toolbar">
          <h2>月度进销存</h2>
        </div>
        <table>
          <thead>
            <tr>
              <th>编码</th>
              <th>物品</th>
              <th>规格</th>
              <th>单位</th>
              <th>入库数量</th>
              <th>入库金额</th>
              <th>出库数量</th>
              <th>出库金额</th>
              <th>结存数量</th>
              <th>结存金额</th>
            </tr>
          </thead>
          <PaginatedTable
            colSpan={10}
            getRowKey={(row) => row.itemId}
            rows={inventory}
          >
            {(row) => (
              <>
                <td>{row.itemCode}</td>
                <td>{row.itemName}</td>
                <td>{row.spec ?? "-"}</td>
                <td>{row.unitName ?? "-"}</td>
                <td>{row.inboundQuantity}</td>
                <td>{formatMoney(row.inboundAmount)}</td>
                <td>{row.outboundQuantity}</td>
                <td>{formatMoney(row.outboundAmount)}</td>
                <td>{row.endingQuantity}</td>
                <td>{formatMoney(row.endingAmount)}</td>
              </>
            )}
          </PaginatedTable>
        </table>
      </div>

      <div className="table-panel report-section">
        <div className="table-toolbar">
          <h2>库存余额表</h2>
        </div>
        <table>
          <thead>
            <tr>
              <th>编码</th>
              <th>物品</th>
              <th>规格</th>
              <th>单位</th>
              <th>当前库存</th>
              <th>库存金额</th>
              <th>移动均价</th>
              <th>最近入库价</th>
              <th>预警线</th>
              <th>状态</th>
            </tr>
          </thead>
          <PaginatedTable
            colSpan={10}
            getRowKey={(row) => row.itemId}
            rows={stockBalances}
          >
            {(row) => (
              <>
                <td>{row.itemCode}</td>
                <td>{row.itemName}</td>
                <td>{row.spec ?? "-"}</td>
                <td>{row.unitName ?? "-"}</td>
                <td>{row.quantity}</td>
                <td>{formatMoney(row.amount)}</td>
                <td>{formatMoney(row.averagePrice)}</td>
                <td>{formatMoney(row.lastInboundPrice)}</td>
                <td>{row.warningQuantity}</td>
                <td>
                  <span
                    className={`status ${row.stockStatus === "normal" ? "enabled" : "disabled"}`}
                  >
                    {row.stockStatus === "normal"
                      ? "正常"
                      : row.stockStatus === "low"
                        ? "低库存"
                        : "负库存"}
                  </span>
                </td>
              </>
            )}
          </PaginatedTable>
        </table>
      </div>

      <ReportGroupHeader
        title="领用分析"
        description="按部门、分类和物品查看领用消耗结构。"
      />
      <div className="workspace-grid">
        <div className="table-panel report-section">
          <div className="table-toolbar">
            <h2>部门领用汇总</h2>
          </div>
          <table>
            <thead>
              <tr>
                <th>部门</th>
                <th>数量</th>
                <th>金额</th>
              </tr>
            </thead>
            <tbody>
              {summary.map((row) => (
                <tr key={row.departmentId}>
                  <td>{row.departmentName}</td>
                  <td>{row.quantity}</td>
                  <td>{formatMoney(row.amount)}</td>
                </tr>
              ))}
              {summary.length === 0 ? <EmptyRow colSpan={3} /> : null}
            </tbody>
          </table>
        </div>

        <div className="table-panel report-section">
          <div className="table-toolbar">
            <h2>部门领用明细</h2>
          </div>
          <table>
            <thead>
              <tr>
                <th>日期</th>
                <th>部门</th>
                <th>物品</th>
                <th>数量</th>
                <th>金额</th>
              </tr>
            </thead>
            <PaginatedTable
              colSpan={5}
              getRowKey={(row, index) =>
                `${row.documentNo}-${row.itemCode}-${index}`
              }
              rows={details}
            >
              {(row) => (
                <>
                  <td>{row.movementDate}</td>
                  <td>{row.departmentName}</td>
                  <td>
                    {row.itemCode} · {row.itemName}
                  </td>
                  <td>{row.quantity}</td>
                  <td>{formatMoney(row.amount)}</td>
                </>
              )}
            </PaginatedTable>
          </table>
        </div>
      </div>

      <div className="workspace-grid">
        <div className="table-panel report-section">
          <div className="table-toolbar">
            <h2>分类消耗统计</h2>
          </div>
          <table>
            <thead>
              <tr>
                <th>分类</th>
                <th>数量</th>
                <th>金额</th>
              </tr>
            </thead>
            <tbody>
              {categoryConsumption.map((row) => (
                <tr key={row.categoryId ?? row.categoryName}>
                  <td>{row.categoryName}</td>
                  <td>{row.quantity}</td>
                  <td>{formatMoney(row.amount)}</td>
                </tr>
              ))}
              {categoryConsumption.length === 0 ? (
                <EmptyRow colSpan={3} />
              ) : null}
            </tbody>
          </table>
        </div>

        <div className="table-panel report-section">
          <div className="table-toolbar">
            <h2>物品消耗排行</h2>
          </div>
          <table>
            <thead>
              <tr>
                <th>编码</th>
                <th>物品</th>
                <th>数量</th>
                <th>金额</th>
              </tr>
            </thead>
            <PaginatedTable
              colSpan={4}
              getRowKey={(row) => row.itemId}
              rows={itemRanking}
            >
              {(row) => (
                <>
                  <td>{row.itemCode}</td>
                  <td>{row.itemName}</td>
                  <td>
                    {row.quantity} {row.unitName ?? ""}
                  </td>
                  <td>{formatMoney(row.amount)}</td>
                </>
              )}
            </PaginatedTable>
          </table>
        </div>
      </div>

      <ReportGroupHeader
        title="流水明细"
        description="入库与出库明细，用于追溯单据来源。"
      />
      <div className="table-panel report-section">
        <div className="table-toolbar">
          <h2>入库明细</h2>
        </div>
        <table>
          <thead>
            <tr>
              <th>日期</th>
              <th>供应商</th>
              <th>物品</th>
              <th>数量</th>
              <th>单价</th>
              <th>金额</th>
              <th>单号</th>
            </tr>
          </thead>
          <PaginatedTable
            colSpan={7}
            getRowKey={(row, index) =>
              `${row.documentNo}-${row.itemCode}-${index}`
            }
            rows={inboundDetails}
          >
            {(row) => (
              <>
                <td>{row.movementDate}</td>
                <td>{row.supplierName}</td>
                <td>
                  {row.itemCode} · {row.itemName}
                </td>
                <td>
                  {row.quantity} {row.unitName ?? ""}
                </td>
                <td>{formatMoney(row.unitPrice)}</td>
                <td>{formatMoney(row.amount)}</td>
                <td>{row.documentNo ?? "-"}</td>
              </>
            )}
          </PaginatedTable>
        </table>
      </div>

      <div className="table-panel report-section">
        <div className="table-toolbar">
          <h2>出库明细</h2>
        </div>
        <table>
          <thead>
            <tr>
              <th>日期</th>
              <th>部门</th>
              <th>物品</th>
              <th>数量</th>
              <th>单价</th>
              <th>金额</th>
              <th>单号</th>
            </tr>
          </thead>
          <PaginatedTable
            colSpan={7}
            getRowKey={(row, index) =>
              `${row.documentNo}-${row.itemCode}-${index}`
            }
            rows={outboundDetails}
          >
            {(row) => (
              <>
                <td>{row.movementDate}</td>
                <td>{row.departmentName}</td>
                <td>
                  {row.itemCode} · {row.itemName}
                </td>
                <td>
                  {row.quantity} {row.unitName ?? ""}
                </td>
                <td>{formatMoney(row.unitPrice)}</td>
                <td>{formatMoney(row.amount)}</td>
                <td>{row.documentNo ?? "-"}</td>
              </>
            )}
          </PaginatedTable>
        </table>
      </div>

      <ReportGroupHeader
        title="库存风险"
        description="库存预警和盘点差异，用于定位缺口与账实不符。"
      />
      <div className="table-panel report-section">
        <div className="table-toolbar">
          <h2>库存预警表</h2>
        </div>
        <table>
          <thead>
            <tr>
              <th>编码</th>
              <th>物品</th>
              <th>规格</th>
              <th>单位</th>
              <th>当前库存</th>
              <th>预警线</th>
              <th>缺口</th>
              <th>库存金额</th>
            </tr>
          </thead>
          <PaginatedTable
            colSpan={8}
            getRowKey={(row) => row.itemId}
            rows={stockWarnings}
          >
            {(row) => (
              <>
                <td>{row.itemCode}</td>
                <td>{row.itemName}</td>
                <td>{row.spec ?? "-"}</td>
                <td>{row.unitName ?? "-"}</td>
                <td>{row.quantity}</td>
                <td>{row.warningQuantity}</td>
                <td>{row.shortageQuantity}</td>
                <td>{formatMoney(row.amount)}</td>
              </>
            )}
          </PaginatedTable>
        </table>
      </div>

      <div className="table-panel report-section">
        <div className="table-toolbar">
          <h2>盘点差异表</h2>
        </div>
        <table>
          <thead>
            <tr>
              <th>日期</th>
              <th>单号</th>
              <th>范围</th>
              <th>物品</th>
              <th>账面数</th>
              <th>实盘数</th>
              <th>差异数</th>
              <th>移动均价</th>
              <th>差异金额</th>
              <th>备注</th>
            </tr>
          </thead>
          <PaginatedTable
            colSpan={10}
            getRowKey={(row, index) =>
              `${row.documentNo}-${row.itemCode}-${index}`
            }
            rows={stocktakeDifferences}
          >
            {(row) => (
              <>
                <td>{row.businessDate}</td>
                <td>{row.documentNo}</td>
                <td>{stocktakeScopeLabel(row.scopeType)}</td>
                <td>
                  {row.itemCode} · {row.itemName}
                </td>
                <td>
                  {row.bookQuantity} {row.unitName ?? ""}
                </td>
                <td>
                  {row.countedQuantity} {row.unitName ?? ""}
                </td>
                <td>
                  {row.differenceQuantity} {row.unitName ?? ""}
                </td>
                <td>{formatMoney(row.averagePrice)}</td>
                <td>{formatMoney(row.differenceAmount)}</td>
                <td>{row.remark ?? "-"}</td>
              </>
            )}
          </PaginatedTable>
        </table>
      </div>
    </section>
  );
}

function ReportGroupHeader({
  description,
  title,
}: {
  description: string;
  title: string;
}) {
  return (
    <div className="report-group-header">
      <h2>{title}</h2>
      <p>{description}</p>
    </div>
  );
}

function BarChartPanel({
  rows,
  title,
  valueFormatter = formatMoney,
}: {
  rows: { label: string; value: number }[];
  title: string;
  valueFormatter?: (value: number) => string;
}) {
  const visibleRows = rows.slice(0, 8);
  const maxValue = Math.max(
    ...visibleRows.map((row) => Math.abs(row.value)),
    0,
  );
  return (
    <div className="chart-panel">
      <div className="table-toolbar">
        <h2>{title}</h2>
      </div>
      <div className="bar-chart">
        {visibleRows.map((row) => {
          const width =
            maxValue > 0
              ? Math.max(3, Math.round((Math.abs(row.value) / maxValue) * 100))
              : 0;
          return (
            <div className="bar-row" key={row.label}>
              <span>{row.label}</span>
              <div className="bar-track">
                <i style={{ width: `${width}%` }} />
              </div>
              <strong>{valueFormatter(row.value)}</strong>
            </div>
          );
        })}
        {visibleRows.length === 0 ? (
          <div className="empty-chart">暂无数据</div>
        ) : null}
      </div>
    </div>
  );
}

function ImportPage({
  canPreviewImport,
  canRunImport,
  isWorking,
  onPreview,
  onRun,
  preview,
  result,
}: {
  canPreviewImport: boolean;
  canRunImport: boolean;
  isWorking: boolean;
  onPreview: (path: string) => Promise<void>;
  onRun: (path: string, mode: "full" | "itemsOnly") => Promise<void>;
  preview: ImportPreview | null;
  result: ImportResult | null;
}) {
  const [path, setPath] = useState("");
  const [mode, setMode] = useState<"full" | "itemsOnly">("full");
  async function selectImportFile() {
    const selected = await chooseSinglePath({
      title: "选择 Excel 导入文件",
      filters: [{ name: "Excel 工作簿", extensions: ["xlsx"] }],
    });
    if (selected) {
      setPath(selected);
    }
  }
  const previewMetrics = [
    { label: "工作表", value: preview?.sheetCount ?? 0, suffix: "张" },
    { label: "物品", value: preview?.itemCount ?? 0, suffix: "种" },
    { label: "待新增物品", value: preview?.newItemCount ?? 0, suffix: "种" },
    { label: "预计单据", value: preview?.documentCount ?? 0, suffix: "张" },
    {
      label: "期初金额",
      value: formatMoney(preview?.openingAmount ?? 0),
      suffix: "元",
    },
    {
      label: "入库金额",
      value: formatMoney(preview?.inboundAmount ?? 0),
      suffix: "元",
    },
    {
      label: "领用金额",
      value: formatMoney(preview?.outboundAmount ?? 0),
      suffix: "元",
    },
    { label: "错误", value: preview?.errors.length ?? 0, suffix: "条" },
  ];
  const canImport = Boolean(preview && preview.errors.length === 0);

  return (
    <section className="import-layout">
      <div className="module-panel import-toolbar">
        <div>
          <p>
            支持旧酒店月报和通用模板，预览不会写入数据库；确认导入后生成物品档案、入库单、出库单、库存流水和余额。
          </p>
        </div>
        <div className="import-path-row">
          <input readOnly value={path} placeholder="请选择 .xlsx 文件" />
          <button
            className="ghost-button"
            disabled={isWorking || !canPreviewImport}
            onClick={selectImportFile}
          >
            选择 Excel
          </button>
          <select
            value={mode}
            onChange={(e) => setMode(e.target.value as "full" | "itemsOnly")}
          >
            <option value="full">完整导入</option>
            <option value="itemsOnly">只导入物品档案</option>
          </select>
          <button
            className="ghost-button"
            disabled={isWorking || !canPreviewImport}
            onClick={() => onPreview(path)}
          >
            预览
          </button>
          <button
            className="primary-button"
            disabled={isWorking || !canImport || !canRunImport}
            onClick={() => onRun(path, mode)}
          >
            执行导入
          </button>
        </div>
      </div>

      <section className="metrics-grid">
        {previewMetrics.map((card) => (
          <div className="metric-card" key={card.label}>
            <span>{card.label}</span>
            <strong>{card.value}</strong>
            <em>{card.suffix}</em>
          </div>
        ))}
      </section>

      {result ? (
        <div className="module-panel import-result">
          <h2>导入结果</h2>
          <div className="result-grid">
            <span>任务 ID</span>
            <strong>{result.jobId}</strong>
            <span>新增物品</span>
            <strong>{result.importedItems} 种</strong>
            <span>匹配物品</span>
            <strong>{result.matchedItems} 种</strong>
            <span>生成单据</span>
            <strong>{result.documentCount} 张</strong>
            <span>生成流水</span>
            <strong>{result.movementCount} 条</strong>
            <span>导入报告</span>
            <strong>{result.reportPath ?? "-"}</strong>
            <span>源文件备份</span>
            <strong>{result.sourceCopyPath ?? "-"}</strong>
          </div>
        </div>
      ) : null}

      <div className="workspace-grid">
        <div className="table-panel report-section">
          <div className="table-toolbar">
            <h2>月份识别</h2>
          </div>
          <table>
            <thead>
              <tr>
                <th>月份</th>
                <th>行数</th>
                <th>期初数量</th>
                <th>入库数量</th>
                <th>领用数量</th>
                <th>领用金额</th>
              </tr>
            </thead>
            <tbody>
              {(preview?.months ?? []).map((row) => (
                <tr key={row.month}>
                  <td>{row.month}</td>
                  <td>{row.rowCount}</td>
                  <td>{row.openingQuantity}</td>
                  <td>{row.inboundQuantity}</td>
                  <td>{row.outboundQuantity}</td>
                  <td>{formatMoney(row.outboundAmount)}</td>
                </tr>
              ))}
              {!preview || preview.months.length === 0 ? (
                <EmptyRow colSpan={6} />
              ) : null}
            </tbody>
          </table>
        </div>

        <div className="table-panel report-section">
          <div className="table-toolbar">
            <h2>校验信息</h2>
          </div>
          <table>
            <thead>
              <tr>
                <th>级别</th>
                <th>位置</th>
                <th>说明</th>
              </tr>
            </thead>
            <PaginatedTable
              colSpan={3}
              getRowKey={(message, index) =>
                `${message.sheet}-${message.row}-${index}`
              }
              rows={[...(preview?.errors ?? []), ...(preview?.warnings ?? [])]}
            >
              {(message) => (
                <>
                  <td>
                    <span
                      className={`status ${message.level === "error" ? "disabled" : "enabled"}`}
                    >
                      {message.level === "error" ? "错误" : "提醒"}
                    </span>
                  </td>
                  <td>
                    {message.sheet} 第 {message.row} 行
                    {message.column ? ` ${message.column}列` : ""}
                  </td>
                  <td>{message.message}</td>
                </>
              )}
            </PaginatedTable>
          </table>
        </div>
      </div>

      <div className="table-panel report-section">
        <div className="table-toolbar">
          <h2>物品预览</h2>
          <span className="table-note">最多按当前解析结果展示全部物品</span>
        </div>
        <table>
          <thead>
            <tr>
              <th>物品</th>
              <th>分类</th>
              <th>规格</th>
              <th>单位</th>
              <th>默认价</th>
              <th>期初</th>
              <th>入库</th>
              <th>领用</th>
              <th>匹配</th>
            </tr>
          </thead>
          <PaginatedTable
            colSpan={9}
            getRowKey={(item) => item.name}
            rows={preview?.items ?? []}
          >
            {(item) => (
              <>
                <td>{item.name}</td>
                <td>{item.categoryName ?? "-"}</td>
                <td>{item.spec ?? "-"}</td>
                <td>{item.unitName ?? "-"}</td>
                <td>{formatMoney(item.defaultPrice)}</td>
                <td>{item.openingQuantity}</td>
                <td>{item.inboundQuantity}</td>
                <td>{item.outboundQuantity}</td>
                <td>
                  <span
                    className={
                      item.existing ? "status enabled" : "status disabled"
                    }
                  >
                    {item.existing ? "已有" : "新增"}
                  </span>
                </td>
              </>
            )}
          </PaginatedTable>
        </table>
      </div>
    </section>
  );
}

function SettingsPage({
  appearanceSettings,
  canManage,
  clientConnectionCheckedAt,
  clientConnections,
  currentUser,
  hostStatus,
  hostTestResult,
  i18n,
  isWorking,
  lastBackup,
  onBackup,
  onAppearanceChange,
  onLogout,
  onOpenConnectionWizard,
  onRemoveClientConnection,
  onStartHostService,
  status,
  systemSettings,
}: {
  appearanceSettings: AppearanceSettings;
  canManage: boolean;
  clientConnectionCheckedAt: string | null;
  clientConnections: ClientConnectionInfo[];
  currentUser: CurrentUser;
  hostStatus: HostServiceStatus | null;
  hostTestResult: HostConnectionTestResult | null;
  i18n: I18n;
  isWorking: boolean;
  lastBackup: BackupSummary | null;
  onBackup: () => Promise<void>;
  onAppearanceChange: React.Dispatch<React.SetStateAction<AppearanceSettings>>;
  onLogout: () => Promise<void>;
  onOpenConnectionWizard: () => void;
  onRemoveClientConnection: (client: ClientConnectionInfo) => Promise<void>;
  onStartHostService: () => Promise<void>;
  status: AppStatus | null;
  systemSettings: SystemSettings | null;
}) {
  const settingsIsClientMode = status?.runtime.mode === "client";
  const canOperateSettings = canManage && !settingsIsClientMode;
  const settingsEffectiveTheme = resolveTheme(appearanceSettings.themeMode);
  const settingsGlassPreviewThemeClass =
    settingsEffectiveTheme === "light"
      ? "preview-glass-light"
      : "preview-glass-dark";
  const settingsBackupMetrics = [
    {
      label: i18n.t("settings.databaseStatus"),
      value: status?.health.databaseOk
        ? i18n.t("settings.statusHealthy")
        : i18n.t("settings.statusAbnormal"),
      suffix: "",
    },
    {
      label: i18n.t("settings.latestBackup"),
      value: status?.health.latestBackupAt ?? "-",
      suffix: "",
    },
    {
      label: i18n.t("settings.secondBackup"),
      value: status?.health.secondBackupOk
        ? i18n.t("settings.available")
        : i18n.t("settings.notReady"),
      suffix: "",
    },
    {
      label: i18n.t("settings.intervalBackup"),
      value: status?.health.intervalBackupEnabled
        ? i18n.t("settings.hours", {
            hours: status.health.intervalBackupHours,
          })
        : i18n.t("settings.closed"),
      suffix: "",
    },
  ];

  return (
    <section className="settings-page">
      <div className="settings-group">
        <h3 className="settings-group-title">{i18n.t("settings.appearance")}</h3>
        <article className="surface settings-block appearance-settings-block">
          <div className="setting-row">
            <div className="settings-meta">
              <span className="settings-label">{i18n.t("settings.themeMode")}</span>
              <p className="settings-hint">
                {i18n.t("settings.effectiveTheme", {
                  theme:
                    settingsEffectiveTheme === "dark"
                      ? i18n.t("settings.dark")
                      : i18n.t("settings.light"),
                })}
              </p>
            </div>
            <div className="setting-control">
              <div className="preview-grid preview-grid-3">
                {(["auto", "light", "dark"] as ThemeMode[]).map((mode) => (
                  <button
                    key={mode}
                    className={`preview-card ${appearanceSettings.themeMode === mode ? "active" : ""}`}
                    type="button"
                    onClick={() =>
                      onAppearanceChange((current) => ({
                        ...current,
                        themeMode: mode,
                      }))
                    }
                  >
                    <span
                      className={`preview-art preview-theme preview-theme-${mode}`}
                    />
                    <span className="preview-label">
                      {mode === "auto"
                        ? i18n.t("settings.followSystem")
                        : mode === "light"
                          ? i18n.t("settings.light")
                          : i18n.t("settings.dark")}
                    </span>
                  </button>
                ))}
              </div>
            </div>
          </div>

          <div className="setting-row">
            <div className="settings-meta">
              <span className="settings-label">Liquid Glass</span>
              <p className="settings-hint">
                {i18n.t("settings.liquidGlassHint")}
              </p>
            </div>
            <div className="setting-control">
              <div className="preview-grid preview-grid-2">
                {(["transparent", "tinted"] as LiquidGlassStyle[]).map(
                  (style) => (
                    <button
                      key={style}
                      className={`preview-card ${appearanceSettings.liquidGlassStyle === style ? "active" : ""}`}
                      type="button"
                      onClick={() =>
                        onAppearanceChange((current) => ({
                          ...current,
                          liquidGlassStyle: style,
                        }))
                      }
                    >
                      <span
                        className={`preview-art preview-glass preview-glass-${style} ${settingsGlassPreviewThemeClass}`}
                      />
                      <span className="preview-label">
                        {style === "transparent"
                          ? i18n.t("settings.transparent")
                          : i18n.t("settings.tinted")}
                      </span>
                    </button>
                  ),
                )}
              </div>
            </div>
          </div>

          <div className="setting-row">
            <div className="settings-meta">
              <span className="settings-label">
                {i18n.t("settings.interfaceLanguage")}
              </span>
            </div>
            <div className="setting-control setting-control-inline">
              <select
                value={appearanceSettings.locale}
                onChange={(event) =>
                  onAppearanceChange((current) => ({
                    ...current,
                    locale: event.target.value as LocaleCode,
                  }))
                }
              >
                <option value="zh-CN">简体中文</option>
                <option value="en-US">English</option>
              </select>
            </div>
          </div>
        </article>
      </div>

      <div className="settings-group settings-group-accent">
        <h3 className="settings-group-title">{i18n.t("settings.theme")}</h3>
        <article className="surface settings-block appearance-settings-block">
          <div className="setting-row setting-row-color">
            <div className="settings-meta">
              <span className="settings-label">{i18n.t("settings.color")}</span>
            </div>
            <div className="setting-control">
              <div className="color-row">
                {accentColors.map((color) => (
                  <div key={color} className="color-option">
                    <button
                      className={`color-dot ${appearanceSettings.accentColor.toLowerCase() === color ? "active" : ""}`}
                      style={{ background: color }}
                      type="button"
                      title={accentColorLabel(color, i18n)}
                      onClick={() =>
                        onAppearanceChange((current) => ({
                          ...current,
                          accentColor: color,
                        }))
                      }
                    />
                    {appearanceSettings.accentColor.toLowerCase() === color ? (
                      <span className="color-option-label">
                        {accentColorLabel(color, i18n)}
                      </span>
                    ) : null}
                  </div>
                ))}
              </div>
            </div>
          </div>
        </article>
      </div>

      <div className="settings-group">
        <h3 className="settings-group-title">
          {i18n.t("settings.runtimeStatus")}
        </h3>
        <article className="surface settings-block">
          {settingsBackupMetrics.map((card) => (
            <div className="setting-row" key={card.label}>
              <div className="settings-meta">
                <span className="settings-label">{card.label}</span>
              </div>
              <div className="setting-control">
                <strong className="setting-value">
                  {card.value}
                  {card.suffix}
                </strong>
              </div>
            </div>
          ))}
        </article>
      </div>

      <div className="settings-group">
        <h3 className="settings-group-title">
          {i18n.t("settings.accountSecurity")}
        </h3>
        <article className="surface settings-block">
          <div className="setting-row">
            <div className="settings-meta">
              <span className="settings-label">
                {i18n.t("settings.currentAccount")}
              </span>
              <p className="settings-hint">
                {currentUser.roles.map((role) => role.name).join("、") ||
                  i18n.t("settings.noRole")}
              </p>
            </div>
            <div className="setting-control">
              <strong className="setting-value">
                {currentUser.displayName}
              </strong>
            </div>
          </div>
          <div className="setting-row">
            <div className="settings-meta">
              <span className="settings-label">
                {i18n.t("settings.changePassword")}
              </span>
              <p className="settings-hint">
                {i18n.t("settings.changePasswordHint")}
              </p>
            </div>
            <div className="setting-control">
              <button
                className="primary-button"
                type="button"
                onClick={() =>
                  void openEditorWindow("changePassword", {
                    width: 520,
                    height: 360,
                  })
                }
              >
                {i18n.t("settings.openChangePassword")}
              </button>
            </div>
          </div>
          <div className="setting-row">
            <div className="settings-meta">
              <span className="settings-label">
                {i18n.t("settings.loginSession")}
              </span>
            </div>
            <div className="setting-control">
              <button className="ghost-button" onClick={onLogout}>
                {i18n.t("settings.logout")}
              </button>
            </div>
          </div>
        </article>
      </div>

      <div className="settings-group">
        <h3 className="settings-group-title">
          {i18n.t("settings.businessAndDirectories")}
        </h3>
        <article className="surface settings-block">
          <div className="setting-row">
            <div className="settings-meta">
              <span className="settings-label">
                {systemSettings?.hotelName ||
                  i18n.t("settings.businessSettings")}
              </span>
              <p className="settings-hint">
                {settingsIsClientMode
                  ? i18n.t("settings.clientBusinessSettingsHint")
                  : i18n.t("settings.businessSettingsHint")}
              </p>
            </div>
            <div className="setting-control">
              <button
                className="primary-button"
                disabled={!canOperateSettings}
                type="button"
                onClick={() =>
                  void openEditorWindow("businessSettings", {
                    width: 720,
                    height: 620,
                  })
                }
              >
                {i18n.t("settings.openBusinessSettings")}
              </button>
            </div>
          </div>
          <div className="setting-row">
            <div className="settings-meta">
              <span className="settings-label">
                {i18n.t("settings.currentPeriod")}
              </span>
            </div>
            <div className="setting-control">
              <strong className="setting-value">
                {systemSettings?.currentPeriod ?? "-"}
              </strong>
            </div>
          </div>
        </article>
      </div>

      <div className="settings-group">
        <h3 className="settings-group-title">
          {i18n.t("settings.multiComputer")}
        </h3>
        <article className="surface settings-feature-panel">
          <div className="settings-feature-header">
            <div>
              <span className="settings-feature-kicker">
                {i18n.t("settings.currentStatus")}
              </span>
              <h4>{connectionStatusLabel(status, hostTestResult, i18n)}</h4>
              <p>
                {connectionStatusHint(status, hostStatus, hostTestResult, i18n)}
              </p>
            </div>
            <div className="settings-feature-actions">
              <button
                className="primary-button"
                disabled={!canManage}
                type="button"
                onClick={onOpenConnectionWizard}
              >
                {i18n.t("settings.openConnectionWizard")}
              </button>
              {status?.runtime.mode === "host" ? (
                <button
                  className="ghost-button"
                  disabled={!canManage || isWorking}
                  type="button"
                  onClick={onStartHostService}
                >
                  {i18n.t("settings.restartSharing")}
                </button>
              ) : null}
              {settingsIsClientMode ? (
                <button
                  className="ghost-button"
                  disabled={!canManage}
                  type="button"
                  onClick={onOpenConnectionWizard}
                >
                  {i18n.t("settings.reconnect")}
                </button>
              ) : null}
            </div>
          </div>

          <dl className="settings-metric-list">
            <div>
              <dt>{i18n.t("settings.hostComputer")}</dt>
              <dd>
                {settingsIsClientMode
                  ? `${status?.runtime.hostAddress ?? "-"}:${status?.runtime.hostPort ?? "-"}`
                  : hostStatus?.running
                    ? i18n.t("settings.thisComputer")
                    : "-"}
              </dd>
            </div>
            <div>
              <dt>{i18n.t("settings.connectionStatus")}</dt>
              <dd>
                {settingsIsClientMode
                  ? hostTestResult?.message ?? i18n.t("settings.notChecked")
                  : hostStatus?.message ??
                    i18n.t("settings.sharingNotStarted")}
              </dd>
            </div>
            <div>
              <dt>{i18n.t("settings.otherComputers")}</dt>
              <dd>
                {status?.runtime.mode === "host"
                  ? i18n.t("settings.computerCount", {
                      count: clientConnections.length,
                    })
                  : "-"}
              </dd>
            </div>
            <div>
              <dt>{i18n.t("settings.lastChecked")}</dt>
              <dd>{clientConnectionCheckedAt ?? "-"}</dd>
            </div>
          </dl>

          {hostStatus?.pairCode && status?.runtime.mode === "host" ? (
            <div className="settings-inline-note">
              <strong>
                {i18n.t("settings.currentPairCode", {
                  code: hostStatus.pairCode,
                })}
              </strong>
              <span>{i18n.t("settings.pairCodeHint")}</span>
            </div>
          ) : null}

          {settingsIsClientMode && hostTestResult?.ok === false ? (
            <div className="settings-inline-note warning">
              <strong>
                {status?.runtime.clientToken
                  ? i18n.t("settings.hostConnectionAbnormal")
                  : i18n.t("settings.hostNotConnected")}
              </strong>
              <span>{hostTestResult.message}</span>
            </div>
          ) : null}

          {status?.runtime.mode === "standalone" ? (
            <p className="settings-footnote">
              {i18n.t("settings.standaloneFootnote")}
            </p>
          ) : null}

          {status?.runtime.mode === "host" ? (
            <div className="settings-compact-table">
              <div className="settings-compact-table-title">
                {i18n.t("settings.connectedClients")}
              </div>
              <table>
                <thead>
                  <tr>
                    <th>{i18n.t("settings.clientName")}</th>
                    <th>{i18n.t("settings.clientDevice")}</th>
                    <th>{i18n.t("settings.clientIp")}</th>
                    <th>{i18n.t("settings.clientVersion")}</th>
                    <th>{i18n.t("settings.clientStatus")}</th>
                    <th>{i18n.t("settings.clientLastSeen")}</th>
                    <th>{i18n.t("settings.clientActions")}</th>
                  </tr>
                </thead>
                <tbody>
                  {clientConnections.map((client) => (
                    <tr key={client.id}>
                      <td>{client.clientName}</td>
                      <td>{client.clientDeviceId}</td>
                      <td>{client.clientIp}</td>
                      <td>{client.appVersion}</td>
                      <td>
                        <span className="status enabled">{client.status}</span>
                      </td>
                      <td>{client.lastSeenAt}</td>
                      <td className="row-actions">
                        <button
                          className="ghost-button"
                          disabled={!canManage || isWorking}
                          type="button"
                          onClick={() => void onRemoveClientConnection(client)}
                        >
                          {i18n.t("settings.removeClient")}
                        </button>
                      </td>
                    </tr>
                  ))}
                  {clientConnections.length === 0 ? <EmptyRow colSpan={7} /> : null}
                </tbody>
              </table>
            </div>
          ) : null}
        </article>
      </div>

      <div className="settings-group">
        <h3 className="settings-group-title">
          {i18n.t("settings.backupAndRestore")}
        </h3>
        <article className="surface settings-feature-panel">
          <div className="settings-feature-header">
            <div>
              <span className="settings-feature-kicker">
                {i18n.t("settings.backupPolicy")}
              </span>
              <h4>{i18n.t("settings.backupAndRestore")}</h4>
              <p>{i18n.t("settings.backupHint")}</p>
            </div>
            <div className="settings-feature-actions">
              <button
                className="primary-button"
                disabled={!canOperateSettings || isWorking}
                onClick={onBackup}
              >
                {i18n.t("settings.createManualBackup")}
              </button>
              <button
                className="ghost-button"
                disabled={!canOperateSettings || isWorking}
                type="button"
                onClick={() =>
                  void openEditorWindow("secondBackupDir", {
                    width: 620,
                    height: 340,
                  })
                }
              >
                {i18n.t("settings.secondBackupDir")}
              </button>
              <button
                className="ghost-button"
                disabled={!canOperateSettings || isWorking}
                type="button"
                onClick={() =>
                  void openEditorWindow("restoreBackup", {
                    width: 720,
                    height: 560,
                  })
                }
              >
                {i18n.t("settings.restoreBackup")}
              </button>
            </div>
          </div>

          <dl className="settings-metric-list">
            <div>
              <dt>{i18n.t("settings.localDir")}</dt>
              <dd>{status?.runtime.backupDir ?? "-"}</dd>
            </div>
            <div>
              <dt>{i18n.t("settings.latestBackup")}</dt>
              <dd>{status?.health.latestBackupAt ?? "-"}</dd>
            </div>
            <div>
              <dt>{i18n.t("settings.latestIntervalBackup")}</dt>
              <dd>{status?.health.latestIntervalBackupAt ?? "-"}</dd>
            </div>
            <div>
              <dt>{i18n.t("settings.autoBackup")}</dt>
              <dd>
                {status?.health.autoBackupEnabled
                  ? i18n.t("settings.enabled")
                  : i18n.t("settings.disabled")}
              </dd>
            </div>
          </dl>

          {settingsIsClientMode ? (
            <p className="settings-footnote">
              {i18n.t("settings.clientBackupFootnote")}
            </p>
          ) : null}

          {lastBackup ? (
            <div className="settings-inline-note">
              <strong>{i18n.t("settings.backupCreated")}</strong>
              <span>{lastBackup.backupFile}</span>
              <span>
                {i18n.t("settings.backupSource", {
                  host: lastBackup.sourceHostName,
                  os: lastBackup.sourceOs,
                  schema: lastBackup.schemaVersion,
                  size: formatFileSize(lastBackup.databaseSize),
                })}
              </span>
              <span>
                {i18n.t("settings.backupSha", {
                  sha: lastBackup.databaseSha256,
                })}
              </span>
              {lastBackup.secondBackupFile ? (
                <span>
                  {i18n.t("settings.secondBackupFile", {
                    file: lastBackup.secondBackupFile,
                  })}
                </span>
              ) : null}
            </div>
          ) : null}
        </article>
      </div>
    </section>
  );

}

function BackupRecordsPage({
  backups,
  i18n = defaultI18n,
}: {
  backups: BackupRecord[];
  i18n?: I18n;
}) {
  return (
    <section className="table-panel">
      <div className="table-toolbar">
        <div>
          <h2>备份记录</h2>
          <span className="table-note">
            手动备份、导入前备份、恢复前保护备份和自动备份记录
          </span>
        </div>
      </div>
      <table>
        <thead>
          <tr>
            <th>时间</th>
            <th>类型</th>
            <th>状态</th>
            <th>主机</th>
            <th>系统</th>
            <th>版本</th>
            <th>Schema</th>
            <th>大小</th>
            <th>SHA256</th>
            <th>文件</th>
            <th>错误</th>
          </tr>
        </thead>
        <PaginatedTable
          colSpan={11}
          getRowKey={(backup) => backup.id}
          rows={backups}
        >
          {(backup) => (
            <>
              <td>{backup.createdAt}</td>
              <td>{backupTypeLabel(backup.backupType, i18n)}</td>
              <td>
                <span
                  className={
                    backup.status === "success"
                      ? "status enabled"
                      : "status disabled"
                  }
                >
                  {backup.status}
                </span>
              </td>
              <td>{backup.hostName ?? "-"}</td>
              <td>{backup.os ?? "-"}</td>
              <td>{backup.appVersion}</td>
              <td>v{backup.schemaVersion}</td>
              <td>{formatFileSize(backup.databaseSize)}</td>
              <td className="path-cell">{backup.sha256 ?? "-"}</td>
              <td className="path-cell">{backup.backupFile}</td>
              <td>{backup.errorMessage ?? "-"}</td>
            </>
          )}
        </PaginatedTable>
      </table>
    </section>
  );
}

function LogsPage({
  auditLogs,
  i18n = defaultI18n,
}: {
  auditLogs: AuditLogRow[];
  i18n?: I18n;
}) {
  return (
    <section className="table-panel">
      <div className="table-toolbar">
        <div>
          <h2>操作日志</h2>
          <span className="table-note">
            系统操作、导入、备份、用户和业务变更记录
          </span>
        </div>
      </div>
      <table>
        <thead>
          <tr>
            <th>时间</th>
            <th>动作</th>
            <th>对象</th>
            <th>摘要</th>
            <th>操作人</th>
          </tr>
        </thead>
        <PaginatedTable
          colSpan={5}
          getRowKey={(log) => log.id}
          rows={auditLogs}
        >
          {(log) => (
            <>
              <td>{log.createdAt}</td>
              <td>{auditActionLabel(log.action, i18n)}</td>
              <td>
                <span className="audit-entity">
                  {auditEntityLabel(log.entityType, i18n)}
                </span>
                <span className="audit-entity-id">{log.entityId}</span>
              </td>
              <td className="audit-summary">{log.summary}</td>
              <td>{log.operator}</td>
            </>
          )}
        </PaginatedTable>
      </table>
    </section>
  );
}

function MasterTablePanel({
  actions,
  children,
  description,
  hideHeading = false,
  title,
}: {
  actions?: React.ReactNode;
  children: React.ReactNode;
  description: string;
  hideHeading?: boolean;
  title: string;
}) {
  return (
    <section className="table-panel">
      <div
        className={hideHeading ? "table-toolbar actions-only" : "table-toolbar"}
      >
        {hideHeading ? null : (
          <div>
            <h2>{title}</h2>
            <span className="table-note">{description}</span>
          </div>
        )}
        {actions ? <div className="toolbar-actions">{actions}</div> : null}
      </div>
      {children}
    </section>
  );
}

function PaginatedTable<T>({
  children,
  colSpan,
  empty,
  getRowKey,
  pageSize = DEFAULT_TABLE_PAGE_SIZE,
  rows,
}: {
  children: (row: T, index: number) => React.ReactNode;
  colSpan: number;
  empty?: React.ReactNode;
  getRowKey: (row: T, index: number) => React.Key;
  pageSize?: number;
  rows: T[];
}) {
  const [page, setPage] = useState(1);
  const pageCount = Math.max(1, Math.ceil(rows.length / pageSize));
  const safePage = Math.min(page, pageCount);

  useEffect(() => {
    setPage(1);
  }, [rows, pageSize]);

  useEffect(() => {
    if (page !== safePage) {
      setPage(safePage);
    }
  }, [page, safePage]);

  const start = (safePage - 1) * pageSize;
  const visibleRows = rows.slice(start, start + pageSize);

  return (
    <>
      <tbody>
        {visibleRows.map((row, index) => (
          <tr key={getRowKey(row, start + index)}>
            {children(row, start + index)}
          </tr>
        ))}
        {rows.length === 0 ? (empty ?? <EmptyRow colSpan={colSpan} />) : null}
      </tbody>
      {rows.length > pageSize ? (
        <tfoot>
          <tr>
            <td colSpan={colSpan}>
              <div className="pagination-bar">
                <span>
                  {start + 1}-{Math.min(start + pageSize, rows.length)} /{" "}
                  {rows.length}
                </span>
                <div className="pagination-actions">
                  <button disabled={safePage <= 1} onClick={() => setPage(1)}>
                    首页
                  </button>
                  <button
                    disabled={safePage <= 1}
                    onClick={() => setPage(safePage - 1)}
                  >
                    上一页
                  </button>
                  <strong>
                    {safePage} / {pageCount}
                  </strong>
                  <button
                    disabled={safePage >= pageCount}
                    onClick={() => setPage(safePage + 1)}
                  >
                    下一页
                  </button>
                  <button
                    disabled={safePage >= pageCount}
                    onClick={() => setPage(pageCount)}
                  >
                    末页
                  </button>
                </div>
              </div>
            </td>
          </tr>
        </tfoot>
      ) : null}
    </>
  );
}

function Field({
  children,
  label,
}: {
  children: React.ReactNode;
  label: string;
}) {
  return (
    <label className="field">
      <span>{label}</span>
      {children}
    </label>
  );
}

function PathPickerField({
  buttonLabel,
  disabled,
  onChoose,
  placeholder,
  value,
}: {
  buttonLabel: string;
  disabled?: boolean;
  onChoose: () => void | Promise<void>;
  placeholder: string;
  value: string;
}) {
  return (
    <div className="path-picker-field">
      <input
        readOnly
        disabled={disabled}
        value={value}
        placeholder={placeholder}
      />
      <button
        className="ghost-button"
        disabled={disabled}
        onClick={onChoose}
        type="button"
      >
        {buttonLabel}
      </button>
    </div>
  );
}

function Status({ enabled }: { enabled: boolean }) {
  return (
    <span className={enabled ? "status enabled" : "status disabled"}>
      {enabled ? "启用" : "停用"}
    </span>
  );
}

function EmptyRow({ colSpan }: { colSpan: number }) {
  return (
    <tr>
      <td className="empty-cell" colSpan={colSpan}>
        暂无数据
      </td>
    </tr>
  );
}

export default App;
