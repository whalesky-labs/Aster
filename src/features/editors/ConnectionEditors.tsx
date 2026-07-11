import { useState } from "react";
import type { AppStatus, HostConnectionTestResult, HostDiscoveryResult } from "../../entities/runtime";
import { Field } from "../../shared/ui/DataTable";
import { EditorForm } from "../../shared/ui/EditorForm";

export function ClientConnectionEditor({
  disabled,
  onDiscover,
  onSave,
  onTest,
  status,
}: {
  disabled: boolean;
  onDiscover: (hostPort: number) => Promise<HostDiscoveryResult[]>;
  onSave: (hostAddress: string, hostPort: number) => Promise<void>;
  onTest: (
    hostAddress: string,
    hostPort: number,
  ) => Promise<HostConnectionTestResult>;
  status: AppStatus | null;
}) {
  const [hostAddress, setHostAddress] = useState(
    status?.runtime.hostAddress ?? "127.0.0.1",
  );
  const [hostPort, setHostPort] = useState(status?.runtime.hostPort ?? 17871);
  const [testResult, setTestResult] = useState<HostConnectionTestResult | null>(null);
  const [hosts, setHosts] = useState<HostDiscoveryResult[]>([]);

  return (
    <EditorForm
      disabled={disabled || !hostAddress.trim()}
      saveLabel="保存客户端连接"
      onSave={() => onSave(hostAddress, hostPort)}
    >
      <Field label="主机地址">
        <input
          autoFocus
          value={hostAddress}
          onChange={(event) => setHostAddress(event.target.value)}
          placeholder="主机 IP 或主机名"
        />
      </Field>
      <Field label="主机端口">
        <input
          max="65535"
          min="1024"
          type="number"
          value={hostPort}
          onChange={(event) => setHostPort(Number(event.target.value))}
        />
      </Field>
      <button
        className="ghost-button"
        disabled={disabled}
        type="button"
        onClick={async () => setTestResult(await onTest(hostAddress, hostPort))}
      >
        测试连接
      </button>
      <button
        className="ghost-button"
        disabled={disabled}
        type="button"
        onClick={async () => setHosts(await onDiscover(hostPort))}
      >
        发现主机
      </button>
      {testResult ? (
        <div className={testResult.ok ? "settings-result success" : "settings-result warning"}>
          <strong>{testResult.message}</strong>
          <span>{testResult.appName ?? "-"} {testResult.appVersion ?? ""}</span>
          <span>Schema：{testResult.schemaVersion ?? "-"}</span>
        </div>
      ) : null}
      {hosts.length > 0 ? (
        <div className="discovery-list">
          {hosts.map((host) => (
            <button
              className="discovery-item"
              key={`${host.hostAddress}:${host.hostPort}`}
              type="button"
              onClick={() => {
                setHostAddress(host.hostAddress);
                setHostPort(host.hostPort);
              }}
            >
              <strong>{host.hostAddress}:{host.hostPort}</strong>
              <span>{host.appName} {host.appVersion} · Schema {host.schemaVersion}</span>
            </button>
          ))}
        </div>
      ) : null}
    </EditorForm>
  );
}

export function ClientPairingEditor({
  disabled,
  onSave,
  status,
}: {
  disabled: boolean;
  onSave: (request: {
    pairCode: string;
    clientName: string;
    clientDeviceId: string;
  }) => Promise<void>;
  status: AppStatus | null;
}) {
  const [pairCode, setPairCode] = useState("");
  const [clientName, setClientName] = useState("Aster 客户端");
  const [clientDeviceId, setClientDeviceId] = useState(
    status?.runtime.clientDeviceId ?? "",
  );
  return (
    <EditorForm
      disabled={disabled || pairCode.length !== 12 || !clientName.trim()}
      saveLabel="完成配对"
      onSave={() => onSave({ pairCode, clientName, clientDeviceId })}
    >
      <Field label="配对码">
        <input
          autoFocus
          inputMode="numeric"
          maxLength={12}
          value={pairCode}
          onChange={(event) => setPairCode(event.target.value)}
        />
      </Field>
      <Field label="客户端名称">
        <input
          value={clientName}
          onChange={(event) => setClientName(event.target.value)}
        />
      </Field>
      <Field label="设备 ID">
        <input
          value={clientDeviceId}
          onChange={(event) => setClientDeviceId(event.target.value)}
        />
      </Field>
    </EditorForm>
  );
}
