import { useMemo, useState } from "react";
import type { Supplier, SupplierPurchaseRecord } from "../../entities/master-data";
import { matchesSearchText } from "../../shared/lib/display";
import { EmptyRow, Status, TableFeatureToolbar, TableSearchToolbar } from "../../shared/ui/DataTable";

export function SuppliersPage({ activeSupplier, canWrite, formatDateTime, formatMoney, onCreate, onEdit, onSelect, onToggle, purchaseRecords, suppliers }: {
  activeSupplier: Supplier | null; canWrite: boolean; formatDateTime: (value?: string | null) => string; formatMoney: (value: number) => string;
  onCreate: () => void; onEdit: (id: string) => void; onSelect: (supplier: Supplier) => Promise<void>;
  onToggle: (id: string, enabled: boolean, expectedUpdatedAt: string) => Promise<void>; purchaseRecords: SupplierPurchaseRecord[]; suppliers: Supplier[];
}) {
  const [activeTab, setActiveTab] = useState<"suppliers" | "purchases">("suppliers");
  const [search, setSearch] = useState("");
  const supplierRows = useMemo(() => suppliers.filter((item) => matchesSearchText(search, [item.name, item.contact, item.phone, item.address, item.remark, item.enabled ? "启用" : "停用"])), [search, suppliers]);
  const purchaseRows = useMemo(() => purchaseRecords.filter((record) => matchesSearchText(search, [record.movementDate, record.documentNo, record.itemCode, record.itemName, record.spec, record.unitName, record.quantity, record.unitPrice, record.amount, record.remark])), [purchaseRecords, search]);
  async function openPurchases(supplier: Supplier) { await onSelect(supplier); setActiveTab("purchases"); }
  const tabs = <div className="segmented supplier-tabs"><button className={activeTab === "suppliers" ? "selected" : ""} onClick={() => setActiveTab("suppliers")} type="button">供应商档案</button><button className={activeTab === "purchases" ? "selected" : ""} onClick={() => setActiveTab("purchases")} type="button">采购记录</button></div>;
  return <section className="table-panel">
    <TableFeatureToolbar action={<button className="primary-button" disabled={!canWrite} onClick={onCreate} type="button">新增供应商</button>}>{tabs}</TableFeatureToolbar>
    <TableSearchToolbar onSearchChange={setSearch} placeholder={activeTab === "suppliers" ? "搜索名称、联系人、电话、地址" : "搜索单号、物品、规格、备注"} search={search} />
    {activeTab === "suppliers" ? <table><thead><tr><th>名称</th><th>联系人</th><th>电话</th><th>地址</th><th>状态</th><th>操作</th></tr></thead><tbody>{supplierRows.map((item) => <tr key={item.id}><td>{item.name}</td><td>{item.contact ?? "-"}</td><td>{item.phone ?? "-"}</td><td>{item.address ?? "-"}</td><td><Status enabled={item.enabled} /></td><td className="row-actions"><button disabled={!canWrite} onClick={() => onEdit(item.id)}>编辑</button><button disabled={!canWrite} onClick={() => onToggle(item.id, !item.enabled, item.updatedAt)}>{item.enabled ? "停用" : "启用"}</button><button onClick={() => void openPurchases(item)}>采购记录</button></td></tr>)}{supplierRows.length === 0 ? <EmptyRow colSpan={6} /> : null}</tbody></table> :
      <div className="subtable supplier-purchase-panel"><div className="subtable-heading"><h3>{activeSupplier ? `${activeSupplier.name} 采购记录` : "供应商采购记录"}</h3></div><table><thead><tr><th>日期</th><th>单号</th><th>物品</th><th>规格</th><th>单位</th><th>数量</th><th>单价</th><th>金额</th><th>备注</th></tr></thead><tbody>{purchaseRows.map((record, index) => <tr key={`${record.documentNo ?? "doc"}-${record.itemCode}-${index}`}><td>{formatDateTime(record.movementDate)}</td><td>{record.documentNo ?? "-"}</td><td>{record.itemCode} · {record.itemName}</td><td>{record.spec ?? "-"}</td><td>{record.unitName ?? "-"}</td><td>{record.quantity}</td><td>{formatMoney(record.unitPrice)}</td><td>{formatMoney(record.amount)}</td><td>{record.remark ?? "-"}</td></tr>)}{purchaseRows.length === 0 ? <EmptyRow colSpan={9} /> : null}</tbody></table></div>}
  </section>;
}
