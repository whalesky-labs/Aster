import { useMemo, useState } from "react";
import type { BackupRecord } from "../../entities/operations";
import type { I18n } from "../../i18n";
import {
  backupTypeLabel,
  formatDateTime,
  formatFileSize,
  matchesSearchText,
} from "../../shared/lib/display";
import { PaginatedTable, TableSearchToolbar } from "../../shared/ui/DataTable";

export function BackupRecordsPage({ backups, i18n }: { backups: BackupRecord[]; i18n: I18n }) {
  const [search, setSearch] = useState("");
  const rows = useMemo(
    () => backups.filter((backup) => matchesSearchText(search, [backup.createdAt, backupTypeLabel(backup.backupType, i18n), backup.status, backup.hostName, backup.os, backup.appVersion, backup.schemaVersion, backup.databaseSize, backup.sha256, backup.backupFile, backup.errorMessage])),
    [backups, i18n, search],
  );
  return (
    <section className="table-panel">
      <TableSearchToolbar onSearchChange={setSearch} placeholder="搜索类型、状态、主机、文件、错误" search={search} />
      <table>
        <thead><tr><th>时间</th><th>类型</th><th>状态</th><th>主机</th><th>系统</th><th>版本</th><th>Schema</th><th>大小</th><th>SHA256</th><th>文件</th><th>错误</th></tr></thead>
        <PaginatedTable colSpan={11} getRowKey={(backup) => backup.id} rows={rows}>
          {(backup) => <><td>{formatDateTime(backup.createdAt)}</td><td>{backupTypeLabel(backup.backupType, i18n)}</td><td><span className={backup.status === "success" ? "status enabled" : "status disabled"}>{backup.status}</span></td><td>{backup.hostName ?? "-"}</td><td>{backup.os ?? "-"}</td><td>{backup.appVersion}</td><td>v{backup.schemaVersion}</td><td>{formatFileSize(backup.databaseSize)}</td><td className="path-cell">{backup.sha256 ?? "-"}</td><td className="path-cell">{backup.backupFile}</td><td>{backup.errorMessage ?? "-"}</td></>}
        </PaginatedTable>
      </table>
    </section>
  );
}
