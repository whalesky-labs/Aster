import { invoke } from "@tauri-apps/api/core";
import type { StocktakeDetail } from "../../entities/stock";
import type { EditorKind } from "../../shared/lib/editorWindows";
import { formatError } from "../../shared/lib/appRuntime";
import { StockDocumentDetailViewer, StockBatchDetailViewer } from "../stock/StockDetailViewers";
import { StocktakeDetailViewer } from "../stock/StocktakeViews";
import { StockDocumentEditor } from "./StockDocumentEditor";
import { AdjustmentEditor } from "./AdjustmentEditor";
import { StocktakeCreateEditor } from "./StocktakeCreateEditor";
import { StocktakeCountsEditor } from "./StocktakeCountsEditor";
import type { EditorWindowController } from "./useEditorWindowController";

export function renderStockEditorContent({
  controller, documentType, editor,
}: { controller: EditorWindowController; documentType?: "inbound" | "outbound"; editor: EditorKind }) {
  const {
    currentUser, enabledCategories, enabledDepartments, enabledItems, enabledSuppliers,
    isLoading, isSaving, runEditorAction, setError, setIsSaving, setNotice,
    setStocktakeDetail, stockBalances, stockBatches, stockDocumentDetail,
    stocktakeDetail, stocktakes,
  } = controller;
  if (editor === "stockDocument" && documentType) {
    return (
      <StockDocumentEditor
        balances={stockBalances}
        currentUser={currentUser}
        departments={enabledDepartments}
        disabled={isSaving || isLoading}
        documentType={documentType}
        items={enabledItems}
        onCreateApproval={(request) =>
          runEditorAction(
            { editor, documentType, message: "审批申请已提交" },
            () => invoke("create_approval_request", { request }),
          )
        }
        onSaveDraft={(request) =>
          runEditorAction(
            {
              editor,
              documentType,
              message:
                documentType === "outbound"
                  ? "出库/领用草稿已保存"
                  : "入库草稿已保存",
            },
            () => invoke("save_stock_document_draft", { request }),
          )
        }
        onSubmit={(request) =>
          runEditorAction(
            {
              editor,
              documentType,
              message:
                documentType === "outbound"
                  ? "出库/领用单已确认，库存已更新"
                  : "入库单已确认，库存已更新",
            },
            () => invoke("submit_stock_document", { request }),
          )
        }
        suppliers={enabledSuppliers}
      />
    );
  } else if (editor === "stockDocumentDetail") {
    return (
      <StockDocumentDetailViewer
        detail={stockDocumentDetail}
        isLoading={isLoading}
      />
    );
  } else if (editor === "stockBatchDetail") {
    return (
      <StockBatchDetailViewer batches={stockBatches} isLoading={isLoading} />
    );
  } else if (editor === "stocktakeDetail") {
    return (
      <StocktakeDetailViewer
        detail={stocktakeDetail}
        disabled={isSaving || isLoading}
        isLoading={isLoading}
        onConfirm={(stocktakeId, handler, remark) =>
          runEditorAction(
            { editor, message: "盘点单已确认，差异流水已生成", stocktakeId },
            () =>
              invoke("confirm_stocktake", {
                request: { stocktakeId, handler, remark },
              }),
          )
        }
        onExport={async (stocktakeId) => {
          try {
            setIsSaving(true);
            setError(null);
            setNotice(null);
            const result = await invoke<{ path: string }>(
              "export_stocktake_sheet",
              {
                request: { stocktakeId },
              },
            );
            setNotice(`盘点表已导出：${result.path}`);
          } catch (err) {
            setError(formatError(err));
          } finally {
            setIsSaving(false);
          }
        }}
        onVoid={(documentId, stocktakeId, reason, handler) =>
          runEditorAction(
            { editor, message: "盘点单已作废，冲正流水已生成", stocktakeId },
            () =>
              invoke("void_stock_document", {
                request: { documentId, reason, handler },
              }),
          )
        }
      />
    );
  } else if (editor === "adjustment") {
    return (
      <AdjustmentEditor
        currentUser={currentUser}
        disabled={isSaving || isLoading}
        items={enabledItems}
        onSubmit={(request) =>
          runEditorAction(
            { editor, message: "调整单已确认，库存流水已生成" },
            () => invoke("submit_adjustment", { request }),
          )
        }
      />
    );
  } else if (editor === "stocktakeCreate") {
    return (
      <StocktakeCreateEditor
        categories={enabledCategories}
        disabled={isSaving || isLoading}
        items={enabledItems}
        onCreate={(request) =>
          runEditorAction({ editor, message: "盘点单已创建" }, () =>
            invoke("create_stocktake", { request }),
          )
        }
      />
    );
  } else if (editor === "stocktakeCounts") {
    return (
      <StocktakeCountsEditor
        detail={stocktakeDetail}
        disabled={isSaving || isLoading}
        onSelect={async (stocktakeId) => {
          try {
            setError(null);
            setStocktakeDetail(
              await invoke<StocktakeDetail>("get_stocktake_detail", {
                stocktakeId,
              }),
            );
          } catch (err) {
            setError(formatError(err));
          }
        }}
        onSaveCounts={(stocktakeId, lines) =>
          runEditorAction(
            { editor, message: "实盘数量已保存", stocktakeId },
            () =>
              invoke("update_stocktake_counts", {
                request: { stocktakeId, lines },
              }),
          )
        }
        stocktakes={stocktakes}
      />
    );
  }

  return null;
}
