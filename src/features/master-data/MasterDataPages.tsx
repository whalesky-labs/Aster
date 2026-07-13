import { useMemo, useState } from "react";
import type { Category, Department, Unit } from "../../entities/master-data";
import { matchesSearchText } from "../../shared/lib/display";
import { EmptyRow, Status, TableFeatureToolbar, TableSearchToolbar } from "../../shared/ui/DataTable";

type ToggleHandler = (id: string, enabled: boolean, expectedUpdatedAt: string) => Promise<void>;

export function DepartmentsPage({ canWrite, departments, onCreate, onEdit, onToggle }: { canWrite: boolean; departments: Department[]; onCreate: () => void; onEdit: (id: string) => void; onToggle: ToggleHandler }) {
  const [search, setSearch] = useState("");
  const rows = useMemo(() => departments.filter((item) => matchesSearchText(search, [item.code, item.name, item.manager, item.remark, item.enabled ? "启用" : "停用"])), [departments, search]);
  return <section className="table-panel"><TableFeatureToolbar action={<button className="primary-button" disabled={!canWrite} onClick={onCreate} type="button">新增部门</button>} /><TableSearchToolbar onSearchChange={setSearch} placeholder="搜索编码、名称、负责人、备注" search={search} /><table><thead><tr><th>编码</th><th>名称</th><th>负责人</th><th>排序</th><th>状态</th><th>操作</th></tr></thead><tbody>{rows.map((item) => <tr key={item.id}><td>{item.code}</td><td>{item.name}</td><td>{item.manager ?? "-"}</td><td>{item.sortOrder}</td><td><Status enabled={item.enabled} /></td><td className="row-actions"><button disabled={!canWrite} onClick={() => onEdit(item.id)}>编辑</button><button disabled={!canWrite} onClick={() => onToggle(item.id, !item.enabled, item.updatedAt)}>{item.enabled ? "停用" : "启用"}</button></td></tr>)}{rows.length === 0 ? <EmptyRow colSpan={6} /> : null}</tbody></table></section>;
}

export function CategoriesPage({ canWrite, categories, onCreate, onEdit, onToggle }: { canWrite: boolean; categories: Category[]; onCreate: () => void; onEdit: (id: string) => void; onToggle: ToggleHandler }) {
  const [search, setSearch] = useState("");
  const parentName = (id?: string | null) => id ? categories.find((item) => item.id === id)?.name ?? "-" : "大类";
  const rows = useMemo(() => categories.filter((item) => matchesSearchText(search, [item.name, parentName(item.parentId), item.parentId ? "小类" : "大类", item.enabled ? "启用" : "停用"])), [categories, search]);
  return <section className="table-panel"><TableFeatureToolbar action={<button className="primary-button" disabled={!canWrite} onClick={onCreate} type="button">新增分类</button>} /><TableSearchToolbar onSearchChange={setSearch} placeholder="搜索分类、上级分类、类型" search={search} /><table><thead><tr><th>名称</th><th>类型</th><th>上级分类</th><th>排序</th><th>状态</th><th>操作</th></tr></thead><tbody>{rows.map((item) => <tr key={item.id}><td>{item.name}</td><td>{item.parentId ? "小类" : "大类"}</td><td>{parentName(item.parentId)}</td><td>{item.sortOrder}</td><td><Status enabled={item.enabled} /></td><td className="row-actions"><button disabled={!canWrite} onClick={() => onEdit(item.id)}>编辑</button><button disabled={!canWrite} onClick={() => onToggle(item.id, !item.enabled, item.updatedAt)}>{item.enabled ? "停用" : "启用"}</button></td></tr>)}{rows.length === 0 ? <EmptyRow colSpan={6} /> : null}</tbody></table></section>;
}

export function UnitsPage({ canWrite, items, onCreate, onEdit, onToggle }: { canWrite: boolean; items: Unit[]; onCreate: () => void; onEdit: (id: string) => void; onToggle: ToggleHandler }) {
  const [search, setSearch] = useState("");
  const rows = useMemo(() => items.filter((item) => matchesSearchText(search, [item.name, item.sortOrder, item.enabled ? "启用" : "停用"])), [items, search]);
  return <section className="table-panel"><TableFeatureToolbar action={<button className="primary-button" disabled={!canWrite} onClick={onCreate} type="button">新增单位</button>} /><TableSearchToolbar onSearchChange={setSearch} placeholder="搜索名称、排序、状态" search={search} /><table><thead><tr><th>名称</th><th>排序</th><th>状态</th><th>操作</th></tr></thead><tbody>{rows.map((item) => <tr key={item.id}><td>{item.name}</td><td>{item.sortOrder}</td><td><Status enabled={item.enabled} /></td><td className="row-actions"><button disabled={!canWrite} onClick={() => onEdit(item.id)}>编辑</button><button disabled={!canWrite} onClick={() => onToggle(item.id, !item.enabled, item.updatedAt)}>{item.enabled ? "停用" : "启用"}</button></td></tr>)}{rows.length === 0 ? <EmptyRow colSpan={4} /> : null}</tbody></table></section>;
}
