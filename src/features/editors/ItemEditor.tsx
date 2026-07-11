import { useEffect, useState } from "react";
import type { Item, OptionRecord } from "../../entities/master-data";
import type { EditorMode } from "../../shared/lib/editorWindows";
import { Field } from "../../shared/ui/DataTable";
import { EditorForm } from "../../shared/ui/EditorForm";

export const emptyItem = {
  id: undefined as string | undefined,
  expectedUpdatedAt: undefined as string | undefined,
  code: "", barcode: "", name: "", categoryId: "", spec: "", unitId: "",
  defaultPrice: 0, salePrice: 0, supplierId: "", warningQuantity: 0,
  enabled: true, remark: "",
};

export function ItemEditor({
  categories,
  disabled,
  item,
  mode,
  onSave,
  suppliers,
  units,
}: {
  categories: OptionRecord[];
  disabled: boolean;
  item?: Item;
  mode: EditorMode;
  onSave: (request: typeof emptyItem) => Promise<void>;
  suppliers: OptionRecord[];
  units: OptionRecord[];
}) {
  const [draft, setDraft] = useState(emptyItem);
  useEffect(() => {
    if (mode === "edit" && item) {
      setDraft({
        id: item.id,
        expectedUpdatedAt: item.updatedAt,
        code: item.code,
        barcode: item.barcode ?? "",
        name: item.name,
        categoryId: item.categoryId ?? "",
        spec: item.spec ?? "",
        unitId: item.unitId ?? "",
        defaultPrice: item.defaultPrice,
        salePrice: item.salePrice,
        supplierId: item.supplierId ?? "",
        warningQuantity: item.warningQuantity,
        enabled: item.enabled,
        remark: item.remark ?? "",
      });
    }
  }, [item, mode]);
  return (
    <EditorForm
      disabled={disabled}
      saveLabel="保存物品"
      onSave={() => onSave(draft)}
    >
      {mode === "edit" ? (
        <Field label="编码">
          <input value={draft.code || "系统生成"} readOnly />
        </Field>
      ) : null}
      <Field label="条码">
        <input
          value={draft.barcode}
          onChange={(e) => setDraft({ ...draft, barcode: e.target.value })}
        />
      </Field>
      <Field label="名称">
        <input
          value={draft.name}
          onChange={(e) => setDraft({ ...draft, name: e.target.value })}
        />
      </Field>
      <Field label="分类">
        <select
          value={draft.categoryId}
          onChange={(e) => setDraft({ ...draft, categoryId: e.target.value })}
        >
          <option value="">未分类</option>
          {categories.map((record) => (
            <option key={record.id} value={record.id}>
              {record.name}
            </option>
          ))}
        </select>
      </Field>
      <Field label="规格">
        <input
          value={draft.spec}
          onChange={(e) => setDraft({ ...draft, spec: e.target.value })}
        />
      </Field>
      <Field label="单位">
        <select
          value={draft.unitId}
          onChange={(e) => setDraft({ ...draft, unitId: e.target.value })}
        >
          <option value="">未设置</option>
          {units.map((record) => (
            <option key={record.id} value={record.id}>
              {record.name}
            </option>
          ))}
        </select>
      </Field>
      <Field label="参考进价">
        <input
          min="0"
          type="number"
          value={draft.defaultPrice}
          onChange={(e) =>
            setDraft({ ...draft, defaultPrice: Number(e.target.value) })
          }
        />
      </Field>
      <Field label="参考售价">
        <input
          min="0"
          type="number"
          value={draft.salePrice}
          onChange={(e) =>
            setDraft({ ...draft, salePrice: Number(e.target.value) })
          }
        />
      </Field>
      <Field label="供应商">
        <select
          value={draft.supplierId}
          onChange={(e) => setDraft({ ...draft, supplierId: e.target.value })}
        >
          <option value="">未设置</option>
          {suppliers.map((record) => (
            <option key={record.id} value={record.id}>
              {record.name}
            </option>
          ))}
        </select>
      </Field>
      <Field label="预警线">
        <input
          min="0"
          type="number"
          value={draft.warningQuantity}
          onChange={(e) =>
            setDraft({ ...draft, warningQuantity: Number(e.target.value) })
          }
        />
      </Field>
      <Field label="备注">
        <input
          value={draft.remark}
          onChange={(e) => setDraft({ ...draft, remark: e.target.value })}
        />
      </Field>
      <label className="checkbox-field">
        <input
          checked={draft.enabled}
          onChange={(e) => setDraft({ ...draft, enabled: e.target.checked })}
          type="checkbox"
        />
        启用
      </label>
    </EditorForm>
  );
}
