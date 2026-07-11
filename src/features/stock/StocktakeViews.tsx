import { useState } from "react";
import type { StocktakeDetail, StocktakeDocument } from "../../entities/stock";
import { createI18n } from "../../i18n";
import { closeCurrentEditorWindow, openEditorWindow } from "../../shared/lib/editorWindows";
import { EmptyRow, MasterTablePanel } from "../../shared/ui/DataTable";

const i18n = createI18n("zh-CN");
const formatMoney = (value: number) => i18n.formatMoney(value);
const stocktakeStatusLabel = (status: string) => i18n.stocktakeStatusLabel(status);
const stocktakeScopeLabel = (scope: string) => i18n.stocktakeScopeLabel(scope);
function formatDateTime(value?: string | null) {
  if (!value) return "-";
  const normalized = value.trim().replace("T", " ").replace(/Z$/, "");
  const [date, time] = normalized.split(/\s+/, 2);
  return time ? `${date} ${time.split(/[.+-]/)[0]}` : date;
}
function ReadOnlyEditorWindow({ children }: { children: React.ReactNode }) {
  return <div className="editor-document readonly-editor-document">
    <div className="readonly-editor-scroll">{children}</div>
    <div className="editor-actions"><button className="primary-button" type="button" onClick={() => void closeCurrentEditorWindow()}>关闭</button></div>
  </div>;
}
export function StocktakePage({
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
export function StocktakeDetailViewer({
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
