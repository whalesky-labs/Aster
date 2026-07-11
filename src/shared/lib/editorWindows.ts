import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

export type EditorKind =
  | "item" | "department" | "category" | "unit" | "supplier" | "budget" | "user"
  | "changePassword" | "passwordReset" | "businessSettings" | "softwareUpdate"
  | "clientConnection" | "clientPairing" | "connectionWizard" | "secondBackupDir"
  | "restoreBackup" | "stockDocument" | "stockDocumentDetail" | "stockBatchDetail"
  | "adjustment" | "stocktakeCreate" | "stocktakeDetail" | "stocktakeCounts";

export type EditorMode = "create" | "edit";

const openingEditorWindows = new Set<string>();

function editorTitle(editor: EditorKind, mode: EditorMode, documentType?: "inbound" | "outbound") {
  if (editor === "stockDocumentDetail") return documentType === "outbound" ? "出库/领用单详情" : "入库单详情";
  if (editor === "stockBatchDetail") return "批次库存";
  if (editor === "stocktakeDetail") return "盘点详情";
  if (editor === "stockDocument") return documentType === "outbound" ? "新建出库/领用单" : "新建入库单";
  const labels: Record<Exclude<EditorKind, "stockDocument" | "stockDocumentDetail" | "stockBatchDetail" | "stocktakeDetail">, string> = {
    adjustment: "库存调整", budget: "预算规则", businessSettings: "业务与目录设置",
    category: "分类", changePassword: "修改密码", passwordReset: "找回密码",
    clientConnection: "客户端连接", clientPairing: "客户端配对", connectionWizard: "多电脑连接",
    department: "部门", item: "物品", restoreBackup: "恢复备份", secondBackupDir: "第二备份目录",
    softwareUpdate: "软件更新", stocktakeCounts: "盘点实盘", stocktakeCreate: "创建盘点单",
    supplier: "供应商", unit: "单位", user: "用户",
  };
  if (editor === "stocktakeCounts") return "录入盘点实盘";
  if (editor === "stocktakeCreate") return "创建盘点单";
  if (editor === "adjustment") return "新建调整单";
  if (["changePassword", "passwordReset", "businessSettings", "softwareUpdate", "clientConnection", "clientPairing", "connectionWizard", "secondBackupDir", "restoreBackup"].includes(editor)) return labels[editor];
  return `${mode === "edit" ? "编辑" : "新增"}${labels[editor]}`;
}

function editorWindowSize(editor: EditorKind) {
  if (["department", "category", "unit", "supplier", "budget", "changePassword", "secondBackupDir"].includes(editor)) return { width: 620, height: 380, minWidth: 520, minHeight: 320 };
  if (editor === "passwordReset") return { width: 460, height: 420, minWidth: 420, minHeight: 360 };
  if (["item", "user", "businessSettings"].includes(editor)) return { width: 760, height: 560, minWidth: 640, minHeight: 420 };
  if (["stockDocumentDetail", "stockBatchDetail", "stocktakeDetail"].includes(editor)) return { width: 980, height: 680, minWidth: 760, minHeight: 520 };
  if (["clientConnection", "restoreBackup"].includes(editor)) return { width: 720, height: 560, minWidth: 620, minHeight: 420 };
  if (editor === "clientPairing") return { width: 620, height: 420, minWidth: 520, minHeight: 340 };
  if (editor === "connectionWizard") return { width: 680, height: 560, minWidth: 560, minHeight: 460 };
  if (editor === "softwareUpdate") return { width: 760, height: 620, minWidth: 620, minHeight: 480 };
  return { width: 860, height: 720, minWidth: 680, minHeight: 560 };
}

async function bringEditorWindowToFront(windowRef: WebviewWindow) {
  if (await windowRef.isMinimized()) await windowRef.unminimize();
  if (!(await windowRef.isVisible())) await windowRef.show();
  await windowRef.setFocus();
}

export async function openEditorWindow(editor: EditorKind, options: {
  extra?: Record<string, string | undefined>; mode?: EditorMode; id?: string;
  documentType?: "inbound" | "outbound"; width?: number; height?: number;
} = {}) {
  const mode = options.mode ?? "create";
  const stableContext = options.id ?? options.documentType ?? options.extra?.periodMonth ?? "new";
  const label = ["editor", editor, mode, stableContext].filter(Boolean).map((part) => String(part).replace(/[^a-zA-Z0-9_-]/g, "-")).join("-");
  const existing = await WebviewWindow.getByLabel(label);
  if (existing) return bringEditorWindowToFront(existing);
  if (openingEditorWindows.has(label)) {
    window.setTimeout(() => void WebviewWindow.getByLabel(label).then((windowRef) => windowRef && bringEditorWindowToFront(windowRef)), 120);
    return;
  }
  openingEditorWindows.add(label);
  try {
    const size = editorWindowSize(editor);
    const search = new URLSearchParams();
    Object.entries({ documentType: options.documentType, editor, id: options.id, mode, ...options.extra }).forEach(([key, value]) => { if (value) search.set(key, value); });
    const windowRef = new WebviewWindow(label, {
      center: true, height: options.height ?? size.height, minHeight: size.minHeight,
      minWidth: size.minWidth, resizable: true, title: editorTitle(editor, mode, options.documentType),
      url: `${window.location.pathname}?${search.toString()}`, width: options.width ?? size.width,
    });
    await new Promise<void>((resolve) => {
      windowRef.once("tauri://created", () => resolve());
      windowRef.once("tauri://error", () => resolve());
    });
  } finally {
    openingEditorWindows.delete(label);
  }
}

export async function closeCurrentEditorWindow() {
  await WebviewWindow.getCurrent().close();
}
