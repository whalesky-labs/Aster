import { useMemo, useState } from "react";
import type { AuditLogRow } from "../../entities/operations";
import type { I18n } from "../../i18n";
import { auditActionLabel, auditEntityLabel, formatDateTime, matchesSearchText } from "../../shared/lib/display";
import { PaginatedTable, TableSearchToolbar } from "../../shared/ui/DataTable";

export function LogsPage({ auditLogs, i18n }: { auditLogs: AuditLogRow[]; i18n: I18n }) {
  const [search, setSearch] = useState("");
  const rows = useMemo(
    () => auditLogs.filter((log) => matchesSearchText(search, [log.createdAt, auditActionLabel(log.action, i18n), auditEntityLabel(log.entityType, i18n), log.entityId, log.summary, log.operator])),
    [auditLogs, i18n, search],
  );
  return (
    <section className="table-panel">
      <TableSearchToolbar onSearchChange={setSearch} placeholder="搜索动作、对象、摘要、操作人" search={search} />
      <table>
        <thead><tr><th>时间</th><th>动作</th><th>对象</th><th>摘要</th><th>操作人</th></tr></thead>
        <PaginatedTable colSpan={5} getRowKey={(log) => log.id} rows={rows}>
          {(log) => <><td>{formatDateTime(log.createdAt)}</td><td>{auditActionLabel(log.action, i18n)}</td><td><span className="audit-entity">{auditEntityLabel(log.entityType, i18n)}</span><span className="audit-entity-id">{log.entityId}</span></td><td className="audit-summary">{log.summary}</td><td>{log.operator}</td></>}
        </PaginatedTable>
      </table>
    </section>
  );
}
