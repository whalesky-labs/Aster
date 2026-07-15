import { useEffect, useState, type ReactNode } from "react";

const DEFAULT_TABLE_PAGE_SIZE = 20;

export function MasterTablePanel({
  actions,
  children,
  description,
  hideHeading = false,
  title,
}: {
  actions?: ReactNode;
  children: ReactNode;
  description: string;
  hideHeading?: boolean;
  title: string;
}) {
  return (
    <section className="table-panel">
      <div
        className={hideHeading ? "table-toolbar actions-only" : "table-toolbar"}
      >
        {hideHeading ? null : (
          <div>
            <h2>{title}</h2>
            <span className="table-note">{description}</span>
          </div>
        )}
        {actions ? <div className="toolbar-actions">{actions}</div> : null}
      </div>
      {children}
    </section>
  );
}

export function TableSearchToolbar({
  children,
  onReset,
  onSearchChange,
  onSubmit,
  placeholder,
  search,
  searchLabel = "搜索",
  submitLabel = "筛选",
}: {
  children?: ReactNode;
  onReset?: () => void | Promise<void>;
  onSearchChange: (value: string) => void;
  onSubmit?: (value: string) => void | Promise<void>;
  placeholder: string;
  search: string;
  searchLabel?: string;
  submitLabel?: string;
}) {
  const [draft, setDraft] = useState(search);
  useEffect(() => setDraft(search), [search]);

  function applySearch(value: string) {
    onSearchChange(value);
    void onSubmit?.(value);
  }

  function resetSearch() {
    setDraft("");
    onSearchChange("");
    if (onReset) void onReset();
    else void onSubmit?.("");
  }

  return (
    <form
      className="document-filters table-search-toolbar"
      onSubmit={(event) => {
        event.preventDefault();
        applySearch(draft.trim());
      }}
    >
      <div className="filter-fields">
        <Field label={searchLabel}>
          <input
            placeholder={placeholder}
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
          />
        </Field>
        {children}
      </div>
      <div className="filter-actions document-filter-actions">
        <button
          className="ghost-button"
          onClick={resetSearch}
          type="button"
        >
          清空
        </button>
        <button className="primary-button" type="submit">
          {submitLabel}
        </button>
      </div>
    </form>
  );
}

export function TableFeatureToolbar({
  action,
  children,
}: {
  action?: ReactNode;
  children?: ReactNode;
}) {
  return (
    <div className="table-toolbar table-feature-toolbar">
      {children ? <div className="table-feature-controls">{children}</div> : null}
      {action ? <div className="toolbar-actions">{action}</div> : null}
    </div>
  );
}

export function PaginatedTable<T>({
  children,
  colSpan,
  empty,
  getRowKey,
  hasMore = false,
  onLoadMore,
  pageSize = DEFAULT_TABLE_PAGE_SIZE,
  resetKey,
  rows,
}: {
  children: (row: T, index: number) => ReactNode;
  colSpan: number;
  empty?: ReactNode;
  getRowKey: (row: T, index: number) => React.Key;
  hasMore?: boolean;
  onLoadMore?: () => Promise<void>;
  pageSize?: number;
  resetKey?: unknown;
  rows: T[];
}) {
  const [page, setPage] = useState(1);
  const [isLoadingMore, setIsLoadingMore] = useState(false);
  const pageCount = Math.max(1, Math.ceil(rows.length / pageSize));
  const safePage = Math.min(page, pageCount);

  useEffect(() => {
    setPage(1);
  }, [resetKey, pageSize]);

  useEffect(() => {
    if (page !== safePage) setPage(safePage);
  }, [page, safePage]);

  const start = (safePage - 1) * pageSize;
  const visibleRows = rows.slice(start, start + pageSize);

  async function nextPage() {
    if (safePage < pageCount) {
      setPage(safePage + 1);
      return;
    }
    if (!hasMore || !onLoadMore || isLoadingMore) return;
    setIsLoadingMore(true);
    try {
      await onLoadMore();
      setPage(safePage + 1);
    } finally {
      setIsLoadingMore(false);
    }
  }

  return (
    <>
      <tbody>
        {visibleRows.map((row, index) => (
          <tr key={getRowKey(row, start + index)}>
            {children(row, start + index)}
          </tr>
        ))}
        {rows.length === 0 ? (empty ?? <EmptyRow colSpan={colSpan} />) : null}
      </tbody>
      {rows.length > pageSize || hasMore ? (
        <tfoot>
          <tr>
            <td colSpan={colSpan}>
              <div className="pagination-bar">
                <span>
                  {start + 1}-{Math.min(start + pageSize, rows.length)} /{" "}
                  {rows.length}{hasMore ? "+" : ""}
                </span>
                <div className="pagination-actions">
                  <button disabled={safePage <= 1} onClick={() => setPage(1)}>
                    首页
                  </button>
                  <button
                    disabled={safePage <= 1}
                    onClick={() => setPage(safePage - 1)}
                  >
                    上一页
                  </button>
                  <strong>
                    {safePage} / {pageCount}
                  </strong>
                  <button
                    disabled={(!hasMore && safePage >= pageCount) || isLoadingMore}
                    onClick={() => void nextPage()}
                  >
                    {isLoadingMore ? "加载中..." : "下一页"}
                  </button>
                  <button
                    disabled={hasMore || safePage >= pageCount}
                    onClick={() => setPage(pageCount)}
                  >
                    末页
                  </button>
                </div>
              </div>
            </td>
          </tr>
        </tfoot>
      ) : null}
    </>
  );
}

export function Field({ children, label }: { children: ReactNode; label: string }) {
  return (
    <label className="field">
      <span>{label}</span>
      {children}
    </label>
  );
}

export function MonthSelect({
  compact = false,
  disabled = false,
  onChange,
  value,
}: {
  compact?: boolean;
  disabled?: boolean;
  onChange: (value: string) => void | Promise<void>;
  value: string;
}) {
  const now = new Date();
  const currentMonth = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}`;
  const safeValue = /^\d{4}-\d{2}$/.test(value) ? value : currentMonth;
  const selectedYear = Number(safeValue.slice(0, 4));
  const selectedMonth = safeValue.slice(5, 7);
  const currentYear = now.getFullYear();
  const years = Array.from({ length: 11 }, (_, index) => currentYear - 5 + index);

  function emitChange(year: number, month: string) {
    void onChange(`${year}-${month}`);
  }

  return (
    <div className={compact ? "month-select compact" : "month-select"}>
      <select
        aria-label="年份"
        disabled={disabled}
        value={selectedYear}
        onChange={(event) => emitChange(Number(event.target.value), selectedMonth)}
      >
        {years.map((year) => (
          <option key={year} value={year}>
            {year}年
          </option>
        ))}
      </select>
      <select
        aria-label="月份"
        disabled={disabled}
        value={selectedMonth}
        onChange={(event) => emitChange(selectedYear, event.target.value)}
      >
        {Array.from({ length: 12 }, (_, index) => {
          const month = String(index + 1).padStart(2, "0");
          return (
            <option key={month} value={month}>
              {month}月
            </option>
          );
        })}
      </select>
    </div>
  );
}

export function PathPickerField({
  buttonLabel,
  disabled,
  onChoose,
  placeholder,
  value,
}: {
  buttonLabel: string;
  disabled?: boolean;
  onChoose: () => void | Promise<void>;
  placeholder: string;
  value: string;
}) {
  return (
    <div className="path-picker-field">
      <input readOnly disabled={disabled} value={value} placeholder={placeholder} />
      <button
        className="ghost-button"
        disabled={disabled}
        onClick={onChoose}
        type="button"
      >
        {buttonLabel}
      </button>
    </div>
  );
}

export function Status({ enabled }: { enabled: boolean }) {
  return (
    <span className={enabled ? "status enabled" : "status disabled"}>
      {enabled ? "启用" : "停用"}
    </span>
  );
}

export function EmptyRow({ colSpan }: { colSpan: number }) {
  return (
    <tr>
      <td className="empty-cell" colSpan={colSpan}>
        暂无数据
      </td>
    </tr>
  );
}
