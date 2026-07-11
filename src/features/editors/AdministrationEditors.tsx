import { useEffect, useState } from "react";
import type { BudgetRule, Category, Department } from "../../entities/master-data";
import type { Role, UserAccount } from "../../entities/users";
import type { EditorMode } from "../../shared/lib/editorWindows";
import { Field, MonthSelect } from "../../shared/ui/DataTable";
import { EditorForm } from "../../shared/ui/EditorForm";

export type BudgetRuleDraft = {
  id?: string; expectedUpdatedAt?: string; departmentId: string;
  categoryId?: string | null; periodMonth: string; amountLimit: number; enabled: boolean;
};

export function BudgetRuleEditor({
  categories,
  departments,
  disabled,
  mode,
  onSave,
  periodMonth,
  rule,
}: {
  categories: Category[];
  departments: Department[];
  disabled: boolean;
  mode: EditorMode;
  onSave: (request: BudgetRuleDraft) => Promise<void>;
  periodMonth: string;
  rule?: BudgetRule;
}) {
  const [draft, setDraft] = useState<BudgetRuleDraft>({
    departmentId: departments[0]?.id ?? "",
    categoryId: null,
    periodMonth,
    amountLimit: 0,
    enabled: true,
  });
  useEffect(() => {
    if (mode === "edit" && rule) {
      setDraft({
        id: rule.id,
        expectedUpdatedAt: rule.updatedAt,
        departmentId: rule.departmentId,
        categoryId: rule.categoryId ?? null,
        periodMonth: rule.periodMonth,
        amountLimit: rule.amountLimit,
        enabled: rule.enabled,
      });
    } else {
      setDraft((current) => ({
        ...current,
        departmentId: current.departmentId || departments[0]?.id || "",
        periodMonth: current.periodMonth || periodMonth,
      }));
    }
  }, [categories, departments, mode, periodMonth, rule]);
  return (
    <EditorForm
      disabled={
        disabled ||
        !draft.departmentId ||
        !draft.periodMonth
      }
      saveLabel="保存预算"
      onSave={() => onSave(draft)}
    >
      <Field label="月份">
        <MonthSelect
          value={draft.periodMonth}
          onChange={(periodMonth) => setDraft({ ...draft, periodMonth })}
        />
      </Field>
      <Field label="部门">
        <select
          value={draft.departmentId}
          onChange={(e) => setDraft({ ...draft, departmentId: e.target.value })}
        >
          {departments.map((record) => (
            <option key={record.id} value={record.id}>
              {record.name}
            </option>
          ))}
        </select>
      </Field>
      <Field label="分类">
        <select
          value={draft.categoryId ?? ""}
          onChange={(e) =>
            setDraft({ ...draft, categoryId: e.target.value || null })
          }
        >
          <option value="">全部分类</option>
          {categories.map((record) => (
            <option key={record.id} value={record.id}>
              {record.name}
            </option>
          ))}
        </select>
      </Field>
      <Field label="预算金额">
        <input
          min="0"
          step="0.01"
          type="number"
          value={draft.amountLimit}
          onChange={(e) =>
            setDraft({ ...draft, amountLimit: Number(e.target.value) })
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
export function UserEditor({
  departments,
  disabled,
  mode,
  onSave,
  roles,
  user,
}: {
  departments: Department[];
  disabled: boolean;
  mode: EditorMode;
  onSave: (request: {
    id?: string;
    username: string;
    displayName: string;
    email?: string | null;
    password?: string | null;
    departmentId?: string | null;
    enabled: boolean;
    roleCodes: string[];
  }) => Promise<void>;
  roles: Role[];
  user?: UserAccount;
}) {
  const empty = {
    id: undefined as string | undefined,
    username: "",
    displayName: "",
    email: "",
    password: "",
    departmentId: "",
    enabled: true,
    roleCodes: ["warehouse"] as string[],
  };
  const [draft, setDraft] = useState(empty);
  useEffect(() => {
    if (mode === "edit" && user) {
      setDraft({
        id: user.id,
        username: user.username,
        displayName: user.displayName,
        email: user.email ?? "",
        password: "",
        departmentId: user.departmentId ?? "",
        enabled: user.enabled,
        roleCodes: user.roles.map((role) => role.code),
      });
    }
  }, [mode, user]);
  function toggleRole(code: string) {
    setDraft((current) => ({
      ...current,
      roleCodes: current.roleCodes.includes(code)
        ? current.roleCodes.filter((item) => item !== code)
        : [...current.roleCodes, code],
    }));
  }
  return (
    <EditorForm
      disabled={disabled}
      saveLabel="保存用户"
      onSave={() =>
        onSave({
          ...draft,
          email: draft.email.trim() ? draft.email.trim() : null,
          departmentId: draft.departmentId || null,
          password: draft.password.trim() ? draft.password : null,
        })
      }
    >
      <Field label="用户名">
        <input
          value={draft.username}
          onChange={(e) => setDraft({ ...draft, username: e.target.value })}
        />
      </Field>
      <Field label="显示名称">
        <input
          value={draft.displayName}
          onChange={(e) => setDraft({ ...draft, displayName: e.target.value })}
        />
      </Field>
      <Field label="邮箱">
        <input
          autoComplete="email"
          value={draft.email}
          onChange={(e) => setDraft({ ...draft, email: e.target.value })}
        />
      </Field>
      <Field label={draft.id ? "新密码" : "初始密码"}>
        <input
          value={draft.password}
          onChange={(e) => setDraft({ ...draft, password: e.target.value })}
          type="password"
        />
      </Field>
      <Field label="所属部门">
        <select
          value={draft.departmentId}
          onChange={(e) => setDraft({ ...draft, departmentId: e.target.value })}
        >
          <option value="">不绑定部门</option>
          {departments.map((department) => (
            <option key={department.id} value={department.id}>
              {department.name}
            </option>
          ))}
        </select>
      </Field>
      <div className="role-checks">
        {roles.map((role) => (
          <label key={role.code}>
            <input
              checked={draft.roleCodes.includes(role.code)}
              onChange={() => toggleRole(role.code)}
              type="checkbox"
            />
            <span>{role.name}</span>
          </label>
        ))}
      </div>
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
