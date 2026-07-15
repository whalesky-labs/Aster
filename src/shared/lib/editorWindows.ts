import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalPosition } from "@tauri-apps/api/dpi";
import {
  editorTitle,
  editorWindowBackground,
  editorWindowSize,
  editorWindowTheme,
  usesMacOverlayTitlebar,
  type EditorKind,
  type EditorMode,
} from "./editorWindowConfig";

export type { EditorKind, EditorMode } from "./editorWindowConfig";

const openingEditorWindows = new Set<string>();
export const EDITOR_WINDOW_ERROR_EVENT = "aster:editor-window-error";

function reportEditorWindowError(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  window.dispatchEvent(new CustomEvent<string>(EDITOR_WINDOW_ERROR_EVENT, {
    detail: `无法打开子窗口：${message}`,
  }));
}

function currentPlatform() {
  const internals = window as Window & {
    __TAURI_OS_PLUGIN_INTERNALS__?: { platform?: string };
  };
  return internals.__TAURI_OS_PLUGIN_INTERNALS__?.platform;
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
  if (existing) {
    try {
      await bringEditorWindowToFront(existing);
    } catch (error) {
      reportEditorWindowError(error);
    }
    return;
  }
  if (openingEditorWindows.has(label)) {
    window.setTimeout(() => {
      void WebviewWindow.getByLabel(label)
        .then((windowRef) => windowRef && bringEditorWindowToFront(windowRef))
        .catch(reportEditorWindowError);
    }, 120);
    return;
  }
  openingEditorWindows.add(label);
  try {
    const size = editorWindowSize(editor);
    const title = editorTitle(editor, mode, options.documentType);
    const overlayTitlebar = usesMacOverlayTitlebar(currentPlatform(), navigator.userAgent);
    const theme = document.documentElement.dataset.theme;
    const search = new URLSearchParams();
    Object.entries({ documentType: options.documentType, editor, id: options.id, mode, titlebar: overlayTitlebar ? "overlay" : undefined, ...options.extra }).forEach(([key, value]) => { if (value) search.set(key, value); });
    const windowRef = new WebviewWindow(label, {
      backgroundColor: editorWindowBackground(theme), center: true,
      height: options.height ?? size.height, minHeight: size.minHeight,
      hiddenTitle: overlayTitlebar, minWidth: size.minWidth, resizable: true,
      theme: editorWindowTheme(theme), title,
      titleBarStyle: overlayTitlebar ? "overlay" : undefined,
      trafficLightPosition: overlayTitlebar ? new LogicalPosition(14, 19) : undefined,
      url: `${window.location.pathname}?${search.toString()}`, width: options.width ?? size.width,
    });
    await new Promise<void>((resolve, reject) => {
      windowRef.once("tauri://created", () => resolve());
      windowRef.once<unknown>("tauri://error", (event) => {
        reject(new Error(String(event.payload ?? "窗口创建失败")));
      });
    });
  } catch (error) {
    reportEditorWindowError(error);
  } finally {
    openingEditorWindows.delete(label);
  }
}

export async function closeCurrentEditorWindow() {
  await WebviewWindow.getCurrent().close();
}
