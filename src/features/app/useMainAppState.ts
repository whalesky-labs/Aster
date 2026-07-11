import { useRef, useState } from "react";
import type { ApprovalRequest } from "../../entities/approvals";
import type { NavKey } from "../../entities/navigation";
import type { AuditLogRow, BackupRecord } from "../../entities/operations";
import type { AppStatus, AppUpdateState, AppearanceSettings, ClientConnectionInfo, HostConnectionTestResult, HostServiceStatus, SystemSettings } from "../../entities/runtime";
import type { BudgetRule, Category, Department, Item, Supplier, SupplierPurchaseRecord, Unit } from "../../entities/master-data";
import type { CurrentUser, UserAccount } from "../../entities/users";
import type { StockBalanceQuery, StockBalanceRow, StockDocument, StockDocumentQuery, StockMovementQuery, StockMovementRow, StocktakeDocument } from "../../entities/stock";
import type { ReportBundle, ReportQuery } from "../../entities/reports";
import type { ImportPreview, ImportResult } from "../../entities/imports";
import { loadAppearanceSettings } from "../settings/appearance";

export type BackupSummary = {
  backupFile: string; backupType: string; createdAt: string; schemaVersion: number;
  sourceHostName: string; sourceOs: string; databaseSize: number; databaseSha256: string;
  secondBackupFile?: string | null;
};
const currentMonthString = () => new Date().toISOString().slice(0, 7);
const initialUpdateState: AppUpdateState = {
  status: "idle", currentVersion: null, latestVersion: null, notes: null,
  downloadedBytes: 0, totalBytes: null, error: null, checkedAt: null, sourceLabel: null,
};

export function useMainAppState() {
  const [activeNav, setActiveNav] = useState<NavKey>("dashboard");
  const [appearanceSettings, setAppearanceSettings] =
    useState<AppearanceSettings>(() => loadAppearanceSettings());
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
  return {
    activeNav, activeSupplier, adjustmentDocumentQuery, adjustmentDocuments, appearanceSettings,
    approvalRequests, auditLogs, backupRecords, budgetRules, categories, clientConnectionCheckedAt,
    clientConnections, currentUser, departments, error, hasCheckedUpdateOnStartupRef,
    hasManualReportMonth, hostStatus, hostTestResult, importPreview, importResult, inboundDocumentQuery,
    inboundDocuments, isBackupWorking, isImporting, isLoginPending, isSavingMode, itemSearch, items,
    lastBackup, lastExportPath, notice, outboundDocumentQuery, outboundDocuments, passwordChangeRequired,
    reportBundle, reportMonth, reportQuery, setActiveNav, setActiveSupplier, setAdjustmentDocumentQuery,
    setAdjustmentDocuments, setAppearanceSettings, setApprovalRequests, setAuditLogs, setBackupRecords,
    setBudgetRules, setCategories, setClientConnectionCheckedAt, setClientConnections, setCurrentUser,
    setDepartments, setError, setHasManualReportMonth, setHostStatus, setHostTestResult, setImportPreview,
    setImportResult, setInboundDocumentQuery, setInboundDocuments, setIsBackupWorking, setIsImporting,
    setIsLoginPending, setIsSavingMode, setItemSearch, setItems, setLastBackup, setLastExportPath,
    setNotice, setOutboundDocumentQuery, setOutboundDocuments, setPasswordChangeRequired,
    setReportBundle, setReportMonth, setReportQuery, setStockBalanceQuery, setStockBalances,
    setStockMovementQuery, setStockMovements, setStocktakes, setSupplierPurchaseRecords,
    setSuppliers, setSystemSettings, setUnits, setUpdateState, setUserAccounts, status, setStatus,
    stockBalanceQuery, stockBalances, stockMovementQuery, stockMovements, stocktakes,
    supplierPurchaseRecords, suppliers, systemSettings, units, updateState, userAccounts,
  };
}
export type MainAppState = ReturnType<typeof useMainAppState>;
