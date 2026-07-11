import type { I18n } from "../../i18n";
import type { AppStatus, ClientConnectionInfo, HostConnectionTestResult, HostServiceStatus } from "../../entities/runtime";
import { EmptyRow } from "../../shared/ui/DataTable";

function statusLabel(status: AppStatus | null, result: HostConnectionTestResult | null, i18n: I18n) {
  if (!status) return i18n.t("connection.loading");
  if (status.runtime.mode === "host") return i18n.t("connection.host");
  if (status.runtime.mode === "client") {
    if (!status.runtime.clientToken) return i18n.t("connection.unpaired");
    return result?.ok === false ? i18n.t("connection.abnormal") : i18n.t("connection.connected");
  }
  return i18n.t("connection.standalone");
}

function statusHint(status: AppStatus | null, host: HostServiceStatus | null, result: HostConnectionTestResult | null, i18n: I18n) {
  if (!status) return i18n.t("connection.hint.loading");
  if (status.runtime.mode === "host") return host?.running ? i18n.t("connection.hint.hostRunning") : i18n.t("connection.hint.hostStopped");
  if (status.runtime.mode === "client") {
    if (!status.runtime.clientToken) return i18n.t("connection.hint.clientUnpaired");
    return result?.ok === false ? i18n.t("connection.hint.clientAbnormal") : i18n.t("connection.hint.clientConnected");
  }
  return i18n.t("connection.hint.standalone");
}

export function ConnectionSettingsPanel({
  canManage, clientConnectionCheckedAt, clientConnections, hostStatus, hostTestResult,
  i18n, isWorking, onOpenConnectionWizard, onRemoveClientConnection, onStartHostService, status,
}: {
  canManage: boolean;
  clientConnectionCheckedAt: string | null;
  clientConnections: ClientConnectionInfo[];
  hostStatus: HostServiceStatus | null;
  hostTestResult: HostConnectionTestResult | null;
  i18n: I18n;
  isWorking: boolean;
  onOpenConnectionWizard: () => void;
  onRemoveClientConnection: (client: ClientConnectionInfo) => Promise<void>;
  onStartHostService: () => Promise<void>;
  status: AppStatus | null;
}) {
  const settingsIsClientMode = status?.runtime.mode === "client";
  const connectionStatusLabel = () => statusLabel(status, hostTestResult, i18n);
  const connectionStatusHint = () => statusHint(status, hostStatus, hostTestResult, i18n);
  return (
      <div className="settings-group">
        <h3 className="settings-group-title">
          {i18n.t("settings.multiComputer")}
        </h3>
        <article className="surface settings-feature-panel">
          <div className="settings-feature-header">
            <div>
              <span className="settings-feature-kicker">
                {i18n.t("settings.currentStatus")}
              </span>
              <h4>{connectionStatusLabel()}</h4>
              <p>
                {connectionStatusHint()}
              </p>
            </div>
            <div className="settings-feature-actions">
              <button
                className="primary-button"
                disabled={!canManage}
                type="button"
                onClick={onOpenConnectionWizard}
              >
                {i18n.t("settings.openConnectionWizard")}
              </button>
              {status?.runtime.mode === "host" ? (
                <button
                  className="ghost-button"
                  disabled={!canManage || isWorking}
                  type="button"
                  onClick={onStartHostService}
                >
                  {i18n.t("settings.restartSharing")}
                </button>
              ) : null}
              {settingsIsClientMode ? (
                <button
                  className="ghost-button"
                  disabled={!canManage}
                  type="button"
                  onClick={onOpenConnectionWizard}
                >
                  {i18n.t("settings.reconnect")}
                </button>
              ) : null}
            </div>
          </div>

          <dl className="settings-metric-list">
            <div>
              <dt>{i18n.t("settings.hostComputer")}</dt>
              <dd>
                {settingsIsClientMode
                  ? `${status?.runtime.hostAddress ?? "-"}:${status?.runtime.hostPort ?? "-"}`
                  : hostStatus?.running
                    ? i18n.t("settings.thisComputer")
                    : "-"}
              </dd>
            </div>
            <div>
              <dt>{i18n.t("settings.connectionStatus")}</dt>
              <dd>
                {settingsIsClientMode
                  ? hostTestResult?.message ?? i18n.t("settings.notChecked")
                  : hostStatus?.message ??
                    i18n.t("settings.sharingNotStarted")}
              </dd>
            </div>
            <div>
              <dt>{i18n.t("settings.otherComputers")}</dt>
              <dd>
                {status?.runtime.mode === "host"
                  ? i18n.t("settings.computerCount", {
                      count: clientConnections.length,
                    })
                  : "-"}
              </dd>
            </div>
            <div>
              <dt>{i18n.t("settings.lastChecked")}</dt>
              <dd>{clientConnectionCheckedAt ?? "-"}</dd>
            </div>
          </dl>

          {hostStatus?.pairCode && status?.runtime.mode === "host" ? (
            <div className="settings-inline-note">
              <strong>
                {i18n.t("settings.currentPairCode", {
                  code: hostStatus.pairCode,
                })}
              </strong>
              <span>{i18n.t("settings.pairCodeHint")}</span>
            </div>
          ) : null}

          {settingsIsClientMode && hostTestResult?.ok === false ? (
            <div className="settings-inline-note warning">
              <strong>
                {status?.runtime.clientToken
                  ? i18n.t("settings.hostConnectionAbnormal")
                  : i18n.t("settings.hostNotConnected")}
              </strong>
              <span>{hostTestResult.message}</span>
            </div>
          ) : null}

          {status?.runtime.mode === "standalone" ? (
            <p className="settings-footnote">
              {i18n.t("settings.standaloneFootnote")}
            </p>
          ) : null}

          {status?.runtime.mode === "host" ? (
            <div className="settings-compact-table">
              <div className="settings-compact-table-title">
                {i18n.t("settings.connectedClients")}
              </div>
              <table>
                <thead>
                  <tr>
                    <th>{i18n.t("settings.clientName")}</th>
                    <th>{i18n.t("settings.clientDevice")}</th>
                    <th>{i18n.t("settings.clientIp")}</th>
                    <th>{i18n.t("settings.clientVersion")}</th>
                    <th>{i18n.t("settings.clientStatus")}</th>
                    <th>{i18n.t("settings.clientLastSeen")}</th>
                    <th>{i18n.t("settings.clientActions")}</th>
                  </tr>
                </thead>
                <tbody>
                  {clientConnections.map((client) => (
                    <tr key={client.id}>
                      <td>{client.clientName}</td>
                      <td>{client.clientDeviceId}</td>
                      <td>{client.clientIp}</td>
                      <td>{client.appVersion}</td>
                      <td>
                        <span className="status enabled">{client.status}</span>
                      </td>
                      <td>{client.lastSeenAt}</td>
                      <td className="row-actions">
                        <button
                          className="ghost-button"
                          disabled={!canManage || isWorking}
                          type="button"
                          onClick={() => void onRemoveClientConnection(client)}
                        >
                          {i18n.t("settings.removeClient")}
                        </button>
                      </td>
                    </tr>
                  ))}
                  {clientConnections.length === 0 ? <EmptyRow colSpan={7} /> : null}
                </tbody>
              </table>
            </div>
          ) : null}
        </article>
      </div>

  );
}
