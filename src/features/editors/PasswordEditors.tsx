import { useState } from "react";
import type { I18n } from "../../i18n";
import { Field } from "../../shared/ui/DataTable";
import { EditorForm } from "../../shared/ui/EditorForm";

export function ChangePasswordEditor({
  disabled,
  onSave,
}: {
  disabled: boolean;
  onSave: (request: { oldPassword: string; newPassword: string }) => Promise<void>;
}) {
  const [oldPassword, setOldPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  return (
    <EditorForm
      disabled={disabled || !oldPassword || !newPassword}
      saveLabel="修改密码"
      onSave={() => onSave({ oldPassword, newPassword })}
    >
      <Field label="旧密码">
        <input
          autoFocus
          type="password"
          value={oldPassword}
          onChange={(event) => setOldPassword(event.target.value)}
        />
      </Field>
      <Field label="新密码">
        <input
          type="password"
          value={newPassword}
          onChange={(event) => setNewPassword(event.target.value)}
        />
      </Field>
    </EditorForm>
  );
}

export function PasswordResetEditor({
  disabled,
  i18n,
  onRequestCode,
  onReset,
}: {
  disabled: boolean;
  i18n: I18n;
  onRequestCode: (username: string) => Promise<void>;
  onReset: (request: {
    username: string;
    code: string;
    newPassword: string;
  }) => Promise<void>;
}) {
  const [username, setUsername] = useState("");
  const [code, setCode] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const canRequestCode = username.trim().length > 0;
  const canReset =
    username.trim().length > 0 && code.length === 6 && newPassword.length >= 6;

  return (
    <EditorForm
      contentClassName="password-reset-grid"
      disabled={disabled || !canReset}
      saveLabel={i18n.t("login.resetPassword")}
      onSave={() =>
        onReset({
          username: username.trim(),
          code,
          newPassword,
        })
      }
    >
      <p className="editor-form-note">{i18n.t("login.resetHint")}</p>
      <Field label={i18n.t("login.username")}>
        <input
          autoComplete="username"
          autoFocus
          disabled={disabled}
          value={username}
          onChange={(event) => setUsername(event.target.value)}
        />
      </Field>
      <div className="editor-inline-actions">
        <button
          className="ghost-button"
          disabled={disabled || !canRequestCode}
          onClick={() => void onRequestCode(username.trim())}
          type="button"
        >
          {i18n.t("login.sendCode")}
        </button>
      </div>
      <Field label={i18n.t("login.resetCode")}>
        <input
          autoComplete="one-time-code"
          disabled={disabled}
          inputMode="numeric"
          maxLength={12}
          value={code}
          onChange={(event) =>
            setCode(event.target.value.replace(/\D/g, "").slice(0, 6))
          }
        />
      </Field>
      <Field label={i18n.t("login.newPassword")}>
        <input
          autoComplete="new-password"
          disabled={disabled}
          type="password"
          value={newPassword}
          onChange={(event) => setNewPassword(event.target.value)}
        />
      </Field>
    </EditorForm>
  );
}
