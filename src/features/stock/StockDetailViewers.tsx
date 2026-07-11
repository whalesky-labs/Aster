import type { ReactNode } from "react";
import type { StockBatchRow, StockDocumentDetail } from "../../entities/stock";
import { createI18n } from "../../i18n";
import { closeCurrentEditorWindow } from "../../shared/lib/editorWindows";
import { formatDateTime } from "../../shared/lib/display";

const i18n = createI18n("zh-CN");
const formatMoney = (value: number) => i18n.formatMoney(value);
export function StockDocumentDetailViewer({
  detail,
  isLoading,
}: {
  detail: StockDocumentDetail | null;
  isLoading: boolean;
}) {
  if (isLoading) {
    return (
      <ReadOnlyEditorWindow>
        <div className="placeholder-panel">正在加载单据明细...</div>
      </ReadOnlyEditorWindow>
    );
  }
  if (!detail) {
    return (
      <ReadOnlyEditorWindow>
        <div className="placeholder-panel">未找到单据明细</div>
      </ReadOnlyEditorWindow>
    );
  }

  const { document, lines, batchLines } = detail;
  const isOutbound = document.documentType === "outbound";
  const partyLabel = isOutbound ? "对象" : "供应商";
  const partyValue = isOutbound
    ? document.outboundKind === "guest_sale"
      ? "酒店客人"
      : (document.departmentName ?? "-")
    : (document.supplierName ?? "-");

  return (
    <ReadOnlyEditorWindow>
      <div
        className={`document-detail-viewer ${
          batchLines.length > 0 ? "has-batches" : "single-lines"
        }`}
      >
        <div className="detail-summary-grid">
          <InfoTile label="单号" value={document.documentNo} />
          <InfoTile label="日期" value={formatDateTime(document.businessDate)} />
          <InfoTile
            label="类型"
            value={
              isOutbound
                ? outboundKindLabel(document.outboundKind)
                : document.documentType === "inbound"
                  ? "入库"
                  : document.documentType
            }
          />
          <InfoTile label={partyLabel} value={partyValue} />
          <InfoTile label="数量" value={String(document.totalQuantity)} />
          <InfoTile
            label={
              isOutbound
                ? document.outboundKind === "guest_sale"
                  ? "销售金额"
                  : "成本金额"
                : "采购金额"
            }
            value={formatMoney(document.totalAmount)}
          />
          {isOutbound && document.outboundKind === "guest_sale" ? (
            <>
              <InfoTile
                label="销售成本"
                value={formatMoney(document.totalCostAmount)}
              />
              <InfoTile
                label="毛利"
                value={formatMoney(document.totalGrossProfit)}
              />
            </>
          ) : null}
          <InfoTile label="经办人" value={document.handler ?? "-"} />
          <InfoTile label="用途" value={document.purpose ?? "-"} />
        </div>

        <div className="subtable document-detail-lines">
          <div className="subtable-heading">
            <h3>商品明细</h3>
            <span>{lines.length} 项</span>
          </div>
          <div className="document-detail-scroll">
            <table>
              <thead>
                <tr>
                  <th>商品</th>
                  <th>规格</th>
                  <th>单位</th>
                  <th>数量</th>
                  <th>{isOutbound ? "成本单价" : "采购单价"}</th>
                  <th>{isOutbound ? "成本金额" : "采购金额"}</th>
                  {isOutbound && document.outboundKind === "guest_sale" ? (
                    <>
                      <th>销售单价</th>
                      <th>销售金额</th>
                      <th>毛利</th>
                    </>
                  ) : null}
                  <th>备注</th>
                </tr>
              </thead>
              <tbody>
                {lines.map((line) => (
                  <tr key={line.id}>
                    <td>
                      <strong>{line.itemName}</strong>
                      <span className="muted-inline">{line.itemCode}</span>
                    </td>
                    <td>{line.spec ?? "-"}</td>
                    <td>{line.unitName ?? "-"}</td>
                    <td>{line.quantity}</td>
                    <td>
                      {formatMoney(
                        isOutbound
                          ? (line.costUnitPrice ?? line.unitPrice)
                          : (line.purchaseUnitPrice ?? line.unitPrice),
                      )}
                    </td>
                    <td>
                      {formatMoney(
                        isOutbound
                          ? (line.costAmount ?? line.amount)
                          : (line.purchaseAmount ?? line.amount),
                      )}
                    </td>
                    {isOutbound && document.outboundKind === "guest_sale" ? (
                      <>
                        <td>{formatMoney(line.saleUnitPrice ?? 0)}</td>
                        <td>{formatMoney(line.saleAmount ?? 0)}</td>
                        <td>{formatMoney(line.grossProfit ?? 0)}</td>
                      </>
                    ) : null}
                    <td>{line.remark ?? "-"}</td>
                  </tr>
                ))}
                {lines.length === 0 ? (
                  <tr>
                    <td
                      colSpan={
                        isOutbound && document.outboundKind === "guest_sale"
                          ? 10
                          : 7
                      }
                    >
                      暂无商品明细
                    </td>
                  </tr>
                ) : null}
              </tbody>
            </table>
          </div>
        </div>

        {batchLines.length > 0 ? (
          <div className="subtable document-detail-lines">
            <div className="subtable-heading">
              <h3>批次成本明细</h3>
              <span>{batchLines.length} 条</span>
            </div>
            <div className="document-detail-scroll">
              <table>
                <thead>
                  <tr>
                    <th>商品</th>
                    <th>批次号</th>
                    <th>入库日期</th>
                    <th>供应商</th>
                    <th>方向</th>
                    <th>数量</th>
                    <th>批次单价</th>
                    <th>批次金额</th>
                  </tr>
                </thead>
                <tbody>
                  {batchLines.map((line) => (
                    <tr key={line.id}>
                      <td>
                        <strong>{line.itemName}</strong>
                        <span className="muted-inline">{line.itemCode}</span>
                      </td>
                      <td>{line.batchNo}</td>
                      <td>{formatDateTime(line.inboundDate)}</td>
                      <td>{line.supplierName ?? "-"}</td>
                      <td>{line.direction === "in" ? "入库" : "出库"}</td>
                      <td>{line.quantity}</td>
                      <td>{formatMoney(line.unitPrice)}</td>
                      <td>{formatMoney(line.amount)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        ) : null}
      </div>
    </ReadOnlyEditorWindow>
  );
}
export function StockBatchDetailViewer({
  batches,
  isLoading,
}: {
  batches: StockBatchRow[];
  isLoading: boolean;
}) {
  if (isLoading) {
    return (
      <ReadOnlyEditorWindow>
        <div className="placeholder-panel">正在加载批次库存...</div>
      </ReadOnlyEditorWindow>
    );
  }
  if (batches.length === 0) {
    return (
      <ReadOnlyEditorWindow>
        <div className="placeholder-panel">暂无批次库存</div>
      </ReadOnlyEditorWindow>
    );
  }

  const first = batches[0];
  const availableBatches = batches.filter(
    (batch) => batch.status !== "voided" && batch.remainingQuantity > 0,
  );
  const totalRemainingQuantity = availableBatches.reduce(
    (sum, batch) => sum + batch.remainingQuantity,
    0,
  );
  const totalRemainingAmount = availableBatches.reduce(
    (sum, batch) => sum + batch.remainingAmount,
    0,
  );

  return (
    <ReadOnlyEditorWindow>
      <div className="document-detail-viewer single-lines">
        <div className="detail-summary-grid">
          <InfoTile label="物品" value={`${first.itemCode} · ${first.itemName}`} />
          <InfoTile label="批次数" value={String(batches.length)} />
          <InfoTile label="可用批次" value={String(availableBatches.length)} />
          <InfoTile
            label="剩余数量"
            value={String(Number(totalRemainingQuantity.toFixed(6)))}
          />
          <InfoTile label="剩余金额" value={formatMoney(totalRemainingAmount)} />
        </div>

        <div className="subtable document-detail-lines">
          <div className="subtable-heading">
            <h3>批次余额</h3>
            <span>{batches.length} 条</span>
          </div>
          <div className="document-detail-scroll">
            <table>
              <thead>
                <tr>
                  <th>批次号</th>
                  <th>入库日期</th>
                  <th>来源单据</th>
                  <th>供应商</th>
                  <th>原始数量</th>
                  <th>剩余数量</th>
                  <th>批次单价</th>
                  <th>剩余金额</th>
                  <th>状态</th>
                </tr>
              </thead>
              <tbody>
                {batches.map((batch) => (
                  <tr key={batch.id}>
                    <td>{batch.batchNo}</td>
                    <td>{formatDateTime(batch.inboundDate)}</td>
                    <td>{batch.sourceDocumentNo ?? "-"}</td>
                    <td>{batch.supplierName ?? "-"}</td>
                    <td>{batch.originalQuantity}</td>
                    <td>{batch.remainingQuantity}</td>
                    <td>{formatMoney(batch.unitPrice)}</td>
                    <td>{formatMoney(batch.remainingAmount)}</td>
                    <td>{stockBatchStatusLabel(batch.status)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </ReadOnlyEditorWindow>
  );
}

function ReadOnlyEditorWindow({ children }: { children: ReactNode }) {
  return (
    <div className="editor-document readonly-editor-document">
      <div className="readonly-editor-scroll">{children}</div>
      <div className="editor-actions">
        <button
          className="primary-button"
          type="button"
          onClick={() => void closeCurrentEditorWindow()}
        >
          关闭
        </button>
      </div>
    </div>
  );
}

function stockBatchStatusLabel(status: string) {
  if (status === "available") return "可用";
  if (status === "depleted") return "已耗尽";
  if (status === "voided") return "已作废";
  if (status === "adjustment") return "调整批次";
  return status;
}

function InfoTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="info-tile">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function outboundKindLabel(kind?: "internal" | "guest_sale" | null) {
  return kind === "guest_sale" ? "酒店客人销售" : "内部员工领用";
}
