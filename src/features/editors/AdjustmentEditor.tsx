import { useEffect, useState } from "react";
import type { CurrentUser } from "../../entities/users";
import type { Item } from "../../entities/master-data";
import { Field } from "../../shared/ui/DataTable";
import { ItemSearchSelect } from "../../shared/ui/ItemSearchSelect";

type AdjustmentLineDraft = {
  itemId: string; direction: "in" | "out"; quantity: number;
  unitPrice: number; amount?: number | null; remark: string;
};
export type AdjustmentDraft = {
  businessDate: string; adjustmentType: "gain" | "loss" | "damage" | "correction";
  handler: string; reason: string; lines: AdjustmentLineDraft[];
};
function currentDateTimeString() {
  const now = new Date();
  return new Date(now.getTime() - now.getTimezoneOffset() * 60 * 1000).toISOString().slice(0, 19);
}
function userDisplayName(user?: CurrentUser | null) {
  return user ? user.displayName?.trim() || user.username : "";
}
function effectiveLineAmount(line: AdjustmentLineDraft) {
  return line.amount && line.amount > 0 ? line.amount : line.quantity * line.unitPrice;
}
function formatMoney(value: number) {
  return new Intl.NumberFormat("zh-CN", { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(value);
}
export function AdjustmentEditor({
  currentUser,
  disabled,
  items,
  onSubmit,
}: {
  currentUser: CurrentUser | null;
  disabled: boolean;
  items: Item[];
  onSubmit: (request: AdjustmentDraft) => Promise<void>;
}) {
  const defaultHandler = userDisplayName(currentUser);
  const emptyLine: AdjustmentLineDraft = {
    itemId: "",
    direction: "out",
    quantity: 1,
    unitPrice: 0,
    amount: null,
    remark: "",
  };
  const [draft, setDraft] = useState<AdjustmentDraft>({
    businessDate: currentDateTimeString(),
    adjustmentType: "damage",
    handler: defaultHandler,
    reason: "",
    lines: [emptyLine],
  });
  const totalAmount = draft.lines.reduce(
    (sum, line) => sum + effectiveLineAmount(line),
    0,
  );
  const correction = draft.adjustmentType === "correction";

  useEffect(() => {
    if (!defaultHandler) return;
    setDraft((current) =>
      current.handler?.trim() ? current : { ...current, handler: defaultHandler },
    );
  }, [defaultHandler]);

  function updateAdjustmentType(type: AdjustmentDraft["adjustmentType"]) {
    const direction = type === "gain" ? "in" : "out";
    setDraft((current) => ({
      ...current,
      adjustmentType: type,
      lines: current.lines.map((line) => ({
        ...line,
        direction: type === "correction" ? line.direction : direction,
      })),
    }));
  }

  function updateLine(index: number, nextLine: Partial<AdjustmentLineDraft>) {
    setDraft((current) => ({
      ...current,
      lines: current.lines.map((line, lineIndex) => {
        if (lineIndex !== index) return line;
        const updated = { ...line, ...nextLine };
        if (nextLine.itemId) {
          const item = items.find((record) => record.id === nextLine.itemId);
          updated.unitPrice = item?.defaultPrice ?? updated.unitPrice;
          updated.amount = null;
        }
        return updated;
      }),
    }));
  }

  function removeLine(index: number) {
    setDraft((current) => {
      const lines = current.lines.filter((_, lineIndex) => lineIndex !== index);
      return { ...current, lines: lines.length ? lines : [emptyLine] };
    });
  }

  return (
    <div className="editor-document document-entry-editor">
      <div className="editor-form-grid">
        <Field label="调整日期">
          <input
            type="datetime-local"
            step={1}
            value={draft.businessDate}
            onChange={(e) =>
              setDraft({ ...draft, businessDate: e.target.value })
            }
          />
        </Field>
        <Field label="调整类型">
          <select
            value={draft.adjustmentType}
            onChange={(e) =>
              updateAdjustmentType(
                e.target.value as AdjustmentDraft["adjustmentType"],
              )
            }
          >
            <option value="gain">盘盈调整</option>
            <option value="loss">盘亏调整</option>
            <option value="damage">损耗调整</option>
            <option value="correction">数据修正</option>
          </select>
        </Field>
        <Field label="经办人">
          <input
            readOnly
            disabled
            value={draft.handler}
          />
        </Field>
        <Field label="调整原因">
          <input
            value={draft.reason}
            onChange={(e) => setDraft({ ...draft, reason: e.target.value })}
          />
        </Field>
      </div>
      <div className="editor-toolbar">
        <h2>调整明细</h2>
        <button
          className="primary-button"
          disabled={disabled}
          onClick={() =>
            setDraft({ ...draft, lines: [...draft.lines, emptyLine] })
          }
        >
          新增一行
        </button>
      </div>
      <div className="editor-table-scroll">
        <table>
          <thead>
            <tr>
              <th>物品</th>
              <th>方向</th>
              <th>数量</th>
              <th>成本单价</th>
              <th>金额</th>
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
                <td>
                  <select
                    className="table-input compact-input"
                    disabled={!correction}
                    value={line.direction}
                    onChange={(e) =>
                      updateLine(index, {
                        direction: e.target.value as "in" | "out",
                      })
                    }
                  >
                    <option value="in">增加</option>
                    <option value="out">减少</option>
                  </select>
                </td>
                <td>
                  <input
                    className="table-input compact-input"
                    min="0"
                    type="number"
                    value={line.quantity}
                    onChange={(e) =>
                      updateLine(index, { quantity: Number(e.target.value) })
                    }
                  />
                </td>
                <td>
                  <input
                    className="table-input compact-input"
                    min="0"
                    type="number"
                    value={line.unitPrice}
                    onChange={(e) =>
                      updateLine(index, { unitPrice: Number(e.target.value) })
                    }
                  />
                </td>
                <td>
                  <input
                    className="table-input compact-input"
                    min="0"
                    placeholder={formatMoney(line.quantity * line.unitPrice)}
                    type="number"
                    value={line.amount ?? ""}
                    onChange={(e) =>
                      updateLine(index, {
                        amount:
                          e.target.value === "" ? null : Number(e.target.value),
                      })
                    }
                  />
                </td>
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
        <strong>合计金额：{formatMoney(totalAmount)} 元</strong>
        <button
          className="primary-button"
          disabled={disabled}
          onClick={() => onSubmit(draft)}
        >
          确认调整
        </button>
      </div>
    </div>
  );
}
