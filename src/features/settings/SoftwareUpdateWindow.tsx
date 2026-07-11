import type { I18n } from "../../i18n";
import type { AppStatus, AppUpdateState } from "../../entities/runtime";

function softwareUpdateTitle(updateState: AppUpdateState, i18n: I18n) {
  if (updateState.status === "checking") return i18n.t("settings.updateStatusChecking");
  if (updateState.status === "available") return i18n.t("settings.updateStatusAvailable", { version: updateState.latestVersion ?? "-" });
  if (updateState.status === "downloading") return i18n.t("settings.updateStatusDownloading");
  if (updateState.status === "installed") return i18n.t("settings.updateStatusInstalled");
  if (updateState.status === "error") return i18n.t("settings.updateStatusError");
  if (updateState.status === "notAvailable") return i18n.t("settings.updateStatusLatest");
  return i18n.t("settings.updateStatusIdle");
}

function softwareUpdateHint(updateState: AppUpdateState, i18n: I18n) {
  if (updateState.status === "available") return i18n.t("settings.updateHintAvailable");
  if (updateState.status === "downloading") return i18n.t("settings.updateHintDownloading");
  if (updateState.status === "installed") return i18n.t("settings.updateHintInstalled");
  if (updateState.status === "error") return i18n.t("settings.updateHintError");
  return i18n.t("settings.updateHintIdle");
}

export function SoftwareUpdateWindow({
  disabled,
  i18n,
  status,
  updateState,
  onCheck,
  onInstall,
  onRestart,
}: {
  disabled: boolean;
  i18n: I18n;
  status: AppStatus | null;
  updateState: AppUpdateState;
  onCheck: () => Promise<void>;
  onInstall: () => Promise<void>;
  onRestart: () => Promise<void>;
}) {
  const updateIsBusy =
    disabled ||
    updateState.status === "checking" ||
    updateState.status === "downloading";
  const updateProgress =
    updateState.totalBytes && updateState.totalBytes > 0
      ? Math.min(100, (updateState.downloadedBytes / updateState.totalBytes) * 100)
      : updateState.status === "downloading"
        ? null
        : 0;

  return (
    <article className="editor-document software-update-window">
      <div className="software-update-scroll">
        <div className="software-update-panel">
          <div className="software-update-header">
            <div>
              <span className="settings-feature-kicker">
                {i18n.t("settings.updateChannel")}
              </span>
              <h4>{softwareUpdateTitle(updateState, i18n)}</h4>
              <p>{softwareUpdateHint(updateState, i18n)}</p>
            </div>
            <span
              className={`status ${
                updateState.status === "error"
                  ? "disabled"
                  : updateState.status === "available" ||
                      updateState.status === "installed"
                    ? "enabled"
                    : ""
              }`}
            >
              {updateState.status === "checking"
                ? i18n.t("settings.checkingUpdate")
                : updateState.status === "downloading"
                  ? i18n.t("settings.updateDownloading")
                  : updateState.status === "installed"
                    ? i18n.t("settings.updateStatusInstalled")
                    : updateState.status === "available"
                      ? i18n.t("settings.updateStatusAvailable", {
                          version: updateState.latestVersion ?? "-",
                        })
                      : updateState.status === "notAvailable"
                        ? i18n.t("settings.updateStatusLatest")
                        : updateState.status === "error"
                          ? i18n.t("settings.updateFailed")
                          : i18n.t("settings.updateStatusIdle")}
            </span>
          </div>

          <dl className="software-update-metrics">
            <div>
              <dt>{i18n.t("settings.currentVersion")}</dt>
              <dd>{updateState.currentVersion ?? status?.appVersion ?? "-"}</dd>
            </div>
            <div>
              <dt>{i18n.t("settings.latestVersion")}</dt>
              <dd>{updateState.latestVersion ?? "-"}</dd>
            </div>
            <div>
              <dt>{i18n.t("settings.updateCheckedAt")}</dt>
              <dd>{updateState.checkedAt ?? "-"}</dd>
            </div>
            <div>
              <dt>{i18n.t("settings.updateSource")}</dt>
              <dd>
                {updateState.sourceLabel
                  ? `GitHub Releases · ${updateState.sourceLabel}`
                  : "GitHub Releases"}
              </dd>
            </div>
          </dl>

          {updateState.status === "downloading" ? (
            <div className="settings-update-progress">
              <div className="settings-update-progress-track">
                <span
                  style={{
                    width:
                      updateProgress == null
                        ? "35%"
                        : `${Math.max(4, updateProgress)}%`,
                  }}
                />
              </div>
              <span>
                {updateProgress == null
                  ? i18n.t("settings.updateDownloading")
                  : i18n.t("settings.updateProgress", {
                      percent: Math.round(updateProgress),
                    })}
              </span>
            </div>
          ) : null}

          {updateState.status === "error" && updateState.error ? (
            <div className="settings-inline-note warning">
              <strong>{i18n.t("settings.updateFailed")}</strong>
              <span>{updateState.error}</span>
            </div>
          ) : null}

          <div className="software-update-notes">
            <strong>{i18n.t("settings.updateNotes")}</strong>
            <p>{updateState.notes || i18n.t("settings.updateNotesEmpty")}</p>
          </div>
        </div>
      </div>
      <div className="editor-actions">
        <button
          className="ghost-button"
          disabled={updateIsBusy}
          type="button"
          onClick={() => void onCheck()}
        >
          {updateState.status === "checking"
            ? i18n.t("settings.checkingUpdate")
            : i18n.t("settings.checkUpdate")}
        </button>
        {updateState.status === "available" ? (
          <button
            className="primary-button"
            disabled={updateIsBusy}
            type="button"
            onClick={() => void onInstall()}
          >
            {i18n.t("settings.downloadInstallUpdate")}
          </button>
        ) : null}
        {updateState.status === "installed" ? (
          <button
            className="primary-button"
            type="button"
            onClick={() => void onRestart()}
          >
            {i18n.t("settings.restartToUpdate")}
          </button>
        ) : null}
      </div>
    </article>
  );
}
