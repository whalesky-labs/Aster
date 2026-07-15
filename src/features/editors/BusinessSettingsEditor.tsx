import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import type { SystemSettings } from "../../entities/runtime";
import { Field, MonthSelect, PathPickerField } from "../../shared/ui/DataTable";
import { EditorForm } from "../../shared/ui/EditorForm";
import { localMonth } from "../../shared/lib/localDate";

async function chooseDirectory(title: string, defaultPath?: string) {
  const selected = await open({
    title,
    directory: true,
    multiple: false,
    defaultPath,
    canCreateDirectories: true,
  });
  return typeof selected === "string" ? selected : null;
}
export function BusinessSettingsEditor({
  disabled,
  localDirectoriesOnly = false,
  onSave,
  settings,
}: {
  disabled: boolean;
  localDirectoriesOnly?: boolean;
  onSave: (request: SystemSettings) => Promise<void>;
  settings: SystemSettings | null;
}) {
  const [draft, setDraft] = useState<SystemSettings>(
    settings ?? {
      hotelName: "",
      currentPeriod: localMonth(),
      defaultMonth: localMonth(),
      allowNegativeStock: false,
      quantityDecimals: 2,
      amountDecimals: 2,
      defaultExportDir: "",
      defaultBackupDir: "",
      autoBackupEnabled: true,
      intervalBackupEnabled: false,
      intervalBackupHours: 24,
      smtpEnabled: false,
      smtpHost: "",
      smtpPort: 465,
      smtpUsername: "",
      smtpPassword: "",
      smtpFromEmail: "",
      smtpFromName: "Aster",
      smtpPasswordConfigured: false,
    },
  );

  useEffect(() => {
    if (settings) setDraft(settings);
  }, [settings]);

  async function selectDirectory(
    title: string,
    key: "defaultExportDir" | "defaultBackupDir",
  ) {
    const selected = await chooseDirectory(title, draft[key] || undefined);
    if (selected) setDraft({ ...draft, [key]: selected });
  }

  return (
    <EditorForm
      disabled={disabled || (!localDirectoriesOnly && !draft.hotelName.trim())}
      saveLabel={localDirectoriesOnly ? "保存本机目录" : "保存系统设置"}
      onSave={() => onSave(draft)}
    >
      {localDirectoriesOnly ? (
        <div className="notice-banner">
          客户端模式下业务参数由主机统一控制，这里只保存当前电脑的导出和备份目录。
        </div>
      ) : null}
      <Field label="酒店名称">
        <input
          autoFocus
          disabled={localDirectoriesOnly}
          value={draft.hotelName}
          onChange={(event) => setDraft({ ...draft, hotelName: event.target.value })}
        />
      </Field>
      <Field label="当前账期">
        <MonthSelect
          disabled={localDirectoriesOnly}
          value={draft.currentPeriod}
          onChange={(currentPeriod) => setDraft({ ...draft, currentPeriod })}
        />
      </Field>
      <Field label="默认月份">
        <MonthSelect
          disabled={localDirectoriesOnly}
          value={draft.defaultMonth}
          onChange={(defaultMonth) => setDraft({ ...draft, defaultMonth })}
        />
      </Field>
      <Field label="数量小数位">
        <input
          max="6"
          min="0"
          disabled={localDirectoriesOnly}
          type="number"
          value={draft.quantityDecimals}
          onChange={(event) =>
            setDraft({ ...draft, quantityDecimals: Number(event.target.value) })
          }
        />
      </Field>
      <Field label="金额小数位">
        <input
          max="6"
          min="0"
          disabled={localDirectoriesOnly}
          type="number"
          value={draft.amountDecimals}
          onChange={(event) =>
            setDraft({ ...draft, amountDecimals: Number(event.target.value) })
          }
        />
      </Field>
      <Field label="定时备份小时">
        <input
          max="168"
          min="1"
          disabled={localDirectoriesOnly}
          type="number"
          value={draft.intervalBackupHours}
          onChange={(event) =>
            setDraft({ ...draft, intervalBackupHours: Number(event.target.value) })
          }
        />
      </Field>
      <Field label="默认导出目录">
        <PathPickerField
          value={draft.defaultExportDir}
          placeholder="请选择默认导出目录"
          buttonLabel="选择"
          onChoose={() => selectDirectory("选择默认导出目录", "defaultExportDir")}
        />
      </Field>
      <Field label="默认备份目录">
        <PathPickerField
          value={draft.defaultBackupDir}
          placeholder="请选择默认备份目录"
          buttonLabel="选择"
          onChoose={() => selectDirectory("选择默认备份目录", "defaultBackupDir")}
        />
      </Field>
      <label className="checkbox-field">
        <input
          checked={draft.allowNegativeStock}
          disabled={localDirectoriesOnly}
          onChange={(event) =>
            setDraft({ ...draft, allowNegativeStock: event.target.checked })
          }
          type="checkbox"
        />
        允许负库存
      </label>
      <label className="checkbox-field">
        <input
          checked={draft.autoBackupEnabled}
          disabled={localDirectoriesOnly}
          onChange={(event) =>
            setDraft({ ...draft, autoBackupEnabled: event.target.checked })
          }
          type="checkbox"
        />
        启动自动备份
      </label>
      <label className="checkbox-field">
        <input
          checked={draft.intervalBackupEnabled}
          disabled={localDirectoriesOnly}
          onChange={(event) =>
            setDraft({ ...draft, intervalBackupEnabled: event.target.checked })
          }
          type="checkbox"
        />
        运行中定时备份
      </label>
      <label className="checkbox-field">
        <input
          checked={draft.smtpEnabled}
          disabled={localDirectoriesOnly}
          onChange={(event) =>
            setDraft({ ...draft, smtpEnabled: event.target.checked })
          }
          type="checkbox"
        />
        启用邮箱验证码找回密码
      </label>
      <Field label="SMTP 主机">
        <input
          disabled={localDirectoriesOnly}
          placeholder="smtp.example.com"
          value={draft.smtpHost}
          onChange={(event) => setDraft({ ...draft, smtpHost: event.target.value })}
        />
      </Field>
      <Field label="SMTP 端口">
        <input
          max="65535"
          min="1"
          disabled={localDirectoriesOnly}
          type="number"
          value={draft.smtpPort}
          onChange={(event) =>
            setDraft({ ...draft, smtpPort: Number(event.target.value) })
          }
        />
      </Field>
      <Field label="SMTP 账号">
        <input
          autoComplete="username"
          disabled={localDirectoriesOnly}
          value={draft.smtpUsername}
          onChange={(event) =>
            setDraft({ ...draft, smtpUsername: event.target.value })
          }
        />
      </Field>
      <Field label="SMTP 授权码">
        <input
          autoComplete="new-password"
          disabled={localDirectoriesOnly}
          placeholder={draft.smtpPasswordConfigured ? "已配置，留空不修改" : ""}
          type="password"
          value={draft.smtpPassword ?? ""}
          onChange={(event) =>
            setDraft({ ...draft, smtpPassword: event.target.value })
          }
        />
      </Field>
      <Field label="发件邮箱">
        <input
          autoComplete="email"
          disabled={localDirectoriesOnly}
          value={draft.smtpFromEmail}
          onChange={(event) =>
            setDraft({ ...draft, smtpFromEmail: event.target.value })
          }
        />
      </Field>
      <Field label="发件名称">
        <input
          disabled={localDirectoriesOnly}
          value={draft.smtpFromName}
          onChange={(event) =>
            setDraft({ ...draft, smtpFromName: event.target.value })
          }
        />
      </Field>
    </EditorForm>
  );
}
