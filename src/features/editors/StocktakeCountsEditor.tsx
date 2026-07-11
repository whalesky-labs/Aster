import { useEffect, useState } from "react";
import type { StocktakeDetail, StocktakeDocument } from "../../entities/stock";
import { createI18n } from "../../i18n";
import { formatDateTime } from "../../shared/lib/display";
import { EmptyRow, Field } from "../../shared/ui/DataTable";

const i18n = createI18n("zh-CN");

function formatMoney(value: number) {
  return new Intl.NumberFormat("zh-CN", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(value);
}

export function StocktakeCountsEditor({
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
    const nextDrafts: Record<string, { countedQuantity: string; remark: string }> = {};
    for (const line of detail?.lines ?? []) {
      nextDrafts[line.id] = {
        countedQuantity: line.countedQuantity == null ? "" : String(line.countedQuantity),
        remark: line.remark ?? "",
      };
    }
    setLineDrafts(nextDrafts);
  }, [detail]);

  function saveCounts() {
    if (!detail) return Promise.resolve();
    const lines = Object.entries(lineDrafts).map(([lineId, draft]) => ({
      lineId,
      countedQuantity:
        draft.countedQuantity.trim() === "" ? null : Number(draft.countedQuantity),
      remark: draft.remark,
    }));
    return onSaveCounts(detail.document.id, lines);
  }

  const canEdit =
    detail && detail.document.status !== "confirmed" && detail.document.status !== "voided";

  return (
    <div className="editor-document stocktake-counts-editor">
      <div className="editor-toolbar">
        <Field label="盘点单">
          <select
            value={detail?.document.id ?? ""}
            onChange={(event) => onSelect(event.target.value)}
          >
            {stocktakes.map((stocktake) => (
              <option key={stocktake.id} value={stocktake.id}>
                {stocktake.documentNo} · {formatDateTime(stocktake.businessDate)} ·{" "}
                {i18n.stocktakeStatusLabel(stocktake.status)}
              </option>
            ))}
          </select>
        </Field>
      </div>
      {detail ? (
        <section className="metrics-grid stocktake-metrics">
          <div className="metric-card"><span>盘点行数</span><strong>{detail.document.lineCount}</strong><em>行</em></div>
          <div className="metric-card"><span>已录入</span><strong>{detail.document.countedCount}</strong><em>行</em></div>
          <div className="metric-card">
            <span>状态</span>
            <strong>{i18n.stocktakeStatusLabel(detail.document.status)}</strong>
            <em>{i18n.stocktakeScopeLabel(detail.document.scopeType)}</em>
          </div>
          <div className="metric-card">
            <span>盘盈/盘亏</span>
            <strong>{formatMoney(detail.document.gainAmount)} / {formatMoney(detail.document.lossAmount)}</strong>
            <em>元</em>
          </div>
        </section>
      ) : null}
      <div className="editor-table-scroll">
        <table>
          <thead>
            <tr><th>编码</th><th>物品</th><th>规格</th><th>单位</th><th>账面</th><th>实盘</th><th>差异</th><th>差异金额</th><th>备注</th></tr>
          </thead>
          <tbody>
            {(detail?.lines ?? []).map((line) => {
              const draft = lineDrafts[line.id] ?? { countedQuantity: "", remark: "" };
              return (
                <tr key={line.id}>
                  <td>{line.itemCode}</td><td>{line.itemName}</td><td>{line.spec ?? "-"}</td><td>{line.unitName ?? "-"}</td><td>{line.bookQuantity}</td>
                  <td>
                    <input
                      className="table-input compact-input"
                      disabled={!canEdit || disabled}
                      min="0"
                      type="number"
                      value={draft.countedQuantity}
                      onChange={(event) => setLineDrafts({ ...lineDrafts, [line.id]: { ...draft, countedQuantity: event.target.value } })}
                    />
                  </td>
                  <td className={line.differenceQuantity === 0 ? "" : line.differenceQuantity > 0 ? "gain-text" : "loss-text"}>{line.differenceQuantity}</td>
                  <td>{formatMoney(line.differenceAmount)}</td>
                  <td>
                    <input
                      className="table-input"
                      disabled={!canEdit || disabled}
                      value={draft.remark}
                      onChange={(event) => setLineDrafts({ ...lineDrafts, [line.id]: { ...draft, remark: event.target.value } })}
                    />
                  </td>
                </tr>
              );
            })}
            {!detail || detail.lines.length === 0 ? <EmptyRow colSpan={9} /> : null}
          </tbody>
        </table>
      </div>
      <div className="editor-actions">
        <button className="primary-button" disabled={disabled || !canEdit} onClick={saveCounts}>
          保存实盘
        </button>
      </div>
    </div>
  );
}
