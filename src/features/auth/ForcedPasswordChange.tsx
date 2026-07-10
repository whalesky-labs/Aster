import { useState } from "react";

export function ForcedPasswordChange({
  error,
  isPending,
  onChange,
}: {
  error: string | null;
  isPending: boolean;
  onChange: (oldPassword: string, newPassword: string) => Promise<void>;
}) {
  const [oldPassword, setOldPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmation, setConfirmation] = useState("");
  const [localError, setLocalError] = useState<string | null>(null);

  function submit(event: React.FormEvent) {
    event.preventDefault();
    if (newPassword.length < 8) {
      setLocalError("新密码至少 8 位");
      return;
    }
    if (newPassword === "admin123") {
      setLocalError("新密码不能继续使用默认密码");
      return;
    }
    if (newPassword !== confirmation) {
      setLocalError("两次输入的新密码不一致");
      return;
    }
    setLocalError(null);
    void onChange(oldPassword, newPassword);
  }

  return (
    <main className="login-shell">
      <section className="login-card">
        <div className="login-card-main">
          <div className="login-card-header">
            <h2>首次登录必须修改默认密码</h2>
            <p>完成修改后才能进入库存系统。</p>
          </div>
          {error || localError ? (
            <div className="error-banner login-message">
              {localError ?? error}
            </div>
          ) : null}
          <form className="login-form" onSubmit={submit}>
            <label>
              <span>当前密码</span>
              <input
                autoComplete="current-password"
                autoFocus
                disabled={isPending}
                onChange={(event) => setOldPassword(event.target.value)}
                type="password"
                value={oldPassword}
              />
            </label>
            <label>
              <span>新密码</span>
              <input
                autoComplete="new-password"
                disabled={isPending}
                onChange={(event) => setNewPassword(event.target.value)}
                type="password"
                value={newPassword}
              />
            </label>
            <label>
              <span>确认新密码</span>
              <input
                autoComplete="new-password"
                disabled={isPending}
                onChange={(event) => setConfirmation(event.target.value)}
                type="password"
                value={confirmation}
              />
            </label>
            <button
              className="primary-button login-submit"
              disabled={isPending || !oldPassword || !newPassword || !confirmation}
              type="submit"
            >
              {isPending ? "正在修改…" : "修改密码并继续"}
            </button>
          </form>
        </div>
      </section>
    </main>
  );
}
