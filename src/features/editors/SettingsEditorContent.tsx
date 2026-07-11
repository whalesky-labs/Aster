import { invoke } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import type { AppStatus, ClientConnectionInfo, HostServiceStatus, RuntimeConfig } from "../../entities/runtime";
import { createI18n } from "../../i18n";
import type { EditorKind } from "../../shared/lib/editorWindows";
import { formatError } from "../../shared/lib/appRuntime";
import { loadAppearanceSettings } from "../settings/appearance";
import { SoftwareUpdateWindow } from "../settings/SoftwareUpdateWindow";
import { ConnectionWizard } from "../connections/ConnectionWizard";
import { BusinessSettingsEditor } from "./BusinessSettingsEditor";
import { ClientConnectionEditor, ClientPairingEditor } from "./ConnectionEditors";
import { RestoreBackupEditor, SecondBackupDirEditor } from "./BackupEditors";
import { ChangePasswordEditor, PasswordResetEditor } from "./PasswordEditors";
import type { EditorWindowController } from "./useEditorWindowController";

export function renderSettingsEditorContent({
  controller, editor, params,
}: { controller: EditorWindowController; editor: EditorKind; params: URLSearchParams }) {
  const {
    checkEditorUpdate, clientConnections, downloadAndInstallEditorUpdate, hostStatus,
    isLoading, isSaving, runEditorAction, runSettingsEditorAction, setClientConnections,
    setError, setHostStatus, setIsSaving, setNotice, setStatus, status, systemSettings,
    updateState,
  } = controller;
  if (editor === "changePassword") {
    return (
      <ChangePasswordEditor
        disabled={isSaving || isLoading}
        onSave={(request) =>
          runSettingsEditorAction("密码已修改", () =>
            invoke("change_password", { request }),
          )
        }
      />
    );
  } else if (editor === "passwordReset") {
    return (
      <PasswordResetEditor
        disabled={isSaving || isLoading}
        i18n={createI18n(loadAppearanceSettings().locale)}
        onRequestCode={async (username) => {
          setIsSaving(true);
          try {
            setError(null);
            const result = await invoke<{
              maskedEmail: string;
              expiresMinutes: number;
            }>("request_password_reset_code", {
              request: { username },
            });
            setNotice(
              `验证码已发送至 ${result.maskedEmail}，${result.expiresMinutes} 分钟内有效。`,
            );
          } catch (err) {
            setError(formatError(err));
            throw err;
          } finally {
            setIsSaving(false);
          }
        }}
        onReset={(request) =>
          runEditorAction({ editor, message: "密码已重置，请使用新密码登录" }, () =>
            invoke("reset_password_with_code", { request }),
          )
        }
      />
    );
  } else if (editor === "businessSettings") {
    return (
      <BusinessSettingsEditor
        disabled={isSaving || isLoading || !systemSettings}
        localDirectoriesOnly={status?.runtime.mode === "client"}
        settings={systemSettings}
        onSave={(request) =>
          runSettingsEditorAction("系统设置已保存", () =>
            invoke("save_system_settings", { request }),
          )
        }
      />
    );
  } else if (editor === "softwareUpdate") {
    return (
      <SoftwareUpdateWindow
        disabled={isSaving || isLoading}
        i18n={createI18n(loadAppearanceSettings().locale)}
        status={status}
        updateState={updateState}
        onCheck={() => checkEditorUpdate()}
        onInstall={downloadAndInstallEditorUpdate}
        onRestart={() => relaunch()}
      />
    );
  } else if (editor === "clientConnection") {
    return (
      <ClientConnectionEditor
        disabled={isSaving || isLoading || !status}
        status={status}
        onDiscover={(hostPort) => invoke("discover_hosts", { hostPort })}
        onSave={(hostAddress, hostPort) =>
          runSettingsEditorAction("客户端连接配置已保存", () =>
            invoke("save_client_config", {
              request: { hostAddress, hostPort },
            }),
          )
        }
        onTest={(hostAddress, hostPort) =>
          invoke("test_host_connection", {
            request: { hostAddress, hostPort },
          })
        }
      />
    );
  } else if (editor === "clientPairing") {
    return (
      <ClientPairingEditor
        disabled={isSaving || isLoading || !status}
        status={status}
        onSave={(request) =>
          runSettingsEditorAction("客户端已完成主机配对", () =>
            invoke("pair_with_host", { request }),
          )
        }
      />
    );
  } else if (editor === "connectionWizard") {
    return (
      <ConnectionWizard
        clientOnly={params.get("clientOnly") === "1"}
        clientConnections={clientConnections}
        disabled={isSaving || isLoading || !status}
        hostStatus={hostStatus}
        status={status}
        onDiscover={(hostPort) => invoke("discover_hosts", { hostPort })}
        onEnableHost={async () => {
          setIsSaving(true);
          try {
            setError(null);
            await invoke<RuntimeConfig>("set_runtime_mode", { mode: "host" });
            const nextHostStatus =
              await invoke<HostServiceStatus>("start_host_service");
            const [nextStatus, nextClients] = await Promise.all([
              invoke<AppStatus>("get_app_status"),
              invoke<ClientConnectionInfo[]>("list_client_connections"),
            ]);
            setStatus(nextStatus);
            setHostStatus(nextHostStatus);
            setClientConnections(nextClients);
            return nextHostStatus;
          } finally {
            setIsSaving(false);
          }
        }}
        onFinish={(message) =>
          runSettingsEditorAction(
            params.get("clientOnly") === "1"
              ? "已连接主电脑，请使用主机账号登录"
              : message,
            () => Promise.resolve(),
          )
        }
        onPair={async (request) => {
          setIsSaving(true);
          try {
            setError(null);
            await invoke<RuntimeConfig>("save_client_config", {
              request: {
                hostAddress: request.hostAddress,
                hostPort: request.hostPort,
              },
            });
            const nextRuntime = await invoke<RuntimeConfig>("pair_with_host", {
              request: {
                pairCode: request.pairCode,
                clientName: request.clientName,
                clientDeviceId: request.clientDeviceId,
              },
            });
            const nextStatus = await invoke<AppStatus>("get_app_status");
            setStatus(nextStatus);
            return nextRuntime;
          } finally {
            setIsSaving(false);
          }
        }}
        onRefreshHost={async () => {
          const [nextStatus, nextHostStatus] = await Promise.all([
            invoke<AppStatus>("get_app_status"),
            invoke<HostServiceStatus>("get_host_service_status"),
          ]);
          setStatus(nextStatus);
          setHostStatus(nextHostStatus);
          if (nextStatus.runtime.mode === "host") {
            setClientConnections(
              await invoke<ClientConnectionInfo[]>("list_client_connections"),
            );
          } else {
            setClientConnections([]);
          }
        }}
        onTest={(hostAddress, hostPort) =>
          invoke("test_host_connection", {
            request: { hostAddress, hostPort },
          })
        }
      />
    );
  } else if (editor === "secondBackupDir") {
    return (
      <SecondBackupDirEditor
        disabled={isSaving || isLoading || !status}
        status={status}
        onSave={(path) =>
          runSettingsEditorAction("第二备份目录已保存", () =>
            invoke("set_second_backup_dir", { request: { path } }),
          )
        }
      />
    );
  } else if (editor === "restoreBackup") {
    return (
      <RestoreBackupEditor
        disabled={isSaving || isLoading || !status}
        status={status}
        onPreview={(backupFile) =>
          invoke("preview_restore_backup", { backupFile })
        }
        onRestore={(request) =>
          runSettingsEditorAction("备份已恢复，数据库健康检查通过", () =>
            invoke("restore_backup", { request }),
          )
        }
      />
    );
  }
  return null;
}
