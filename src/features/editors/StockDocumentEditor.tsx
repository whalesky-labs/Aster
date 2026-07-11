import { useEffect, useMemo, useState } from "react";
import type { CurrentUser } from "../../entities/users";
import type { Department, Item, Supplier } from "../../entities/master-data";
import type { StockBalanceRow } from "../../entities/stock";
import { Field } from "../../shared/ui/DataTable";
import { ItemSearchSelect } from "../../shared/ui/ItemSearchSelect";
import { effectiveDraftAmount, currentDateTimeString, formatMoney,
  optionName, type StockDocumentDraft, type StockDocumentLineDraft,
  userDisplayName } from "./stockDocumentDraft";
type ApprovalDraft = { entityType: string; entityId: string; reason: string };
export function StockDocumentEditor({
  balances,
  currentUser,
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
  currentUser: CurrentUser | null;
  departments: Department[];
  disabled: boolean;
  documentType: "inbound" | "outbound";
  items: Item[];
  onCreateApproval: (request: ApprovalDraft) => Promise<void>;
  onSaveDraft: (request: StockDocumentDraft) => Promise<void>;
  onSubmit: (request: StockDocumentDraft) => Promise<void>;
  suppliers: Supplier[];
}) {
  const defaultHandler = userDisplayName(currentUser);
  const emptyLine: StockDocumentLineDraft = {
    itemId: "", quantity: 1, unitPrice: 0, amount: null,
    purchaseUnitPrice: null, purchaseAmount: null,
    saleUnitPrice: null, saleAmount: null,
    costUnitPrice: null, costAmount: null,
    remark: "",
  };
  const [draft, setDraft] = useState<StockDocumentDraft>({
    documentId: undefined,
    documentType,
    outboundKind: documentType === "outbound" ? "internal" : undefined,
    businessDate: currentDateTimeString(),
    departmentId: "",
    supplierId: "",
    handler: defaultHandler,
    purpose: "",
    remark: "",
    approvalRequestId: "",
    lines: [emptyLine],
  });
  const [scanCode, setScanCode] = useState("");
  const isOutbound = documentType === "outbound";
  const isInternalOutbound =
    isOutbound && (draft.outboundKind ?? "internal") === "internal";
  const currentOutboundKind = draft.outboundKind ?? "internal";
  const totalAmount = draft.lines.reduce(
    (sum, line) =>
      sum + effectiveDraftAmount(line, documentType, currentOutboundKind),
    0,
  );
  const balanceByItemId = useMemo(
    () => new Map(balances.map((balance) => [balance.itemId, balance])),
    [balances],
  );
  useEffect(() => {
    if (!defaultHandler) return;
    setDraft((current) =>
      current.handler?.trim() ? current : { ...current, handler: defaultHandler },
    );
  }, [defaultHandler]);
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
          const nextPrice =
            documentType === "inbound"
              ? (item?.defaultPrice ?? updated.unitPrice)
              : currentOutboundKind === "guest_sale"
                ? (item?.salePrice ?? updated.unitPrice)
                : 0;
          updated.unitPrice = nextPrice;
          updated.purchaseUnitPrice =
            documentType === "inbound" ? nextPrice : null;
          updated.saleUnitPrice =
            documentType === "outbound" && currentOutboundKind === "guest_sale"
              ? nextPrice
              : null;
          updated.amount = null;
          updated.purchaseAmount = null;
          updated.saleAmount = null;
          updated.costUnitPrice = null;
          updated.costAmount = null;
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
      unitPrice:
        documentType === "inbound"
          ? item.defaultPrice
          : currentOutboundKind === "guest_sale"
            ? item.salePrice
            : 0,
      amount: null,
      purchaseUnitPrice: documentType === "inbound" ? item.defaultPrice : null,
      purchaseAmount: null,
      saleUnitPrice:
        documentType === "outbound" && currentOutboundKind === "guest_sale"
          ? item.salePrice
          : null,
      saleAmount: null,
      costUnitPrice: null,
      costAmount: null,
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
    <div className="editor-document document-entry-editor">
      <div className="editor-form-grid">
        <Field label="业务日期">
          <input
            type="datetime-local"
            step={1}
            value={draft.businessDate}
            onChange={(e) =>
              setDraft({ ...draft, businessDate: e.target.value })
            }
          />
        </Field>
        {isOutbound ? (
          <Field label="出库类型">
            <select
              value={draft.outboundKind ?? "internal"}
              onChange={(e) => {
                const nextOutboundKind = e.target.value as
                  | "internal"
                  | "guest_sale";
                setDraft({
                  ...draft,
                  outboundKind: nextOutboundKind,
                  departmentId:
                    nextOutboundKind === "guest_sale" ? "" : draft.departmentId,
                  approvalRequestId:
                    nextOutboundKind === "guest_sale"
                      ? ""
                      : draft.approvalRequestId,
                  lines: draft.lines.map((line) => {
                    const item = items.find((record) => record.id === line.itemId);
                    if (nextOutboundKind === "guest_sale") {
                      const saleUnitPrice = item?.salePrice ?? line.saleUnitPrice ?? 0;
                      return {
                        ...line,
                        unitPrice: saleUnitPrice,
                        amount: null,
                        saleUnitPrice,
                        saleAmount: null,
                        costUnitPrice: null,
                        costAmount: null,
                      };
                    }
                    return {
                      ...line,
                      unitPrice: 0,
                      amount: null,
                      saleUnitPrice: null,
                      saleAmount: null,
                      costUnitPrice: null,
                      costAmount: null,
                    };
                  }),
                });
              }}
            >
              <option value="internal">内部员工领用</option>
              <option value="guest_sale">酒店客人销售</option>
            </select>
          </Field>
        ) : null}
        {isInternalOutbound ? (
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
        ) : isOutbound ? (
          <Field label="销售对象">
            <input disabled value="酒店客人" />
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
            readOnly
            disabled
            value={draft.handler}
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
        {isInternalOutbound ? (
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
              {isInternalOutbound ? null : (
                <>
                  <th>{isOutbound ? "销售单价" : "本次进价"}</th>
                  <th>{isOutbound ? "销售金额" : "采购金额"}</th>
                </>
              )}
              {isInternalOutbound ? <th>成本核算</th> : null}
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
                {isInternalOutbound ? (
                  <td>
                    <span className="muted-inline">提交后按 FIFO 批次成本计算</span>
                  </td>
                ) : (
                  <>
                    <td>
                      <input
                        className="table-input"
                        min="0"
                        type="number"
                        value={
                          isOutbound
                            ? (line.saleUnitPrice ?? line.unitPrice)
                            : (line.purchaseUnitPrice ?? line.unitPrice)
                        }
                        onChange={(e) => {
                          const value = Number(e.target.value);
                          updateLine(
                            index,
                            isOutbound
                              ? { unitPrice: value, saleUnitPrice: value }
                              : { unitPrice: value, purchaseUnitPrice: value },
                          );
                        }}
                      />
                    </td>
                    <td>
                      <input
                        className="table-input"
                        min="0"
                        placeholder={formatMoney(
                          line.quantity *
                            (isOutbound
                              ? (line.saleUnitPrice ?? line.unitPrice)
                              : (line.purchaseUnitPrice ?? line.unitPrice)),
                        )}
                        type="number"
                        value={
                          isOutbound
                            ? (line.saleAmount ?? "")
                            : (line.purchaseAmount ?? line.amount ?? "")
                        }
                        onChange={(e) => {
                          const value =
                            e.target.value === "" ? null : Number(e.target.value);
                          updateLine(
                            index,
                            isOutbound
                              ? { amount: value, saleAmount: value }
                              : { amount: value, purchaseAmount: value },
                          );
                        }}
                      />
                    </td>
                  </>
                )}
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
        <strong>
          {isOutbound
            ? currentOutboundKind === "guest_sale"
              ? "销售合计"
              : "预计成本"
            : "采购合计"}
          ：{formatMoney(totalAmount)} 元
        </strong>
        {isInternalOutbound ? (
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
