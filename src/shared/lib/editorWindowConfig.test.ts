import { describe, expect, it } from "vitest";

import {
  editorTitle,
  editorWindowBackground,
  editorWindowSize,
  editorWindowTheme,
  usesMacOverlayTitlebar,
  type EditorKind,
} from "./editorWindowConfig";

const editorKinds: EditorKind[] = [
  "item", "department", "category", "unit", "supplier", "budget", "user",
  "changePassword", "passwordReset", "businessSettings", "softwareUpdate",
  "clientConnection", "clientPairing", "connectionWizard", "secondBackupDir",
  "restoreBackup", "stockDocument", "stockDocumentDetail", "stockBatchDetail",
  "adjustment", "stocktakeCreate", "stocktakeDetail", "stocktakeCounts",
];

describe("editor window configuration", () => {
  it("uses overlay titlebars only on macOS", () => {
    expect(usesMacOverlayTitlebar("macos", "Windows")).toBe(true);
    expect(usesMacOverlayTitlebar("windows", "Macintosh")).toBe(false);
    expect(usesMacOverlayTitlebar(undefined, "Mozilla/5.0 (Macintosh)")).toBe(true);
    expect(usesMacOverlayTitlebar(undefined, "Mozilla/5.0 (Windows NT 10.0)")).toBe(false);
  });

  it("keeps native window colors aligned with the active theme", () => {
    expect(editorWindowTheme("dark")).toBe("dark");
    expect(editorWindowBackground("dark")).toEqual([28, 28, 30, 255]);
    expect(editorWindowTheme("light")).toBe("light");
    expect(editorWindowBackground("light")).toEqual([251, 251, 253, 255]);
  });

  it("defines usable dimensions and titles for every editor", () => {
    for (const editor of editorKinds) {
      const size = editorWindowSize(editor);
      expect(size.width).toBeGreaterThanOrEqual(size.minWidth);
      expect(size.height).toBeGreaterThanOrEqual(size.minHeight);
      expect(editorTitle(editor, "create").trim().length).toBeGreaterThan(0);
    }
    expect(editorTitle("stockDocument", "create", "outbound")).toBe("新建出库/领用单");
    expect(editorTitle("stockDocumentDetail", "edit", "outbound")).toContain("出库");
    expect(editorTitle("stockBatchDetail", "edit")).toBe("批次库存");
    expect(editorTitle("stocktakeDetail", "edit")).toBe("盘点详情");
    expect(editorTitle("item", "edit")).toBe("编辑物品");
  });
});
