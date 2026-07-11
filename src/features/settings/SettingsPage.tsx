import type { Dispatch, SetStateAction } from "react";
import type { I18n } from "../../i18n";
import type {
  AppStatus,
  AppUpdateState,
  AppearanceSettings,
  ClientConnectionInfo,
  HostConnectionTestResult,
  HostServiceStatus,
  SystemSettings,
} from "../../entities/runtime";
import type { CurrentUser } from "../../entities/users";
import { formatFileSize } from "../../shared/lib/display";
import { AppearanceSettingsPanel } from "./AppearanceSettingsPanel";
import { ConnectionSettingsPanel } from "./ConnectionSettingsPanel";

type BackupSummary = {
  backupFile: string;
  backupType: string;
  createdAt: string;
  schemaVersion: number;
  sourceHostName: string;
  sourceOs: string;
  databaseSize: number;
  databaseSha256: string;
  secondBackupFile?: string | null;
};

function softwareUpdateTitle(state: AppUpdateState, i18n: I18n) {
  if (state.status === "checking") return i18n.t("settings.updateStatusChecking");
  if (state.status === "available") return i18n.t("settings.updateStatusAvailable", { version: state.latestVersion ?? "-" });
  if (state.status === "downloading") return i18n.t("settings.updateStatusDownloading");
  if (state.status === "installed") return i18n.t("settings.updateStatusInstalled");
  if (state.status === "error") return i18n.t("settings.updateStatusError");
  if (state.status === "notAvailable") return i18n.t("settings.updateStatusLatest");
  return i18n.t("settings.updateStatusIdle");
}

function softwareUpdateHint(state: AppUpdateState, i18n: I18n) {
  if (state.status === "available") return i18n.t("settings.updateHintAvailable");
  if (state.status === "downloading") return i18n.t("settings.updateHintDownloading");
  if (state.status === "installed") return i18n.t("settings.updateHintInstalled");
  if (state.status === "error") return i18n.t("settings.updateHintError");
  return i18n.t("settings.updateHintIdle");
}

export function SettingsPage({
  appearanceSettings,
  canManage,
  clientConnectionCheckedAt,
  clientConnections,
  currentUser,
  hostStatus,
  hostTestResult,
  i18n,
  isWorking,
  lastBackup,
  onBackup,
  onAppearanceChange,
  onLogout,
  onOpenBusinessSettings,
  onOpenChangePassword,
  onOpenRestoreBackup,
  onOpenSecondBackupDir,
  onOpenConnectionWizard,
  onOpenSoftwareUpdate,
  onRemoveClientConnection,
  onStartHostService,
  status,
  systemSettings,
  updateState,
}: {
  appearanceSettings: AppearanceSettings;
  canManage: boolean;
  clientConnectionCheckedAt: string | null;
  clientConnections: ClientConnectionInfo[];
  currentUser: CurrentUser;
  hostStatus: HostServiceStatus | null;
  hostTestResult: HostConnectionTestResult | null;
  i18n: I18n;
  isWorking: boolean;
  lastBackup: BackupSummary | null;
  onBackup: () => Promise<void>;
  onAppearanceChange: Dispatch<SetStateAction<AppearanceSettings>>;
  onLogout: () => Promise<void>;
  onOpenBusinessSettings: () => void;
  onOpenChangePassword: () => void;
  onOpenRestoreBackup: () => void;
  onOpenSecondBackupDir: () => void;
  onOpenConnectionWizard: () => void;
  onOpenSoftwareUpdate: () => void;
  onRemoveClientConnection: (client: ClientConnectionInfo) => Promise<void>;
  onStartHostService: () => Promise<void>;
  status: AppStatus | null;
  systemSettings: SystemSettings | null;
  updateState: AppUpdateState;
}) {
  const settingsIsClientMode = status?.runtime.mode === "client";
  const canOpenBusinessSettings = canManage;
  const canOperatePrimaryDatabase = canManage && !settingsIsClientMode;
  const settingsBackupMetrics = [
    {
      label: i18n.t("settings.databaseStatus"),
      value: status?.health.databaseOk
        ? i18n.t("settings.statusHealthy")
        : i18n.t("settings.statusAbnormal"),
      suffix: "",
    },
    {
      label: i18n.t("settings.latestBackup"),
      value: status?.health.latestBackupAt ?? "-",
      suffix: "",
    },
    {
      label: i18n.t("settings.secondBackup"),
      value: status?.health.secondBackupOk
        ? i18n.t("settings.available")
        : i18n.t("settings.notReady"),
      suffix: "",
    },
    {
      label: i18n.t("settings.intervalBackup"),
      value: status?.health.intervalBackupEnabled
        ? i18n.t("settings.hours", {
            hours: status.health.intervalBackupHours,
          })
        : i18n.t("settings.closed"),
      suffix: "",
    },
  ];
  return (
    <section className="settings-page">
      <AppearanceSettingsPanel
        appearanceSettings={appearanceSettings}
        i18n={i18n}
        onAppearanceChange={onAppearanceChange}
      />
      <div className="settings-group">
        <h3 className="settings-group-title">
          {i18n.t("settings.runtimeStatus")}
        </h3>
        <article className="surface settings-block">
          {settingsBackupMetrics.map((card) => (
            <div className="setting-row" key={card.label}>
              <div className="settings-meta">
                <span className="settings-label">{card.label}</span>
              </div>
              <div className="setting-control">
                <strong className="setting-value">
                  {card.value}
                  {card.suffix}
                </strong>
              </div>
            </div>
          ))}
        </article>
      </div>

      <div className="settings-group">
        <h3 className="settings-group-title">
          {i18n.t("settings.softwareUpdate")}
        </h3>
        <article className="surface settings-feature-panel settings-update-summary">
          <div className="settings-feature-header">
            <div>
              <span className="settings-feature-kicker">
                {i18n.t("settings.updateChannel")}
              </span>
              <h4>{softwareUpdateTitle(updateState, i18n)}</h4>
              <p>{softwareUpdateHint(updateState, i18n)}</p>
            </div>
            <div className="settings-feature-actions">
              <button
                className="primary-button"
                type="button"
                onClick={onOpenSoftwareUpdate}
              >
                {i18n.t("settings.openSoftwareUpdate")}
              </button>
            </div>
          </div>

          <dl className="settings-metric-list">
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
        </article>
      </div>

      <div className="settings-group">
        <h3 className="settings-group-title">
          {i18n.t("settings.accountSecurity")}
        </h3>
        <article className="surface settings-block">
          <div className="setting-row">
            <div className="settings-meta">
              <span className="settings-label">
                {i18n.t("settings.currentAccount")}
              </span>
              <p className="settings-hint">
                {currentUser.roles.map((role) => role.name).join("、") ||
                  i18n.t("settings.noRole")}
              </p>
            </div>
            <div className="setting-control">
              <strong className="setting-value">
                {currentUser.displayName}
              </strong>
            </div>
          </div>
          <div className="setting-row">
            <div className="settings-meta">
              <span className="settings-label">
                {i18n.t("settings.changePassword")}
              </span>
              <p className="settings-hint">
                {i18n.t("settings.changePasswordHint")}
              </p>
            </div>
            <div className="setting-control">
              <button
                className="primary-button"
                type="button"
                onClick={onOpenChangePassword}
              >
                {i18n.t("settings.openChangePassword")}
              </button>
            </div>
          </div>
          <div className="setting-row">
            <div className="settings-meta">
              <span className="settings-label">
                {i18n.t("settings.loginSession")}
              </span>
            </div>
            <div className="setting-control">
              <button className="ghost-button" onClick={onLogout}>
                {i18n.t("settings.logout")}
              </button>
            </div>
          </div>
        </article>
      </div>

      <div className="settings-group">
        <h3 className="settings-group-title">
          {i18n.t("settings.businessAndDirectories")}
        </h3>
        <article className="surface settings-block">
          <div className="setting-row">
            <div className="settings-meta">
              <span className="settings-label">
                {systemSettings?.hotelName ||
                  i18n.t("settings.businessSettings")}
              </span>
              <p className="settings-hint">
                {settingsIsClientMode
                  ? i18n.t("settings.clientBusinessSettingsHint")
                  : i18n.t("settings.businessSettingsHint")}
              </p>
            </div>
            <div className="setting-control">
              <button
                className="primary-button"
                disabled={!canOpenBusinessSettings}
                type="button"
                onClick={onOpenBusinessSettings}
              >
                {i18n.t("settings.openBusinessSettings")}
              </button>
            </div>
          </div>
          <div className="setting-row">
            <div className="settings-meta">
              <span className="settings-label">
                {i18n.t("settings.currentPeriod")}
              </span>
            </div>
            <div className="setting-control">
              <strong className="setting-value">
                {systemSettings?.currentPeriod ?? "-"}
              </strong>
            </div>
          </div>
        </article>
      </div>

      <ConnectionSettingsPanel
        canManage={canManage}
        clientConnectionCheckedAt={clientConnectionCheckedAt}
        clientConnections={clientConnections}
        hostStatus={hostStatus}
        hostTestResult={hostTestResult}
        i18n={i18n}
        isWorking={isWorking}
        onOpenConnectionWizard={onOpenConnectionWizard}
        onRemoveClientConnection={onRemoveClientConnection}
        onStartHostService={onStartHostService}
        status={status}
      />
      <div className="settings-group">
        <h3 className="settings-group-title">
          {i18n.t("settings.backupAndRestore")}
        </h3>
        <article className="surface settings-feature-panel">
          <div className="settings-feature-header">
            <div>
              <span className="settings-feature-kicker">
                {i18n.t("settings.backupPolicy")}
              </span>
              <h4>{i18n.t("settings.backupAndRestore")}</h4>
              <p>{i18n.t("settings.backupHint")}</p>
            </div>
            <div className="settings-feature-actions">
              <button
                className="primary-button"
                disabled={!canOperatePrimaryDatabase || isWorking}
                onClick={onBackup}
              >
                {i18n.t("settings.createManualBackup")}
              </button>
              <button
                className="ghost-button"
                disabled={!canOperatePrimaryDatabase || isWorking}
                type="button"
                onClick={onOpenSecondBackupDir}
              >
                {i18n.t("settings.secondBackupDir")}
              </button>
              <button
                className="ghost-button"
                disabled={!canOperatePrimaryDatabase || isWorking}
                type="button"
                onClick={onOpenRestoreBackup}
              >
                {i18n.t("settings.restoreBackup")}
              </button>
            </div>
          </div>

          <dl className="settings-metric-list">
            <div>
              <dt>{i18n.t("settings.localDir")}</dt>
              <dd>{status?.runtime.backupDir ?? "-"}</dd>
            </div>
            <div>
              <dt>{i18n.t("settings.latestBackup")}</dt>
              <dd>{status?.health.latestBackupAt ?? "-"}</dd>
            </div>
            <div>
              <dt>{i18n.t("settings.latestIntervalBackup")}</dt>
              <dd>{status?.health.latestIntervalBackupAt ?? "-"}</dd>
            </div>
            <div>
              <dt>{i18n.t("settings.autoBackup")}</dt>
              <dd>
                {status?.health.autoBackupEnabled
                  ? i18n.t("settings.enabled")
                  : i18n.t("settings.disabled")}
              </dd>
            </div>
          </dl>

          {settingsIsClientMode ? (
            <p className="settings-footnote">
              {i18n.t("settings.clientBackupFootnote")}
            </p>
          ) : null}

          {lastBackup ? (
            <div className="settings-inline-note">
              <strong>{i18n.t("settings.backupCreated")}</strong>
              <span>{lastBackup.backupFile}</span>
              <span>
                {i18n.t("settings.backupSource", {
                  host: lastBackup.sourceHostName,
                  os: lastBackup.sourceOs,
                  schema: lastBackup.schemaVersion,
                  size: formatFileSize(lastBackup.databaseSize),
                })}
              </span>
              <span>
                {i18n.t("settings.backupSha", {
                  sha: lastBackup.databaseSha256,
                })}
              </span>
              {lastBackup.secondBackupFile ? (
                <span>
                  {i18n.t("settings.secondBackupFile", {
                    file: lastBackup.secondBackupFile,
                  })}
                </span>
              ) : null}
            </div>
          ) : null}
        </article>
      </div>
    </section>
  );

}
