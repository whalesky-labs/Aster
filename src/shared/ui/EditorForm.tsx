import type { ReactNode } from "react";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

export function EditorForm({
  children,
  contentClassName,
  disabled,
  onSave,
  saveLabel,
}: {
  children: ReactNode;
  contentClassName?: string;
  disabled: boolean;
  onSave: () => Promise<void>;
  saveLabel: string;
}) {
  return (
    <div className="editor-form">
      <div
        className={
          contentClassName
            ? `editor-form-grid ${contentClassName}`
            : "editor-form-grid"
        }
      >
        {children}
      </div>
      <div className="editor-actions">
        <button
          className="ghost-button"
          disabled={disabled}
          onClick={() => void WebviewWindow.getCurrent().close()}
          type="button"
        >
          取消
        </button>
        <button className="primary-button" disabled={disabled} onClick={onSave}>
          {saveLabel}
        </button>
      </div>
    </div>
  );
}
