export type RefreshTarget = "none" | "master" | "stock" | "admin" | "connection" | "business";

export function refreshTargetForEditor(editor: string): RefreshTarget {
  if (["item", "category", "unit", "department", "supplier"].includes(editor)) return "master";
  if (["stockDocument", "adjustment", "stocktakeCreate", "stocktakeCounts"].includes(editor)) return "stock";
  if (["connectionWizard", "clientConnection", "clientPairing"].includes(editor)) return "connection";
  if (["budget", "user", "businessSettings", "secondBackupDir", "restoreBackup"].includes(editor)) return "admin";
  return "none";
}
