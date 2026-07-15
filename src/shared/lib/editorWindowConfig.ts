export type EditorKind =
  | "item" | "department" | "category" | "unit" | "supplier" | "budget" | "user"
  | "changePassword" | "passwordReset" | "businessSettings" | "softwareUpdate"
  | "clientConnection" | "clientPairing" | "connectionWizard" | "secondBackupDir"
  | "restoreBackup" | "stockDocument" | "stockDocumentDetail" | "stockBatchDetail"
  | "adjustment" | "stocktakeCreate" | "stocktakeDetail" | "stocktakeCounts";

export type EditorMode = "create" | "edit";

export function editorWindowBackground(theme?: string): [number, number, number, number] {
  return theme === "dark" ? [28, 28, 30, 255] : [251, 251, 253, 255];
}

export function editorWindowTheme(theme?: string): "dark" | "light" {
  return theme === "dark" ? "dark" : "light";
}

export function usesMacOverlayTitlebar(platform?: string, userAgent = "") {
  return platform === "macos" || (!platform && /Macintosh|Mac OS X/.test(userAgent));
}

export function editorTitle(
  editor: EditorKind,
  mode: EditorMode,
  documentType?: "inbound" | "outbound",
) {
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

export function editorWindowSize(editor: EditorKind) {
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
