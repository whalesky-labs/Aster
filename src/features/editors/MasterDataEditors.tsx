import { useEffect, useState } from "react";
import type { Category, Department, Supplier, Unit } from "../../entities/master-data";
import type { EditorMode } from "../../shared/lib/editorWindows";
import { Field } from "../../shared/ui/DataTable";
import { EditorForm } from "../../shared/ui/EditorForm";

export type DepartmentDraft = {
  id?: string; expectedUpdatedAt?: string; code: string; name: string;
  manager: string; enabled: boolean; sortOrder: number; remark: string;
};
export type SimpleNameDraft = {
  id?: string; expectedUpdatedAt?: string; name: string; enabled: boolean; sortOrder: number;
};
export type CategoryDraft = {
  id?: string; expectedUpdatedAt?: string; parentId: string; name: string;
  enabled: boolean; sortOrder: number;
};
export type SupplierDraft = {
  id?: string; expectedUpdatedAt?: string; name: string; contact: string;
  phone: string; address: string; enabled: boolean; remark: string;
};

export function DepartmentEditor({
  departments,
  disabled,
  item,
  mode,
  onSave,
}: {
  departments: Department[];
  disabled: boolean;
  item?: Department;
  mode: EditorMode;
  onSave: (request: DepartmentDraft) => Promise<void>;
}) {
  const [draft, setDraft] = useState<DepartmentDraft>({
    code: "",
    name: "",
    manager: "",
    enabled: true,
    sortOrder: departments.length + 1,
    remark: "",
  });
  useEffect(() => {
    if (mode === "edit" && item) {
      setDraft({
        ...item,
        id: item.id,
        expectedUpdatedAt: item.updatedAt,
        manager: item.manager ?? "",
        remark: item.remark ?? "",
      });
    } else {
      setDraft((current) => ({
        ...current,
        sortOrder: departments.length + 1,
      }));
    }
  }, [departments.length, item, mode]);
  return (
    <EditorForm
      disabled={disabled}
      saveLabel="保存部门"
      onSave={() => onSave(draft)}
    >
      <Field label="编码">
        <input
          value={draft.code}
          onChange={(e) => setDraft({ ...draft, code: e.target.value })}
        />
      </Field>
      <Field label="名称">
        <input
          value={draft.name}
          onChange={(e) => setDraft({ ...draft, name: e.target.value })}
        />
      </Field>
      <Field label="负责人">
        <input
          value={draft.manager ?? ""}
          onChange={(e) => setDraft({ ...draft, manager: e.target.value })}
        />
      </Field>
      <Field label="排序">
        <input
          type="number"
          value={draft.sortOrder}
          onChange={(e) =>
            setDraft({ ...draft, sortOrder: Number(e.target.value) })
          }
        />
      </Field>
      <Field label="备注">
        <input
          value={draft.remark ?? ""}
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
export function CategoryEditor({
  categories,
  disabled,
  item,
  mode,
  onSave,
}: {
  categories: Category[];
  disabled: boolean;
  item?: Category;
  mode: EditorMode;
  onSave: (request: CategoryDraft) => Promise<void>;
}) {
  const [draft, setDraft] = useState<CategoryDraft>({
    parentId: "",
    name: "",
    enabled: true,
    sortOrder: categories.length + 1,
  });
  useEffect(() => {
    if (mode === "edit" && item) {
      setDraft({
        id: item.id,
        expectedUpdatedAt: item.updatedAt,
        parentId: item.parentId ?? "",
        name: item.name,
        enabled: item.enabled,
        sortOrder: item.sortOrder,
      });
    } else {
      setDraft((current) => ({ ...current, sortOrder: categories.length + 1 }));
    }
  }, [categories.length, item, mode]);
  const parentOptions = categories.filter(
    (record) => !record.parentId && record.id !== draft.id,
  );
  return (
    <EditorForm
      disabled={disabled}
      saveLabel="保存分类"
      onSave={() => onSave({ ...draft, parentId: draft.parentId || "" })}
    >
      <Field label="上级分类">
        <select
          value={draft.parentId}
          onChange={(e) => setDraft({ ...draft, parentId: e.target.value })}
        >
          <option value="">作为大类</option>
          {parentOptions.map((record) => (
            <option key={record.id} value={record.id}>
              {record.name}
            </option>
          ))}
        </select>
      </Field>
      <Field label="名称">
        <input
          value={draft.name}
          onChange={(e) => setDraft({ ...draft, name: e.target.value })}
        />
      </Field>
      <Field label="排序">
        <input
          type="number"
          value={draft.sortOrder}
          onChange={(e) =>
            setDraft({ ...draft, sortOrder: Number(e.target.value) })
          }
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

export function SimpleNameEditor({
  disabled,
  fallbackSortOrder,
  item,
  label,
  mode,
  onSave,
}: {
  disabled: boolean;
  fallbackSortOrder: number;
  item?: Unit | Category;
  label: string;
  mode: EditorMode;
  onSave: (request: SimpleNameDraft) => Promise<void>;
}) {
  const [draft, setDraft] = useState<SimpleNameDraft>({
    name: "",
    enabled: true,
    sortOrder: fallbackSortOrder,
  });
  useEffect(() => {
    if (mode === "edit" && item) {
      setDraft({
        id: item.id,
        expectedUpdatedAt: item.updatedAt,
        name: item.name,
        enabled: item.enabled,
        sortOrder: item.sortOrder,
      });
    } else {
      setDraft((current) => ({ ...current, sortOrder: fallbackSortOrder }));
    }
  }, [fallbackSortOrder, item, mode]);
  return (
    <EditorForm
      disabled={disabled}
      saveLabel={`保存${label}`}
      onSave={() => onSave(draft)}
    >
      <Field label="名称">
        <input
          value={draft.name}
          onChange={(e) => setDraft({ ...draft, name: e.target.value })}
        />
      </Field>
      <Field label="排序">
        <input
          type="number"
          value={draft.sortOrder}
          onChange={(e) =>
            setDraft({ ...draft, sortOrder: Number(e.target.value) })
          }
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

export function SupplierEditor({
  disabled,
  item,
  mode,
  onSave,
}: {
  disabled: boolean;
  item?: Supplier;
  mode: EditorMode;
  onSave: (request: SupplierDraft) => Promise<void>;
}) {
  const [draft, setDraft] = useState<SupplierDraft>({
    name: "",
    contact: "",
    phone: "",
    address: "",
    enabled: true,
    remark: "",
  });
  useEffect(() => {
    if (mode === "edit" && item) {
      setDraft({
        ...item,
        id: item.id,
        expectedUpdatedAt: item.updatedAt,
        contact: item.contact ?? "",
        phone: item.phone ?? "",
        address: item.address ?? "",
        remark: item.remark ?? "",
      });
    }
  }, [item, mode]);
  return (
    <EditorForm
      disabled={disabled}
      saveLabel="保存供应商"
      onSave={() => onSave(draft)}
    >
      <Field label="名称">
        <input
          value={draft.name}
          onChange={(e) => setDraft({ ...draft, name: e.target.value })}
        />
      </Field>
      <Field label="联系人">
        <input
          value={draft.contact ?? ""}
          onChange={(e) => setDraft({ ...draft, contact: e.target.value })}
        />
      </Field>
      <Field label="电话">
        <input
          value={draft.phone ?? ""}
          onChange={(e) => setDraft({ ...draft, phone: e.target.value })}
        />
      </Field>
      <Field label="地址">
        <input
          value={draft.address ?? ""}
          onChange={(e) => setDraft({ ...draft, address: e.target.value })}
        />
      </Field>
      <Field label="备注">
        <input
          value={draft.remark ?? ""}
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
