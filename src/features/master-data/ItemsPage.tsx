import { useEffect, useState } from "react";
import type { Item, OptionRecord } from "../../entities/master-data";
import {
  Field,
  PaginatedTable,
  Status,
  TableFeatureToolbar,
  TableSearchToolbar,
} from "../../shared/ui/DataTable";

function optionName(options: OptionRecord[], id?: string | null) {
  return options.find((item) => item.id === id)?.name ?? "-";
}

export function ItemsPage({ canImportItems, canWrite, categories, formatMoney, hasMore, itemSearch, itemSupplierId, items, onCreate, onEdit, onExportItems, onImportItems, onLoadMore, onSearch, onToggle, suppliers, units }: {
  canImportItems: boolean; canWrite: boolean; categories: OptionRecord[]; itemSearch: string; itemSupplierId: string; items: Item[];
  hasMore: boolean;
  formatMoney: (value: number) => string;
  onCreate: () => void; onEdit: (id: string) => void; onExportItems: () => Promise<void>; onImportItems: () => Promise<void>;
  onLoadMore: () => Promise<void>; onSearch: (search: string, supplierId: string) => Promise<void>; onToggle: (id: string, enabled: boolean, expectedUpdatedAt: string) => Promise<void>;
  suppliers: OptionRecord[]; units: OptionRecord[];
}) {
  const [search, setSearch] = useState(itemSearch);
  const [supplierId, setSupplierId] = useState(itemSupplierId);
  useEffect(() => setSearch(itemSearch), [itemSearch]);
  useEffect(() => setSupplierId(itemSupplierId), [itemSupplierId]);
  return <section className="table-panel">
    <TableFeatureToolbar action={<><button className="ghost-button" disabled={!canImportItems} onClick={onImportItems} type="button">导入</button><button className="ghost-button" onClick={onExportItems} type="button">导出</button><button className="primary-button" disabled={!canWrite} onClick={onCreate} type="button">新增物品</button></>}><div className="table-utility-info"><span>物品档案</span><em>{items.length} 条记录</em></div></TableFeatureToolbar>
    <TableSearchToolbar
      onReset={() => {
        setSupplierId("");
        return onSearch("", "");
      }}
      onSearchChange={setSearch}
      onSubmit={(nextSearch) => onSearch(nextSearch, supplierId)}
      placeholder="搜索编码、名称、规格"
      search={search}
      searchLabel="关键字"
    >
      <Field label="供应商">
        <select value={supplierId} onChange={(event) => setSupplierId(event.target.value)}>
          <option value="">全部供应商</option>
          {suppliers.map((supplier) => <option key={supplier.id} value={supplier.id}>{supplier.name}</option>)}
        </select>
      </Field>
    </TableSearchToolbar>
    <table><thead><tr><th>编码</th><th>条码</th><th>名称</th><th>分类</th><th>规格</th><th>单位</th><th>参考进价</th><th>参考售价</th><th>供应商</th><th>状态</th><th>操作</th></tr></thead>
      <PaginatedTable colSpan={11} getRowKey={(item) => item.id} hasMore={hasMore} onLoadMore={onLoadMore} resetKey={`${itemSearch}:${itemSupplierId}`} rows={items}>{(item) => <><td>{item.code}</td><td>{item.barcode ?? "-"}</td><td>{item.name}</td><td>{item.categoryName ?? optionName(categories, item.categoryId)}</td><td>{item.spec ?? "-"}</td><td>{item.unitName ?? optionName(units, item.unitId)}</td><td>{formatMoney(item.defaultPrice)}</td><td>{formatMoney(item.salePrice)}</td><td>{item.supplierName ?? optionName(suppliers, item.supplierId)}</td><td><Status enabled={item.enabled} /></td><td className="row-actions"><button disabled={!canWrite} onClick={() => onEdit(item.id)}>编辑</button><button disabled={!canWrite} onClick={() => onToggle(item.id, !item.enabled, item.updatedAt)}>{item.enabled ? "停用" : "启用"}</button></td></>}</PaginatedTable>
    </table>
  </section>;
}
