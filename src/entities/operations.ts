export type BackupRecord = {
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

export type AuditLogRow = {
  id: string;
  action: string;
  entityType: string;
  entityId: string;
  summary: string;
  operator: string;
  createdAt: string;
};
