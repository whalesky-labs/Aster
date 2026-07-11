import { useState } from "react";
import type { Item } from "../../entities/master-data";
import type { StockDocument, StockDocumentQuery } from "../../entities/stock";
import { openEditorWindow } from "../../shared/lib/editorWindows";
import { DocumentList, DocumentVoidControls } from "./StockDocumentPage";

export function AdjustmentPage({
  canWrite, documents, handlerOptions, items, onQueryChange, onVoid, query,
}: {
  canWrite: boolean;
  documents: StockDocument[];
  handlerOptions: string[];
  items: Item[];
  onQueryChange: (query: StockDocumentQuery) => Promise<void>;
  onVoid: (documentId: string, reason: string, handler: string) => Promise<void>;
  query: StockDocumentQuery;
}) {
  const [voidReason, setVoidReason] = useState("");
  const [voidHandler, setVoidHandler] = useState("");
  return (
    <section className="table-panel">
      <div className="table-toolbar document-action-toolbar">
        <DocumentVoidControls
          approvalRequestId="" isOutbound={false} setApprovalRequestId={() => undefined}
          setVoidHandler={setVoidHandler} setVoidReason={setVoidReason}
          voidHandler={voidHandler} voidReason={voidReason}
        />
        <button className="primary-button" disabled={!canWrite} onClick={() => openEditorWindow("adjustment", { width: 980, height: 760 })}>
          新建调整单
        </button>
      </div>
      <DocumentList
        canVoid={canWrite} documents={documents} handlerOptions={handlerOptions}
        items={items} isOutbound={false} onQueryChange={onQueryChange} onVoid={onVoid}
        query={query} voidHandler={voidHandler} voidReason={voidReason}
      />
    </section>
  );
}
