import { useMemo, useState } from "react";
import type { BudgetRule } from "../../entities/master-data";
import { matchesSearchText } from "../../shared/lib/display";
import { EmptyRow, Field, MonthSelect, Status, TableSearchToolbar } from "../../shared/ui/DataTable";

export function BudgetRulesPage({ canManage, formatMoney, month, onCreate, onEdit, onMonthChange, onToggle, rules }: {
  canManage: boolean; formatMoney: (value: number) => string; month: string; onCreate: () => void; onEdit: (id: string, periodMonth: string) => void;
  onMonthChange: (month: string) => Promise<void>; onToggle: (id: string, enabled: boolean, expectedUpdatedAt: string) => Promise<void>; rules: BudgetRule[];
}) {
  const [search, setSearch] = useState("");
  const rows = useMemo(() => rules.filter((rule) => matchesSearchText(search, [rule.periodMonth, rule.departmentName, rule.categoryName || "全部分类", rule.amountLimit, rule.usedAmount, rule.amountLimit - rule.usedAmount, rule.enabled ? "启用" : "停用"])), [rules, search]);
  return <section className="table-panel"><TableSearchToolbar action={<button className="primary-button" disabled={!canManage} onClick={onCreate} type="button">新增预算</button>} extra={<Field label="月份"><MonthSelect compact value={month} onChange={onMonthChange} /></Field>} onSearchChange={setSearch} placeholder="搜索部门、分类、金额、状态" search={search} /><table><thead><tr><th>月份</th><th>部门</th><th>分类</th><th>预算</th><th>已用</th><th>剩余</th><th>状态</th><th>操作</th></tr></thead><tbody>{rows.map((rule) => { const remaining = rule.amountLimit - rule.usedAmount; return <tr key={rule.id}><td>{rule.periodMonth}</td><td>{rule.departmentName}</td><td>{rule.categoryName || "全部分类"}</td><td>{formatMoney(rule.amountLimit)}</td><td>{formatMoney(rule.usedAmount)}</td><td className={remaining < 0 ? "danger-cell" : ""}>{formatMoney(remaining)}</td><td><Status enabled={rule.enabled} /></td><td className="row-actions"><button disabled={!canManage} onClick={() => onEdit(rule.id, rule.periodMonth)}>编辑</button><button disabled={!canManage} onClick={() => onToggle(rule.id, !rule.enabled, rule.updatedAt)}>{rule.enabled ? "停用" : "启用"}</button></td></tr>; })}{rows.length === 0 ? <EmptyRow colSpan={8} /> : null}</tbody></table></section>;
}
