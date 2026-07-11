import { useState } from "react";
import type { Item, OptionRecord } from "../../entities/master-data";
import { Field } from "../../shared/ui/DataTable";
import { EditorForm } from "../../shared/ui/EditorForm";

function currentDateTimeString() {
  const now = new Date();
  return new Date(now.getTime() - now.getTimezoneOffset() * 60 * 1000)
    .toISOString()
    .slice(0, 19);
}

export function StocktakeCreateEditor({
  categories,
  disabled,
  items,
  onCreate,
}: {
  categories: OptionRecord[];
  disabled: boolean;
  items: Item[];
  onCreate: (request: {
    businessDate: string;
    scopeType: string;
    categoryId?: string | null;
    itemIds: string[];
    handler?: string | null;
    remark?: string | null;
  }) => Promise<void>;
}) {
  const [businessDate, setBusinessDate] = useState(currentDateTimeString());
  const [scopeType, setScopeType] = useState<"all" | "category" | "custom">("all");
  const [categoryId, setCategoryId] = useState("");
  const [selectedItemId, setSelectedItemId] = useState("");
  const [customItemIds, setCustomItemIds] = useState<string[]>([]);
  const [handler, setHandler] = useState("");
  const [remark, setRemark] = useState("");

  function addCustomItem() {
    if (selectedItemId && !customItemIds.includes(selectedItemId)) {
      setCustomItemIds([...customItemIds, selectedItemId]);
    }
    setSelectedItemId("");
  }

  return (
    <EditorForm
      disabled={disabled}
      saveLabel="创建盘点单"
      onSave={() =>
        onCreate({
          businessDate,
          scopeType,
          categoryId,
          itemIds: customItemIds,
          handler,
          remark,
        })
      }
    >
      <Field label="盘点日期">
        <input
          type="datetime-local"
          step={1}
          value={businessDate}
          onChange={(event) => setBusinessDate(event.target.value)}
        />
      </Field>
      <Field label="盘点范围">
        <select
          value={scopeType}
          onChange={(event) =>
            setScopeType(event.target.value as "all" | "category" | "custom")
          }
        >
          <option value="all">全部物品</option>
          <option value="category">按分类</option>
          <option value="custom">自定义物品</option>
        </select>
      </Field>
      {scopeType === "category" ? (
        <Field label="分类">
          <select value={categoryId} onChange={(event) => setCategoryId(event.target.value)}>
            <option value="">请选择分类</option>
            {categories.map((category) => (
              <option key={category.id} value={category.id}>{category.name}</option>
            ))}
          </select>
        </Field>
      ) : null}
      {scopeType === "custom" ? (
        <div className="custom-picker">
          <Field label="物品">
            <select
              value={selectedItemId}
              onChange={(event) => setSelectedItemId(event.target.value)}
            >
              <option value="">请选择物品</option>
              {items.map((item) => (
                <option key={item.id} value={item.id}>{item.code} · {item.name}</option>
              ))}
            </select>
          </Field>
          <button className="ghost-button" disabled={disabled} onClick={addCustomItem}>
            加入
          </button>
          <div className="selected-tags">
            {customItemIds.map((selectedId) => {
              const item = items.find((record) => record.id === selectedId);
              return (
                <button
                  key={selectedId}
                  onClick={() =>
                    setCustomItemIds(customItemIds.filter((itemId) => itemId !== selectedId))
                  }
                >
                  {item?.name ?? selectedId} x
                </button>
              );
            })}
          </div>
        </div>
      ) : null}
      <Field label="经办人">
        <input value={handler} onChange={(event) => setHandler(event.target.value)} />
      </Field>
      <Field label="备注">
        <input value={remark} onChange={(event) => setRemark(event.target.value)} />
      </Field>
    </EditorForm>
  );
}
