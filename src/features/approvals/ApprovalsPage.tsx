import { useMemo, useState } from "react";
import type { ApprovalRequest } from "../../entities/approvals";
import { matchesSearchText } from "../../shared/lib/display";
import { EmptyRow, Field, TableFeatureToolbar, TableSearchToolbar } from "../../shared/ui/DataTable";

export function ApprovalsPage({ approvals, canManage, formatDateTime, statusLabel, typeLabel, onDecide }: {
  approvals: ApprovalRequest[]; canManage: boolean; formatDateTime: (value?: string | null) => string; statusLabel: (value: string) => string; typeLabel: (value: string) => string;
  onDecide: (approvalId: string, approve: boolean, decisionNote: string) => Promise<void>;
}) {
  const [decisionNote, setDecisionNote] = useState(""); const [search, setSearch] = useState("");
  const rows = useMemo(() => approvals.filter((item) => matchesSearchText(search, [item.id, typeLabel(item.entityType), item.entityId, item.reason, statusLabel(item.status), item.requestedBy, item.decidedBy, item.decisionNote])), [approvals, search, statusLabel, typeLabel]);
  return <section className="table-panel"><TableFeatureToolbar><Field label="审批意见"><input value={decisionNote} onChange={(event) => setDecisionNote(event.target.value)} placeholder="通过/驳回说明" /></Field></TableFeatureToolbar><TableSearchToolbar onSearchChange={setSearch} placeholder="搜索类型、对象、原因、状态、申请人" search={search} /><table><thead><tr><th>ID</th><th>类型</th><th>对象</th><th>原因</th><th>状态</th><th>申请时间</th><th>审批时间</th><th>操作</th></tr></thead><tbody>{rows.map((item) => <tr key={item.id}><td className="path-cell">{item.id}</td><td>{typeLabel(item.entityType)}</td><td>{item.entityId}</td><td>{item.reason ?? "-"}</td><td><span className={item.status === "approved" ? "status enabled" : item.status === "pending" ? "status" : "status disabled"}>{statusLabel(item.status)}</span></td><td>{formatDateTime(item.createdAt)}</td><td>{formatDateTime(item.decidedAt)}</td><td className="row-actions">{item.status === "pending" ? <><button disabled={!canManage} onClick={() => onDecide(item.id, true, decisionNote)}>通过</button><button disabled={!canManage} onClick={() => onDecide(item.id, false, decisionNote)}>驳回</button></> : "-"}</td></tr>)}{rows.length === 0 ? <EmptyRow colSpan={8} /> : null}</tbody></table></section>;
}
