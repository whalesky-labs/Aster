import { type KeyboardEvent, useEffect, useState } from "react";
import type { Item } from "../../entities/master-data";
import type { StockMovementQuery, StockMovementRow } from "../../entities/stock";
import { Field, PaginatedTable } from "../../shared/ui/DataTable";
import { ItemSearchSelect } from "../../shared/ui/ItemSearchSelect";
import { formatDateTime } from "../../shared/lib/display";

function submitOnEnter(event: KeyboardEvent<HTMLDivElement>, submit: () => void) {
  if (event.key !== "Enter" || (event.target as HTMLElement).tagName === "TEXTAREA") return;
  event.preventDefault();
  submit();
}
function formatMoney(value: number) {
  return new Intl.NumberFormat("zh-CN", { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(value);
}
function movementTypeLabel(type: string) {
  const labels: Record<string, string> = { opening: "期初", inbound: "入库", outbound: "出库", stocktake_gain: "盘盈", stocktake_loss: "盘亏", adjustment: "调整", reversal: "冲正" };
  return labels[type] ?? type;
}

export function StockMovementPage({
  hasMore,
  items,
  movements,
  onQueryChange,
  onLoadMore,
  query,
}: {
  hasMore: boolean;
  items: Item[];
  movements: StockMovementRow[];
  onQueryChange: (query: StockMovementQuery) => Promise<void>;
  onLoadMore: () => Promise<void>;
  query: StockMovementQuery;
}) {
  const [filterDraft, setFilterDraft] = useState<StockMovementQuery>(query);
  useEffect(() => setFilterDraft(query), [query]);

  function updateFilter(next: Partial<StockMovementQuery>) {
    setFilterDraft({ ...filterDraft, ...next });
  }

  function applyFilters() {
    applyFiltersWithDraft(filterDraft);
  }

  function applyFiltersWithDraft(draft: StockMovementQuery) {
    onQueryChange(draft);
  }

  function updateItemFilter(itemId: string) {
    const nextDraft = { ...filterDraft, itemId };
    setFilterDraft(nextDraft);
    applyFiltersWithDraft(nextDraft);
  }

  function resetFilters() {
    const nextQuery: StockMovementQuery = {};
    setFilterDraft(nextQuery);
    onQueryChange(nextQuery);
  }

  return (
    <section className="table-panel">
      <div
        className="document-filters"
        onKeyDown={(event) => submitOnEnter(event, applyFilters)}
      >
        <div className="filter-fields">
          <Field label="关键字">
            <input
              placeholder="编码/名称/单号"
              value={filterDraft.search ?? ""}
              onChange={(e) => updateFilter({ search: e.target.value })}
            />
          </Field>
          <Field label="物品">
            <ItemSearchSelect
              allowEmpty
              disabled={false}
              emptyLabel="全部物品"
              items={items}
              value={filterDraft.itemId ?? ""}
              onChange={(itemId) => updateFilter({ itemId })}
              onCommit={updateItemFilter}
            />
          </Field>
          <Field label="方向">
            <select
              value={filterDraft.direction ?? ""}
              onChange={(e) =>
                updateFilter({
                  direction: e.target.value as StockMovementQuery["direction"],
                })
              }
            >
              <option value="">全部</option>
              <option value="in">入</option>
              <option value="out">出</option>
            </select>
          </Field>
          <Field label="流水类型">
            <select
              value={filterDraft.movementType ?? ""}
              onChange={(e) => updateFilter({ movementType: e.target.value })}
            >
              <option value="">全部</option>
              <option value="opening">期初</option>
              <option value="inbound">入库</option>
              <option value="outbound">出库</option>
              <option value="stocktake_gain">盘盈</option>
              <option value="stocktake_loss">盘亏</option>
              <option value="adjustment">调整</option>
              <option value="reversal">冲正</option>
            </select>
          </Field>
        </div>
        <div className="filter-actions document-filter-actions">
          <button className="ghost-button" onClick={resetFilters}>
            清空
          </button>
          <button className="primary-button" onClick={applyFilters}>
            筛选
          </button>
        </div>
      </div>
      <table>
        <thead>
          <tr>
            <th>日期</th>
            <th>单号</th>
            <th>类型</th>
            <th>物品</th>
            <th>方向</th>
            <th>数量</th>
            <th>单价</th>
            <th>金额</th>
            <th>部门</th>
            <th>供应商</th>
            <th>操作人</th>
            <th>备注</th>
          </tr>
        </thead>
        <PaginatedTable
          colSpan={12}
          getRowKey={(row) => row.id}
          hasMore={hasMore}
          onLoadMore={onLoadMore}
          resetKey={JSON.stringify(query)}
          rows={movements}
        >
          {(row) => (
            <>
              <td>{formatDateTime(row.movementDate)}</td>
              <td>{row.documentNo ?? "-"}</td>
              <td>{movementTypeLabel(row.movementType)}</td>
              <td>
                {row.itemCode} · {row.itemName}
              </td>
              <td>{row.direction === "in" ? "入库" : "出库"}</td>
              <td>{row.quantity}</td>
              <td>{formatMoney(row.unitPrice)}</td>
              <td>{formatMoney(row.amount)}</td>
              <td>{row.departmentName ?? "-"}</td>
              <td>{row.supplierName ?? "-"}</td>
              <td>{row.operator ?? "-"}</td>
              <td>{row.remark ?? "-"}</td>
            </>
          )}
        </PaginatedTable>
      </table>
    </section>
  );
}
