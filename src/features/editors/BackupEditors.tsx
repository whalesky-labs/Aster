import { useState } from "react";
import { open, type OpenDialogOptions } from "@tauri-apps/plugin-dialog";
import type { AppStatus } from "../../entities/runtime";
import { formatDateTime } from "../../shared/lib/display";
import { Field, PathPickerField } from "../../shared/ui/DataTable";
import { EditorForm } from "../../shared/ui/EditorForm";

type RestorePreview = {
  backupFile: string;
  metadata: { createdAt: string; schemaVersion: number; sourceHostName?: string | null; databaseSha256: string };
  message: string;
  validationToken: string;
};

async function choosePath(options: OpenDialogOptions) {
  const selected = await open({ ...options, multiple: false });
  return typeof selected === "string" ? selected : null;
}

export function SecondBackupDirEditor({
  disabled,
  onSave,
  status,
}: {
  disabled: boolean;
  onSave: (path: string) => Promise<void>;
  status: AppStatus | null;
}) {
  const [path, setPath] = useState(status?.runtime.backupDir ?? "");
  async function selectDirectory() {
    const selected = await choosePath({
      title: "选择第二备份目录",
      directory: true,
      defaultPath: path || undefined,
      canCreateDirectories: true,
    });
    if (selected) setPath(selected);
  }
  return (
    <EditorForm
      disabled={disabled || !path}
      saveLabel="保存目录"
      onSave={() => onSave(path)}
    >
      <Field label="第二备份目录">
        <PathPickerField
          value={path}
          placeholder="请选择外接硬盘或同步盘目录"
          buttonLabel="选择"
          onChoose={selectDirectory}
        />
      </Field>
    </EditorForm>
  );
}

export function RestoreBackupEditor({
  disabled,
  onPreview,
  onRestore,
  status,
}: {
  disabled: boolean;
  onPreview: (backupFile: string) => Promise<RestorePreview>;
  onRestore: (request: {
    backupFile: string;
    confirmation: string;
    validationToken: string;
  }) => Promise<void>;
  status: AppStatus | null;
}) {
  const [backupFile, setBackupFile] = useState("");
  const [confirmation, setConfirmation] = useState("");
  const [preview, setPreview] = useState<RestorePreview | null>(null);
  async function selectFile() {
    const selected = await choosePath({
      title: "选择备份文件",
      filters: [{ name: "Aster 备份文件", extensions: ["zip"] }],
      defaultPath: backupFile || status?.runtime.backupDir || undefined,
    });
    if (selected) {
      setBackupFile(selected);
      setPreview(null);
    }
  }
  return (
    <EditorForm
      disabled={
        disabled ||
        !preview ||
        preview.backupFile !== backupFile ||
        !preview.validationToken ||
        confirmation !== "RESTORE"
      }
      saveLabel="恢复备份"
      onSave={() =>
        onRestore({
          backupFile,
          confirmation,
          validationToken: preview?.validationToken ?? "",
        })
      }
    >
      <Field label="备份文件">
        <PathPickerField
          value={backupFile}
          placeholder="请选择 aster-backup-YYYYMMDD-HHMMSS-短ID.zip"
          buttonLabel="选择"
          onChoose={selectFile}
        />
      </Field>
      <button
        className="ghost-button"
        disabled={disabled || !backupFile}
        type="button"
        onClick={async () => setPreview(await onPreview(backupFile))}
      >
        校验备份包
      </button>
      <Field label="确认文本">
        <input
          value={confirmation}
          onChange={(event) => setConfirmation(event.target.value)}
          placeholder="输入 RESTORE"
        />
      </Field>
      {preview ? (
        <div className="settings-result">
          <strong>{preview.message}</strong>
          <span>创建时间：{formatDateTime(preview.metadata.createdAt)}</span>
          <span>Schema：v{preview.metadata.schemaVersion}</span>
          <span>来源主机：{preview.metadata.sourceHostName ?? "-"}</span>
          <span>校验：{preview.metadata.databaseSha256}</span>
        </div>
      ) : null}
    </EditorForm>
  );
}
