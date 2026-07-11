import type { NavKey } from "../../entities/navigation";
import type { AppStatus, RuntimeMode } from "../../entities/runtime";
import type { I18n } from "../../i18n";
import { EmptyRow } from "../../shared/ui/DataTable";

export function Dashboard({ changeMode, i18n, isSavingMode, metricCards, modeLabel, movementTypeLabel, onNavigate, status, workstreams }: {
  changeMode: (mode: RuntimeMode) => void; i18n: I18n; isSavingMode: boolean;
  metricCards: { label: string; value: string | number; suffix: string }[]; modeLabel: (mode: RuntimeMode) => string; movementTypeLabel: (type: string) => string;
  onNavigate: (key: NavKey) => void; status: AppStatus | null; workstreams: { titleKey: string; bodyKey: string }[];
}) {
  const quickActions: { label: string; nav: NavKey }[] = [
    { label: i18n.t("dashboard.quick.inbound"), nav: "inbound" }, { label: i18n.t("dashboard.quick.outbound"), nav: "outbound" },
    { label: i18n.t("dashboard.quick.import"), nav: "import" }, { label: i18n.t("dashboard.quick.reports"), nav: "reports" }, { label: i18n.t("dashboard.quick.stocktake"), nav: "stocktake" },
  ];
  return <>
    <section className="status-grid">
      <div className="status-panel"><span className="panel-label">{i18n.t("dashboard.runtimeMode")}</span><strong>{status ? modeLabel(status.runtime.mode) : i18n.t("dashboard.loading")}</strong><div className="segmented">{(["standalone", "host", "client"] as RuntimeMode[]).map((mode) => <button className={status?.runtime.mode === mode ? "selected" : ""} disabled={isSavingMode} key={mode} onClick={() => changeMode(mode)}>{modeLabel(mode)}</button>)}</div></div>
      <div className="status-panel"><span className="panel-label">{i18n.t("dashboard.database")}</span><strong>{status?.health.databaseOk ? i18n.t("dashboard.databaseHealthy") : i18n.t("dashboard.databasePending")}</strong><p>{status?.health.message ?? i18n.t("dashboard.databaseInitializing")}</p>{status && !status.health.stockBalanceConsistencyOk ? <p className="warning-text">{i18n.t("dashboard.stockBalanceMismatch", { count: status.health.stockBalanceIssueCount })}</p> : null}{!status?.health.secondBackupOk ? <p className="warning-text">{i18n.t("dashboard.secondBackupNotReady")}</p> : null}</div>
      <div className="status-panel"><span className="panel-label">{i18n.t("dashboard.appVersion")}</span><strong>{status?.appVersion ?? "0.1.0"}</strong><p>Schema v{status?.schemaVersion ?? 0}</p></div>
    </section>
    <section className="metrics-grid">{metricCards.map((card) => <div className="metric-card" key={card.label}><span>{card.label}</span><strong>{card.value}</strong><em>{card.suffix}</em></div>)}</section>
    <section className="workspace-grid">
      <div className="module-panel"><div className="section-heading"><h2>{i18n.t("dashboard.quickActions")}</h2><span>{i18n.t("dashboard.quickActionsHint")}</span></div><div className="quick-action-grid">{quickActions.map((item) => <button key={item.nav} onClick={() => onNavigate(item.nav)}>{item.label}</button>)}</div></div>
      <div className="module-panel recent-panel"><div className="section-heading"><h2>{i18n.t("dashboard.recentOperations")}</h2><span>{i18n.t("dashboard.recentOperationsHint")}</span></div><table className="compact-table"><thead><tr><th>{i18n.t("dashboard.table.time")}</th><th>{i18n.t("dashboard.table.type")}</th><th>{i18n.t("dashboard.table.item")}</th><th>{i18n.t("dashboard.table.quantity")}</th><th>{i18n.t("dashboard.table.departmentSupplier")}</th></tr></thead><tbody>{(status?.recentOperations ?? []).map((row) => <tr key={row.id}><td>{row.occurredAt}</td><td>{movementTypeLabel(row.businessType)}</td><td>{row.itemName}</td><td>{row.quantity}</td><td>{row.departmentName ?? row.supplierName ?? "-"}</td></tr>)}{!status || status.recentOperations.length === 0 ? <EmptyRow colSpan={5} /> : null}</tbody></table></div>
      <div className="module-panel"><div className="section-heading"><h2>{i18n.t("dashboard.mainline")}</h2><span>{i18n.t("dashboard.mainlineHint")}</span></div><div className="workstream-list">{workstreams.map((item) => <div className="workstream" key={item.titleKey}><strong>{i18n.t(item.titleKey)}</strong><p>{i18n.t(item.bodyKey)}</p></div>)}</div></div>
      <div className="module-panel"><div className="section-heading"><h2>{i18n.t("dashboard.localData")}</h2><span>{i18n.t("dashboard.localDataHint")}</span></div><dl className="path-list"><dt>{i18n.t("dashboard.dataDir")}</dt><dd>{status?.runtime.dataDir ?? "-"}</dd><dt>SQLite</dt><dd>{status?.runtime.databasePath ?? "-"}</dd><dt>{i18n.t("dashboard.backupDir")}</dt><dd>{status?.runtime.backupDir ?? "-"}</dd><dt>{i18n.t("dashboard.importReportDir")}</dt><dd>{status?.runtime.importReportDir ?? "-"}</dd></dl></div>
    </section>
  </>;
}
