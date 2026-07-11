import type { ReactNode } from "react";

function formatMoney(value: number) {
  return new Intl.NumberFormat("zh-CN", { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(value);
}

export function ReportGroupHeader({
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

export function ReportInsightGrid({ children }: { children: ReactNode }) {
  return <div className="report-insight-grid">{children}</div>;
}

export function ReportInsightCard({
  detail,
  label,
  value,
}: {
  detail: string;
  label: string;
  value: string;
}) {
  return (
    <div className="report-insight-card">
      <span>{label}</span>
      <strong>{value}</strong>
      <p>{detail}</p>
    </div>
  );
}

export function BarChartPanel({
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
