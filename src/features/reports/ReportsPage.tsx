import { type KeyboardEvent, useEffect, useState } from "react";
import type { Category, Department, Item, Supplier } from "../../entities/master-data";
import type { ReportBundle, ReportQuery } from "../../entities/reports";
import { ItemSearchSelect } from "../../shared/ui/ItemSearchSelect";
import { Field, MonthSelect } from "../../shared/ui/DataTable";
import {
  BarChartPanel,
  ReportGroupHeader,
  ReportInsightCard,
  ReportInsightGrid,
} from "./ReportComponents";

function currentMonthString() {
  return new Date().toISOString().slice(0, 7);
}

function formatMoney(value: number) {
  return new Intl.NumberFormat("zh-CN", { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(value);
}

function submitOnEnter(event: KeyboardEvent<HTMLDivElement>, onSubmit: () => void) {
  if (event.key !== "Enter") return;
  const target = event.target as HTMLElement;
  if (target.tagName === "TEXTAREA") return;
  event.preventDefault();
  onSubmit();
}

export function ReportsPage({
  bundle,
  categories,
  canViewReports,
  departments,
  exportPath,
  items,
  onExport,
  onQueryChange,
  query,
  suppliers,
}: {
  bundle: ReportBundle | null;
  categories: Category[];
  canViewReports: boolean;
  departments: Department[];
  exportPath: string | null;
  items: Item[];
  onExport: (query: ReportQuery) => Promise<void>;
  onQueryChange: (query: ReportQuery) => Promise<void>;
  query: ReportQuery;
  suppliers: Supplier[];
}) {
  const [filterDraft, setFilterDraft] = useState<ReportQuery>(query);
  useEffect(() => setFilterDraft(query), [query]);

  const inventory = bundle?.monthlyInventory ?? [];
  const summary = bundle?.departmentSummary ?? [];
  const categoryConsumption = bundle?.categoryConsumption ?? [];
  const itemRanking = bundle?.itemConsumptionRanking ?? [];
  const inboundDetails = bundle?.inboundDetails ?? [];
  const outboundDetails = bundle?.outboundDetails ?? [];
  const salesProfit = bundle?.salesProfit ?? [];
  const stockBalances = bundle?.stockBalances ?? [];
  const stockWarnings = bundle?.stockWarnings ?? [];
  const stocktakeDifferences = bundle?.stocktakeDifferences ?? [];
  const totalInbound = inventory.reduce(
    (sum, row) => sum + row.inboundAmount,
    0,
  );
  const totalOutbound = inventory.reduce(
    (sum, row) => sum + row.outboundAmount,
    0,
  );
  const totalSales = salesProfit.reduce((sum, row) => sum + row.saleAmount, 0);
  const totalSalesCost = salesProfit.reduce(
    (sum, row) => sum + row.costAmount,
    0,
  );
  const totalGrossProfit = salesProfit.reduce(
    (sum, row) => sum + row.grossProfit,
    0,
  );
  const grossMargin = totalSales > 0 ? totalGrossProfit / totalSales : null;
  const negativeProfitCount = salesProfit.filter(
    (row) => row.negativeProfit,
  ).length;
  const reportRangeLabel = `${filterDraft.month || currentMonthString()} 月`;

  function printReport() {
    window.print();
  }

  function updateFilter(next: Partial<ReportQuery>) {
    setFilterDraft({ ...filterDraft, ...next });
  }

  function applyFilters() {
    applyFiltersWithDraft(filterDraft);
  }

  function applyFiltersWithDraft(draft: ReportQuery) {
    onQueryChange({
      ...draft,
      startDate: null,
      endDate: null,
    });
  }

  function updateItemFilter(itemId: string) {
    const nextDraft = { ...filterDraft, itemId };
    setFilterDraft(nextDraft);
    applyFiltersWithDraft(nextDraft);
  }

  function resetFilters() {
    const nextQuery = {
      month: filterDraft.month || currentMonthString(),
      startDate: null,
      endDate: null,
    };
    setFilterDraft(nextQuery);
    onQueryChange(nextQuery);
  }

  return (
    <section className="report-layout">
      <div className="module-panel report-command-center">
        <div className="report-command-main">
          <div>
            <span className="report-kicker">报表中心</span>
            <h2>{reportRangeLabel}经营报表</h2>
            <p>
              入库 {formatMoney(totalInbound)} 元 · 成本出库{" "}
              {formatMoney(totalOutbound)} 元 · 销售 {formatMoney(totalSales)} 元 ·
              毛利 {formatMoney(totalGrossProfit)} 元
            </p>
          </div>
          <div className="report-actions">
            <button
              className="primary-button"
              disabled={!canViewReports}
              onClick={() => onExport(query)}
            >
              导出 Excel
            </button>
            <button
              className="ghost-button"
              disabled={!canViewReports || !bundle}
              onClick={printReport}
            >
              打印
            </button>
          </div>
        </div>
        <div
          className="report-filters"
          onKeyDown={(event) => submitOnEnter(event, applyFilters)}
        >
          <div className="filter-fields">
            <Field label="月份">
              <MonthSelect
                disabled={!canViewReports}
                value={filterDraft.month}
                onChange={(month) => updateFilter({ month })}
              />
            </Field>
            <Field label="部门">
              <select
                disabled={!canViewReports}
                value={filterDraft.departmentId ?? ""}
                onChange={(e) => updateFilter({ departmentId: e.target.value })}
              >
                <option value="">全部</option>
                {departments.map((item) => (
                  <option key={item.id} value={item.id}>
                    {item.name}
                  </option>
                ))}
              </select>
            </Field>
            <Field label="分类">
              <select
                disabled={!canViewReports}
                value={filterDraft.categoryId ?? ""}
                onChange={(e) => updateFilter({ categoryId: e.target.value })}
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
                disabled={!canViewReports}
                emptyLabel="全部物品"
                items={items}
                value={filterDraft.itemId ?? ""}
                onChange={(itemId) => updateFilter({ itemId })}
                onCommit={updateItemFilter}
              />
            </Field>
            <Field label="供应商">
              <select
                disabled={!canViewReports}
                value={filterDraft.supplierId ?? ""}
                onChange={(e) => updateFilter({ supplierId: e.target.value })}
              >
                <option value="">全部</option>
                {suppliers.map((item) => (
                  <option key={item.id} value={item.id}>
                    {item.name}
                  </option>
                ))}
              </select>
            </Field>
          </div>
          <div className="filter-actions report-filter-actions">
            <button
              className="ghost-button"
              disabled={!canViewReports}
              onClick={resetFilters}
            >
              清空
            </button>
            <button
              className="primary-button"
              disabled={!canViewReports || !filterDraft.month}
              onClick={applyFilters}
            >
              查询
            </button>
          </div>
        </div>
        {exportPath ? (
          <div className="export-path">已导出：{exportPath}</div>
        ) : null}
      </div>

      <section className="report-metrics-grid">
        <div className="metric-card">
          <span>本月入库金额</span>
          <strong>{formatMoney(totalInbound)}</strong>
          <em>元</em>
        </div>
        <div className="metric-card">
          <span>本月出库成本</span>
          <strong>{formatMoney(totalOutbound)}</strong>
          <em>元</em>
        </div>
        <div className="metric-card">
          <span>客销收入</span>
          <strong>{formatMoney(totalSales)}</strong>
          <em>元</em>
        </div>
        <div className="metric-card">
          <span>销售毛利</span>
          <strong>{formatMoney(totalGrossProfit)}</strong>
          <em>{grossMargin === null ? "无销售" : `${(grossMargin * 100).toFixed(1)}%`}</em>
        </div>
        <div className="metric-card">
          <span>库存预警</span>
          <strong>{stockWarnings.length}</strong>
          <em>项</em>
        </div>
        <div className="metric-card">
          <span>盘点差异</span>
          <strong>{stocktakeDifferences.length}</strong>
          <em>行</em>
        </div>
      </section>

      <ReportGroupHeader
        title="经营分析"
        description="按部门、分类、物品和库存预警查看核心变化。"
      />
      <div className="workspace-grid">
        <BarChartPanel
          rows={summary
            .filter((row) => row.amount > 0)
            .map((row) => ({ label: row.departmentName, value: row.amount }))}
          title="部门领用金额"
        />
        <BarChartPanel
          rows={categoryConsumption.map((row) => ({
            label: row.categoryName,
            value: row.amount,
          }))}
          title="分类消耗金额"
        />
      </div>

      <ReportGroupHeader
        title="销售毛利"
        description="酒店客人销售按销售收入、FIFO 成本和毛利单独核算。"
      />
      <ReportInsightGrid>
        <ReportInsightCard
          label="销售收入"
          value={`${formatMoney(totalSales)} 元`}
          detail={`${salesProfit.length} 行客销明细，完整记录随 Excel 导出`}
        />
        <ReportInsightCard
          label="销售成本"
          value={`${formatMoney(totalSalesCost)} 元`}
          detail="成本来自对应出库批次的实际采购成本"
        />
        <ReportInsightCard
          label="毛利率"
          value={grossMargin === null ? "暂无" : `${(grossMargin * 100).toFixed(1)}%`}
          detail={
            negativeProfitCount > 0
              ? `${negativeProfitCount} 行销售低于成本`
              : "暂无亏损销售"
          }
        />
      </ReportInsightGrid>
      <div className="workspace-grid">
        <BarChartPanel
          rows={salesProfit
            .slice(0, 8)
            .map((row) => ({ label: row.itemName, value: row.grossProfit }))}
          title="销售毛利排行"
        />
        <BarChartPanel
          rows={salesProfit
            .filter((row) => row.negativeProfit)
            .slice(0, 8)
            .map((row) => ({ label: row.itemName, value: row.grossProfit }))}
          title="亏损销售提醒"
        />
      </div>

      <div className="workspace-grid">
        <BarChartPanel
          rows={itemRanking
            .slice(0, 8)
            .map((row) => ({ label: row.itemName, value: row.amount }))}
          title="物品消耗排行"
        />
        <BarChartPanel
          rows={stockWarnings
            .slice(0, 8)
            .map((row) => ({
              label: row.itemName,
              value: row.shortageQuantity,
            }))}
          title="库存预警缺口"
          valueFormatter={(value) => value.toFixed(2)}
        />
      </div>

      <ReportGroupHeader
        title="库存总览"
        description="保留库存规模、结存金额和预警概览，明细数据通过导出查看。"
      />
      <ReportInsightGrid>
        <ReportInsightCard
          label="统计物品"
          value={`${inventory.length} 项`}
          detail={`当前余额覆盖 ${stockBalances.length} 项物品`}
        />
        <ReportInsightCard
          label="结存金额"
          value={`${formatMoney(
            inventory.reduce((sum, row) => sum + row.endingAmount, 0),
          )} 元`}
          detail="按所选期间的进销存汇总计算"
        />
        <ReportInsightCard
          label="库存风险"
          value={`${stockWarnings.length} 项`}
          detail="低库存、负库存和预警缺口会进入导出明细"
        />
      </ReportInsightGrid>

      <ReportGroupHeader
        title="领用分析"
        description="突出领用结构和消耗集中度，不在页面铺开明细清单。"
      />
      <ReportInsightGrid>
        <ReportInsightCard
          label="参与部门"
          value={`${summary.filter((row) => row.amount > 0).length} 个`}
          detail="按部门图表查看主要消耗来源"
        />
        <ReportInsightCard
          label="最高消耗部门"
          value={summary[0]?.departmentName ?? "暂无"}
          detail={
            summary[0] ? `${formatMoney(summary[0].amount)} 元` : "暂无领用记录"
          }
        />
        <ReportInsightCard
          label="最高消耗物品"
          value={itemRanking[0]?.itemName ?? "暂无"}
          detail={
            itemRanking[0]
              ? `${formatMoney(itemRanking[0].amount)} 元`
              : "暂无物品消耗"
          }
        />
      </ReportInsightGrid>

      <ReportGroupHeader
        title="流水概览"
        description="页面只展示流转规模，具体入库和出库明细请使用导出或库存流水。"
      />
      <ReportInsightGrid>
        <ReportInsightCard
          label="入库明细"
          value={`${inboundDetails.length} 行`}
          detail={`${formatMoney(totalInbound)} 元，完整明细随 Excel 导出`}
        />
        <ReportInsightCard
          label="出库明细"
          value={`${outboundDetails.length} 行`}
          detail={`${formatMoney(totalOutbound)} 元，含内部领用与客人销售`}
        />
        <ReportInsightCard
          label="导出范围"
          value="完整报表"
          detail="导出文件保留全部明细表和追溯字段"
        />
      </ReportInsightGrid>

      <ReportGroupHeader
        title="库存风险"
        description="只保留风险摘要，风险明细从导出表继续追溯。"
      />
      <ReportInsightGrid>
        <ReportInsightCard
          label="预警物品"
          value={`${stockWarnings.length} 项`}
          detail={`合计缺口 ${stockWarnings
            .reduce((sum, row) => sum + row.shortageQuantity, 0)
            .toFixed(2)}`}
        />
        <ReportInsightCard
          label="盘点差异"
          value={`${stocktakeDifferences.length} 行`}
          detail={`${formatMoney(
            stocktakeDifferences.reduce(
              (sum, row) => sum + Math.abs(row.differenceAmount),
              0,
            ),
          )} 元差异金额`}
        />
        <ReportInsightCard
          label="风险处理"
          value={stockWarnings.length || stocktakeDifferences.length ? "需关注" : "正常"}
          detail="库存预警和盘点差异建议在导出表中逐项核对"
        />
      </ReportInsightGrid>
    </section>
  );
}
