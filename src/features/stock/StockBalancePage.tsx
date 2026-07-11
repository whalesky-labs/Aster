import { type KeyboardEvent, useEffect, useMemo, useState } from "react";
import type { Item, Category } from "../../entities/master-data";
import type { StockBalanceQuery, StockBalanceRow } from "../../entities/stock";
import { Field, PaginatedTable } from "../../shared/ui/DataTable";
import { ItemSearchSelect } from "../../shared/ui/ItemSearchSelect";

function submitOnEnter(event: KeyboardEvent<HTMLDivElement>, submit: () => void) {
  if (event.key !== "Enter" || (event.target as HTMLElement).tagName === "TEXTAREA") return;
  event.preventDefault();
  submit();
}
function formatMoney(value: number) {
  return new Intl.NumberFormat("zh-CN", { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(value);
}

export function StockBalancePage({
  balances,
  categories,
  items,
  onQueryChange,
  onViewBatches,
  onViewMovements,
  query,
}: {
  balances: StockBalanceRow[];
  categories: Category[];
  items: Item[];
  onQueryChange: (query: StockBalanceQuery) => Promise<void>;
  onViewBatches: (itemId: string) => void;
  onViewMovements: (itemId: string) => Promise<void>;
  query: StockBalanceQuery;
}) {
  const [filterDraft, setFilterDraft] = useState<StockBalanceQuery>(query);
  const [quantitySort, setQuantitySort] = useState<"asc" | "desc" | null>(
    null,
  );
  useEffect(() => setFilterDraft({ ...query, search: null }), [query]);
  const searchItems = useMemo(() => {
    if (!filterDraft.categoryId) return items;
    return items.filter((item) => item.categoryId === filterDraft.categoryId);
  }, [filterDraft.categoryId, items]);
  const sortedBalances = useMemo(() => {
    if (!quantitySort) return balances;
    return [...balances].sort((left, right) => {
      const diff = left.quantity - right.quantity;
      return quantitySort === "asc" ? diff : -diff;
    });
  }, [balances, quantitySort]);

  function toggleQuantitySort() {
    setQuantitySort((current) =>
      current === "asc" ? "desc" : current === "desc" ? null : "asc",
    );
  }

  function updateFilter(next: Partial<StockBalanceQuery>) {
    setFilterDraft((current) => ({ ...current, ...next }));
  }

  function updateCategoryFilter(categoryId: string) {
    setFilterDraft((current) => {
      const selectedItem = current.itemId
        ? items.find((item) => item.id === current.itemId)
        : null;
      const shouldClearItem =
        Boolean(categoryId) &&
        Boolean(selectedItem) &&
        selectedItem?.categoryId !== categoryId;
      return {
        ...current,
        categoryId,
        itemId: shouldClearItem ? "" : current.itemId,
      };
    });
  }

  function applyFilters() {
    applyFiltersWithDraft(filterDraft);
  }

  function applyFiltersWithDraft(draft: StockBalanceQuery) {
    onQueryChange({ ...draft, search: null });
  }

  function updateItemFilter(itemId: string) {
    const nextDraft = { ...filterDraft, itemId, search: null };
    setFilterDraft(nextDraft);
    applyFiltersWithDraft(nextDraft);
  }

  function resetFilters() {
    const nextQuery: StockBalanceQuery = {};
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
          <Field label="分类">
            <select
              value={filterDraft.categoryId ?? ""}
              onChange={(e) => updateCategoryFilter(e.target.value)}
            >
              <option value="">全部</option>
              {categories.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.name}
                </option>
              ))}
            </select>
          </Field>
          <Field label="物品">
            <ItemSearchSelect
              allowEmpty
              disabled={false}
              emptyLabel="全部物品"
              items={searchItems}
              value={filterDraft.itemId ?? ""}
              onChange={(itemId) => updateFilter({ itemId })}
              onCommit={updateItemFilter}
            />
          </Field>
          <Field label="库存状态">
            <select
              value={filterDraft.stockStatus ?? ""}
              onChange={(e) =>
                updateFilter({
                  stockStatus: e.target.value as StockBalanceQuery["stockStatus"],
                })
              }
            >
              <option value="">全部</option>
              <option value="normal">正常</option>
              <option value="low">低库存</option>
              <option value="negative">负库存</option>
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
            <th>编码</th>
            <th>物品</th>
            <th>规格</th>
            <th>单位</th>
            <th>
              <button
                className="table-sort-button"
                onClick={toggleQuantitySort}
                type="button"
              >
                库存
                <span>
                  {quantitySort === "asc"
                    ? "↑"
                    : quantitySort === "desc"
                      ? "↓"
                      : "↕"}
                </span>
              </button>
            </th>
            <th>金额</th>
            <th>均价</th>
            <th>预警线</th>
            <th>状态</th>
            <th>操作</th>
          </tr>
        </thead>
        <PaginatedTable
          colSpan={10}
          getRowKey={(row) => row.itemId}
          rows={sortedBalances}
        >
          {(row) => (
            <>
              <td>{row.itemCode}</td>
              <td>{row.itemName}</td>
              <td>{row.spec ?? "-"}</td>
              <td>{row.unitName ?? "-"}</td>
              <td>{row.quantity}</td>
              <td>{formatMoney(row.amount)}</td>
              <td>{formatMoney(row.averagePrice)}</td>
              <td>{row.warningQuantity}</td>
              <td>
                <span
                  className={`status ${row.stockStatus === "normal" ? "enabled" : "disabled"}`}
                >
                  {row.stockStatus === "normal"
                    ? "正常"
                    : row.stockStatus === "low"
                      ? "低库存"
                      : "负库存"}
                </span>
              </td>
              <td className="row-actions">
                <button onClick={() => onViewBatches(row.itemId)}>
                  批次
                </button>
                <button onClick={() => onViewMovements(row.itemId)}>
                  流水
                </button>
              </td>
            </>
          )}
        </PaginatedTable>
      </table>
    </section>
  );
}
