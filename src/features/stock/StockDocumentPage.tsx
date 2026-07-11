import { type KeyboardEvent, useEffect, useMemo, useState } from "react";
import type { Department, Item, Supplier } from "../../entities/master-data";
import type { StockDocument, StockDocumentQuery } from "../../entities/stock";
import { createI18n } from "../../i18n";
import { openEditorWindow } from "../../shared/lib/editorWindows";
import { Field, MonthSelect, PaginatedTable } from "../../shared/ui/DataTable";
import { ItemSearchSelect } from "../../shared/ui/ItemSearchSelect";

const i18n = createI18n("zh-CN");
const currentMonthString = () => new Date().toISOString().slice(0, 7);
const formatMoney = (value: number) => i18n.formatMoney(value);
function formatDateTime(value?: string | null) {
  if (!value) return "-";
  const normalized = value.trim().replace("T", " ").replace(/Z$/, "");
  const [date, time] = normalized.split(/\s+/, 2);
  return time ? `${date} ${time.split(/[.+-]/)[0]}` : date;
}
function uniqueTextOptions(values: Array<string | null | undefined>) {
  return [...new Set(values.map((value) => String(value ?? "").trim()).filter(Boolean))];
}
function submitOnEnter(event: KeyboardEvent<HTMLDivElement>, onSubmit: () => void) {
  if (event.key !== "Enter" || (event.target as HTMLElement).tagName === "TEXTAREA") return;
  event.preventDefault();
  onSubmit();
}
function outboundKindLabel(kind?: "internal" | "guest_sale" | null) {
  return kind === "guest_sale" ? "酒店客人销售" : "内部员工领用";
}
export function StockDocumentPage({
  canWrite,
  departments,
  documentType,
  documents,
  handlerOptions,
  items,
  onConfirmDraft,
  onQueryChange,
  onVoid,
  query,
  suppliers,
}: {
  canWrite: boolean;
  departments: Department[];
  documentType: "inbound" | "outbound";
  documents: StockDocument[];
  handlerOptions: string[];
  items: Item[];
  onConfirmDraft: (
    documentId: string,
    approvalRequestId?: string | null,
  ) => Promise<void>;
  onQueryChange: (query: StockDocumentQuery) => Promise<void>;
  onVoid: (
    documentId: string,
    reason: string,
    handler: string,
  ) => Promise<void>;
  query: StockDocumentQuery;
  suppliers: Supplier[];
}) {
  const isOutbound = documentType === "outbound";
  const [approvalRequestId, setApprovalRequestId] = useState("");
  const [voidReason, setVoidReason] = useState("");
  const [voidHandler, setVoidHandler] = useState("");

  return (
    <section className="table-panel">
      <div className="table-toolbar document-action-toolbar">
        <DocumentVoidControls
          approvalRequestId={approvalRequestId}
          isOutbound={isOutbound}
          setApprovalRequestId={setApprovalRequestId}
          setVoidHandler={setVoidHandler}
          setVoidReason={setVoidReason}
          voidHandler={voidHandler}
          voidReason={voidReason}
        />
        <button
          className="primary-button"
          disabled={!canWrite}
          onClick={() =>
            openEditorWindow("stockDocument", {
              documentType,
              width: 980,
              height: 760,
            })
          }
        >
          {isOutbound ? "新建出库/领用单" : "新建入库单"}
        </button>
      </div>
      <DocumentList
        departments={departments}
        documents={documents}
        handlerOptions={handlerOptions}
        items={items}
        isOutbound={isOutbound}
        canVoid={canWrite}
        approvalRequestId={approvalRequestId}
        onConfirmDraft={onConfirmDraft}
        onQueryChange={onQueryChange}
        onVoid={onVoid}
        query={query}
        voidHandler={voidHandler}
        voidReason={voidReason}
        suppliers={suppliers}
      />
    </section>
  );
}
export function DocumentVoidControls({
  approvalRequestId,
  isOutbound,
  setApprovalRequestId,
  setVoidHandler,
  setVoidReason,
  voidHandler,
  voidReason,
}: {
  approvalRequestId: string;
  isOutbound: boolean;
  setApprovalRequestId: (value: string) => void;
  setVoidHandler: (value: string) => void;
  setVoidReason: (value: string) => void;
  voidHandler: string;
  voidReason: string;
}) {
  return (
    <div className="void-controls">
      {isOutbound ? (
        <input
          placeholder="审批单 ID"
          value={approvalRequestId}
          onChange={(e) => setApprovalRequestId(e.target.value)}
        />
      ) : null}
      <input
        placeholder="作废原因"
        value={voidReason}
        onChange={(e) => setVoidReason(e.target.value)}
      />
      <input
        placeholder="经办人"
        value={voidHandler}
        onChange={(e) => setVoidHandler(e.target.value)}
      />
    </div>
  );
}

export function DocumentList({
  approvalRequestId = "",
  canVoid = true,
  departments = [],
  documents,
  handlerOptions = [],
  items = [],
  isOutbound,
  onConfirmDraft,
  onQueryChange,
  onVoid,
  query,
  suppliers = [],
  title,
  voidHandler = "",
  voidReason = "",
}: {
  approvalRequestId?: string;
  canVoid?: boolean;
  departments?: Department[];
  documents: StockDocument[];
  handlerOptions?: string[];
  items?: Item[];
  isOutbound: boolean;
  onConfirmDraft?: (
    documentId: string,
    approvalRequestId?: string | null,
  ) => Promise<void>;
  onQueryChange?: (query: StockDocumentQuery) => Promise<void>;
  onVoid?: (
    documentId: string,
    reason: string,
    handler: string,
  ) => Promise<void>;
  query?: StockDocumentQuery;
  suppliers?: Supplier[];
  title?: string;
  voidHandler?: string;
  voidReason?: string;
}) {
  const [filterDraft, setFilterDraft] = useState<StockDocumentQuery>(
    query ?? {
      documentType: isOutbound ? "outbound" : "inbound",
      month: currentMonthString(),
    },
  );
  useEffect(() => {
    if (query) {
      setFilterDraft({
        ...query,
        month: query.month || currentMonthString(),
      });
    }
  }, [query]);

  const partyLabel = isOutbound ? "领用部门" : "供应商";
  const isAdjustment = filterDraft.documentType === "adjustment";
  const partyValue = isOutbound
    ? (filterDraft.departmentId ?? "")
    : (filterDraft.supplierId ?? "");
  const partyOptions = isOutbound ? departments : suppliers;
  const effectiveHandlerOptions = useMemo(
    () =>
      uniqueTextOptions([
        ...handlerOptions,
        ...documents.map((document) => document.handler),
      ]),
    [documents, handlerOptions],
  );

  function updateFilter(next: Partial<StockDocumentQuery>) {
    setFilterDraft({ ...filterDraft, ...next });
  }

  function applyFilters() {
    if (!query || !onQueryChange) return;
    applyFiltersWithDraft(filterDraft);
  }

  function applyFiltersWithDraft(draft: StockDocumentQuery) {
    if (!query || !onQueryChange) return;
    onQueryChange({
      ...draft,
      documentType: query.documentType,
      outboundKind: isOutbound ? draft.outboundKind || null : null,
      month: draft.month || currentMonthString(),
      departmentId: isOutbound ? draft.departmentId || null : null,
      supplierId: !isOutbound && !isAdjustment ? draft.supplierId || null : null,
      itemId: draft.itemId || null,
      handler: draft.handler || null,
      search: draft.search?.trim() || null,
    });
  }

  function updateItemFilter(itemId: string) {
    const nextDraft = { ...filterDraft, itemId };
    setFilterDraft(nextDraft);
    applyFiltersWithDraft(nextDraft);
  }

  function resetFilters() {
    if (!query || !onQueryChange) return;
    const nextQuery: StockDocumentQuery = {
      documentType: query.documentType,
      month: currentMonthString(),
    };
    setFilterDraft(nextQuery);
    onQueryChange(nextQuery);
  }

  return (
    <div className="subtable">
      {title ? (
        <div className="subtable-heading">
          <h3>{title}</h3>
        </div>
      ) : null}
      {query && onQueryChange ? (
        <div
          className="document-filters"
          onKeyDown={(event) => submitOnEnter(event, applyFilters)}
        >
          <div className="filter-fields">
            <Field label="月份">
              <MonthSelect
                value={filterDraft.month ?? currentMonthString()}
                onChange={(month) => updateFilter({ month })}
              />
            </Field>
            {isAdjustment ? null : (
              <Field label={partyLabel}>
                <select
                  value={partyValue}
                  onChange={(e) =>
                    isOutbound
                      ? updateFilter({ departmentId: e.target.value })
                      : updateFilter({ supplierId: e.target.value })
                  }
                >
                  <option value="">全部</option>
                  {partyOptions.map((item) => (
                    <option key={item.id} value={item.id}>
                      {item.name}
                    </option>
                  ))}
                </select>
              </Field>
            )}
            {isAdjustment ? null : isOutbound ? (
              <Field label="出库类型">
                <select
                  value={filterDraft.outboundKind ?? ""}
                  onChange={(e) =>
                    updateFilter({
                      outboundKind:
                        e.target.value === ""
                          ? null
                          : (e.target.value as "internal" | "guest_sale"),
                    })
                  }
                >
                  <option value="">全部</option>
                  <option value="internal">内部员工领用</option>
                  <option value="guest_sale">酒店客人销售</option>
                </select>
              </Field>
            ) : null}
            <Field label="经办人">
              <select
                value={filterDraft.handler ?? ""}
                onChange={(e) => updateFilter({ handler: e.target.value })}
              >
                <option value="">全部</option>
                {effectiveHandlerOptions.map((handler) => (
                  <option key={handler} value={handler}>
                    {handler}
                  </option>
                ))}
              </select>
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
            <Field label="关键字">
              <input
                placeholder="单号/物品/备注"
                value={filterDraft.search ?? ""}
                onChange={(e) => updateFilter({ search: e.target.value })}
              />
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
      ) : null}
      <table>
        <thead>
          <tr>
            <th>单号</th>
            <th>日期</th>
            {isOutbound ? <th>类型</th> : null}
            <th>对象</th>
            <th>物品</th>
            <th>审批单</th>
            <th>数量</th>
            <th>金额</th>
            <th>状态</th>
            <th>操作</th>
          </tr>
        </thead>
        <PaginatedTable
          colSpan={isOutbound ? 10 : 9}
          getRowKey={(doc) => doc.id}
          rows={documents}
        >
          {(doc) => (
            <>
              <td>{doc.documentNo}</td>
              <td>{formatDateTime(doc.businessDate)}</td>
              {isOutbound ? (
                <td>{outboundKindLabel(doc.outboundKind)}</td>
              ) : null}
              <td>
                {doc.documentType === "outbound"
                  ? doc.outboundKind === "guest_sale"
                    ? "酒店客人"
                    : (doc.departmentName ?? "-")
                  : (doc.supplierName ?? "-")}
              </td>
              <td className="item-summary-cell" title={doc.itemSummary ?? ""}>
                {doc.itemSummary ?? "-"}
              </td>
              <td>{doc.approvalRequestId ?? "-"}</td>
              <td>{doc.totalQuantity}</td>
              <td>{formatMoney(doc.totalAmount)}</td>
              <td>
                <span
                  className={
                    doc.status === "voided"
                      ? "status disabled"
                      : "status enabled"
                  }
                >
                  {doc.status === "confirmed"
                    ? "已确认"
                    : doc.status === "voided"
                      ? "已作废"
                      : doc.status === "draft"
                        ? "草稿"
                        : doc.status}
                </span>
              </td>
              <td className="row-actions">
                <button
                  onClick={() =>
                    openEditorWindow("stockDocumentDetail", {
                      documentType: doc.documentType as "inbound" | "outbound",
                      id: doc.id,
                      mode: "edit",
                      width: 980,
                      height: 680,
                    })
                  }
                >
                  详情
                </button>
                {doc.status === "draft" && onConfirmDraft ? (
                  <button
                    disabled={!canVoid}
                    onClick={() =>
                      onConfirmDraft(doc.id, approvalRequestId || null)
                    }
                  >
                    确认草稿
                  </button>
                ) : null}
                {onVoid && doc.status === "confirmed" ? (
                  <button
                    disabled={!canVoid}
                    onClick={() => onVoid(doc.id, voidReason, voidHandler)}
                  >
                    作废
                  </button>
                ) : null}
                {doc.status !== "draft" && doc.status !== "confirmed"
                  ? "-"
                  : null}
              </td>
            </>
          )}
        </PaginatedTable>
      </table>
    </div>
  );
}
