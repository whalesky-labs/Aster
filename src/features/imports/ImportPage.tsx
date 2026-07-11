import { useState } from "react";
import type { ImportPreview, ImportResult } from "../../entities/imports";
import { EmptyRow, PaginatedTable } from "../../shared/ui/DataTable";

type ImportMode = "full" | "itemsOnly";

export function ImportPage({ canPreviewImport, canRunImport, formatMoney, isWorking, onExportTemplate, onPreview, onRun, onSelectFile, preview, result }: {
  canPreviewImport: boolean; canRunImport: boolean; formatMoney: (value: number) => string; isWorking: boolean;
  onExportTemplate: () => Promise<void>; onPreview: (path: string) => Promise<void>; onRun: (path: string, mode: ImportMode) => Promise<void>;
  onSelectFile: () => Promise<string | null>; preview: ImportPreview | null; result: ImportResult | null;
}) {
  const [path, setPath] = useState(""); const [mode, setMode] = useState<ImportMode>("full");
  async function selectFile() { const selected = await onSelectFile(); if (selected) setPath(selected); }
  const metrics = [
    { label: "工作表", value: preview?.sheetCount ?? 0, suffix: "张" }, { label: "物品", value: preview?.itemCount ?? 0, suffix: "种" },
    { label: "待新增物品", value: preview?.newItemCount ?? 0, suffix: "种" }, { label: "预计单据", value: preview?.documentCount ?? 0, suffix: "张" },
    { label: "期初金额", value: formatMoney(preview?.openingAmount ?? 0), suffix: "元" }, { label: "入库金额", value: formatMoney(preview?.inboundAmount ?? 0), suffix: "元" },
    { label: "领用金额", value: formatMoney(preview?.outboundAmount ?? 0), suffix: "元" }, { label: "错误", value: preview?.errors.length ?? 0, suffix: "条" },
  ];
  return <section className="import-layout">
    <div className="module-panel import-toolbar"><div><p>请使用新版三表模板：物品档案、入库明细、出库明细。预览不会写入数据库；确认导入后按真实单据生成批次、流水、库存和销售成本。</p></div><div className="import-path-row"><input readOnly value={path} placeholder="请选择 .xlsx 文件" /><button className="ghost-button" disabled={isWorking || !canPreviewImport} onClick={onExportTemplate}>生成导入模板</button><button className="ghost-button" disabled={isWorking || !canPreviewImport} onClick={selectFile}>选择 Excel</button><select value={mode} onChange={(event) => setMode(event.target.value as ImportMode)}><option value="full">完整导入</option><option value="itemsOnly">只导入物品档案</option></select><button className="ghost-button" disabled={isWorking || !canPreviewImport} onClick={() => onPreview(path)}>预览</button><button className="primary-button" disabled={isWorking || !preview || preview.errors.length > 0 || !canRunImport} onClick={() => onRun(path, mode)}>执行导入</button></div></div>
    <section className="metrics-grid">{metrics.map((card) => <div className="metric-card" key={card.label}><span>{card.label}</span><strong>{card.value}</strong><em>{card.suffix}</em></div>)}</section>
    {result ? <ImportResultPanel result={result} /> : null}
    <div className="workspace-grid"><ImportMonths preview={preview} formatMoney={formatMoney} /><ImportMessages preview={preview} /></div>
    <ImportItems preview={preview} formatMoney={formatMoney} />
  </section>;
}

function ImportResultPanel({ result }: { result: ImportResult }) {
  return <div className="module-panel import-result"><h2>导入结果</h2><div className="result-grid"><span>任务 ID</span><strong>{result.jobId}</strong><span>新增物品</span><strong>{result.importedItems} 种</strong><span>匹配物品</span><strong>{result.matchedItems} 种</strong><span>生成单据</span><strong>{result.documentCount} 张</strong><span>生成流水</span><strong>{result.movementCount} 条</strong><span>导入报告</span><strong>{result.reportPath ?? "-"}</strong><span>源文件备份</span><strong>{result.sourceCopyPath ?? "-"}</strong></div></div>;
}

function ImportMonths({ preview, formatMoney }: { preview: ImportPreview | null; formatMoney: (value: number) => string }) {
  return <div className="table-panel report-section"><div className="table-toolbar"><h2>月份识别</h2></div><table><thead><tr><th>月份</th><th>行数</th><th>期初数量</th><th>入库数量</th><th>领用数量</th><th>领用金额</th></tr></thead><tbody>{(preview?.months ?? []).map((row) => <tr key={row.month}><td>{row.month}</td><td>{row.rowCount}</td><td>{row.openingQuantity}</td><td>{row.inboundQuantity}</td><td>{row.outboundQuantity}</td><td>{formatMoney(row.outboundAmount)}</td></tr>)}{!preview || preview.months.length === 0 ? <EmptyRow colSpan={6} /> : null}</tbody></table></div>;
}

function ImportMessages({ preview }: { preview: ImportPreview | null }) {
  const rows = [...(preview?.errors ?? []), ...(preview?.warnings ?? [])];
  return <div className="table-panel report-section"><div className="table-toolbar"><h2>校验信息</h2></div><table><thead><tr><th>级别</th><th>位置</th><th>说明</th></tr></thead><PaginatedTable colSpan={3} getRowKey={(message, index) => `${message.sheet}-${message.row}-${index}`} rows={rows}>{(message) => <><td><span className={`status ${message.level === "error" ? "disabled" : "enabled"}`}>{message.level === "error" ? "错误" : "提醒"}</span></td><td>{message.sheet} 第 {message.row} 行{message.column ? ` ${message.column}列` : ""}</td><td>{message.message}</td></>}</PaginatedTable></table></div>;
}

function ImportItems({ preview, formatMoney }: { preview: ImportPreview | null; formatMoney: (value: number) => string }) {
  return <div className="table-panel report-section"><div className="table-toolbar"><h2>物品预览</h2><span className="table-note">最多按当前解析结果展示全部物品</span></div><table><thead><tr><th>物品</th><th>分类</th><th>规格</th><th>单位</th><th>默认价</th><th>期初</th><th>入库</th><th>领用</th><th>匹配</th></tr></thead><PaginatedTable colSpan={9} getRowKey={(item) => item.name} rows={preview?.items ?? []}>{(item) => <><td>{item.name}</td><td>{item.categoryName ?? "-"}</td><td>{item.spec ?? "-"}</td><td>{item.unitName ?? "-"}</td><td>{formatMoney(item.defaultPrice)}</td><td>{item.openingQuantity}</td><td>{item.inboundQuantity}</td><td>{item.outboundQuantity}</td><td><span className={item.existing ? "status enabled" : "status disabled"}>{item.existing ? "已有" : "新增"}</span></td></>}</PaginatedTable></table></div>;
}
