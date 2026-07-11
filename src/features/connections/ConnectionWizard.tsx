import { useEffect, useState } from "react";
import type {
  AppStatus,
  ClientConnectionInfo,
  HostConnectionTestResult,
  HostDiscoveryResult,
  HostServiceStatus,
  RuntimeConfig,
} from "../../entities/runtime";
import { Field } from "../../shared/ui/DataTable";
import { WizardFooterActions } from "./WizardFooterActions";
import {
  type ConnectionWizardStep,
  defaultClientDeviceId, defaultClientName, detectDesktopPlatform, formatError,
  wizardStepTitle,
} from "./connectionWizardUtils";

export function ConnectionWizard({
  clientOnly = false,
  clientConnections,
  disabled,
  hostStatus,
  onDiscover,
  onEnableHost,
  onFinish,
  onPair,
  onRefreshHost,
  onTest,
  status,
}: {
  clientOnly?: boolean;
  clientConnections: ClientConnectionInfo[];
  disabled: boolean;
  hostStatus: HostServiceStatus | null;
  onDiscover: (hostPort: number) => Promise<HostDiscoveryResult[]>;
  onEnableHost: () => Promise<HostServiceStatus>;
  onFinish: (message: string) => Promise<void>;
  onPair: (request: {
    hostAddress: string;
    hostPort: number;
    pairCode: string;
    clientName: string;
    clientDeviceId: string;
  }) => Promise<RuntimeConfig>;
  onRefreshHost: () => Promise<void>;
  onTest: (
    hostAddress: string,
    hostPort: number,
  ) => Promise<HostConnectionTestResult>;
  status: AppStatus | null;
}) {
  const [step, setStep] = useState<ConnectionWizardStep>(
    clientOnly ? "discover" : "role",
  );
  const [hosts, setHosts] = useState<HostDiscoveryResult[]>([]);
  const [selectedHost, setSelectedHost] = useState<HostDiscoveryResult | null>(
    null,
  );
  const [hostAddress, setHostAddress] = useState(
    status?.runtime.hostAddress ?? "",
  );
  const [hostPort, setHostPort] = useState(status?.runtime.hostPort ?? 17871);
  const [pairCode, setPairCode] = useState("");
  const [clientName, setClientName] = useState(() =>
    defaultClientName(detectDesktopPlatform()),
  );
  const [clientDeviceId, setClientDeviceId] = useState(
    status?.runtime.clientDeviceId || defaultClientDeviceId(),
  );
  const [testResult, setTestResult] =
    useState<HostConnectionTestResult | null>(null);
  const [isBusy, setIsBusy] = useState(false);
  const [localError, setLocalError] = useState<string | null>(null);
  const effectiveHostStatus = hostStatus;
  const effectiveHostAddress =
    selectedHost?.hostAddress || hostAddress.trim() || "";
  const effectiveHostPort = selectedHost?.hostPort || hostPort || 17871;

  async function discover() {
    setStep("discover");
    setIsBusy(true);
    setLocalError(null);
    setTestResult(null);
    try {
      const results = await onDiscover(hostPort || 17871);
      setHosts(results);
      if (results.length === 1) {
        setSelectedHost(results[0]);
      }
    } catch (err) {
      setHosts([]);
      setLocalError(formatError(err));
    } finally {
      setIsBusy(false);
    }
  }

  useEffect(() => {
    if (!clientOnly) return;
    void discover();
  }, [clientOnly]);

  async function enableHost() {
    setIsBusy(true);
    setLocalError(null);
    try {
      await onEnableHost();
      setStep("hostReady");
    } catch (err) {
      setLocalError(formatError(err));
    } finally {
      setIsBusy(false);
    }
  }

  async function testManualHost() {
    if (!hostAddress.trim()) return;
    setIsBusy(true);
    setLocalError(null);
    try {
      const result = await onTest(hostAddress.trim(), hostPort || 17871);
      setTestResult(result);
      if (result.ok) {
        setSelectedHost({
          hostAddress: hostAddress.trim(),
          hostPort: hostPort || 17871,
          appName: result.appName ?? "Aster",
          appVersion: result.appVersion ?? "-",
          schemaVersion: result.schemaVersion ?? 0,
          message: result.message,
        });
        setStep("pair");
      }
    } catch (err) {
      setTestResult({
        ok: false,
        message: formatError(err),
        appName: null,
        appVersion: null,
        schemaVersion: null,
      });
    } finally {
      setIsBusy(false);
    }
  }

  async function pairHost() {
    if (!effectiveHostAddress || pairCode.length !== 12 || !clientName.trim()) {
      return;
    }
    setIsBusy(true);
    setLocalError(null);
    try {
      await onPair({
        hostAddress: effectiveHostAddress,
        hostPort: effectiveHostPort,
        pairCode,
        clientName: clientName.trim(),
        clientDeviceId: clientDeviceId.trim() || defaultClientDeviceId(),
      });
      setStep("clientReady");
    } catch (err) {
      setLocalError(formatError(err));
    } finally {
      setIsBusy(false);
    }
  }

  const footerActions = (
    <WizardFooterActions
      clientOnly={clientOnly} disabled={disabled} enableHost={enableHost}
      hostAddress={hostAddress} hostsCount={hosts.length} isBusy={isBusy}
      onFinish={onFinish} onRefreshHost={onRefreshHost} pairCode={pairCode}
      pairHost={pairHost} clientName={clientName} setStep={setStep}
      step={step} testManualHost={testManualHost}
    />
  );
  return (
    <div className="connection-wizard">
      <div className="wizard-header">
        <span>连接向导</span>
        <h2>{wizardStepTitle(step)}</h2>
      </div>

      <div className="wizard-content">
        {localError ? <div className="error-banner">{localError}</div> : null}

        {step === "role" && !clientOnly ? (
          <div className="wizard-choice-grid">
            <button
              className="wizard-choice"
              disabled={disabled || isBusy}
              type="button"
              onClick={() => setStep("hostConfirm")}
            >
              <strong>这台作为主电脑</strong>
              <span>正式库存数据保存在这台电脑，其他电脑连接过来一起使用。</span>
            </button>
            <button
              className="wizard-choice"
              disabled={disabled || isBusy}
              type="button"
              onClick={() => void discover()}
            >
              <strong>连接到主电脑</strong>
              <span>这台电脑连接已有主电脑，共用同一套库存数据。</span>
            </button>
          </div>
        ) : null}

      {step === "hostConfirm" && !clientOnly ? (
        <div className="wizard-panel">
          <p>
            正式库存数据将保存在这台电脑。其他电脑需要输入这台电脑显示的配对码后才能连接。
          </p>
          <div className="wizard-actions">
            <button
              className="ghost-button"
              disabled={isBusy}
              type="button"
              onClick={() => setStep("role")}
            >
              返回
            </button>
            <button
              className="primary-button"
              disabled={disabled || isBusy}
              type="button"
              onClick={() => void enableHost()}
            >
              {isBusy ? "开启中..." : "开启共享"}
            </button>
          </div>
        </div>
      ) : null}

      {step === "hostReady" && !clientOnly ? (
        <div className="wizard-panel">
          <div className="pair-code-card">
            <span>给其他电脑输入的配对码</span>
            <strong>{effectiveHostStatus?.pairCode ?? "------"}</strong>
          </div>
          <dl className="wizard-summary">
            <div>
              <dt>共享状态</dt>
              <dd>{effectiveHostStatus?.message ?? "主电脑共享已开启"}</dd>
            </div>
            <div>
              <dt>连接地址</dt>
              <dd>
                {effectiveHostStatus?.running
                  ? `${effectiveHostStatus.bindAddress}:${effectiveHostStatus.port}`
                  : "-"}
              </dd>
            </div>
            <div>
              <dt>已连接其他电脑</dt>
              <dd>{clientConnections.length} 台</dd>
            </div>
          </dl>
          <div className="wizard-actions">
            <button
              className="ghost-button"
              disabled={isBusy}
              type="button"
              onClick={() => void onRefreshHost()}
            >
              刷新状态
            </button>
            <button
              className="primary-button"
              disabled={disabled || isBusy}
              type="button"
              onClick={() => void onFinish("这台电脑已开启主电脑共享")}
            >
              完成
            </button>
          </div>
        </div>
      ) : null}

      {step === "discover" ? (
        <div className="wizard-panel">
          <div className="wizard-toolbar">
            <div>
              <strong>局域网主电脑</strong>
              <p>
                {isBusy
                  ? "正在搜索局域网内的主电脑..."
                  : hosts.length > 0
                    ? "选择要连接的主电脑，然后输入主机上的配对码。"
                    : "没有找到主电脑，请确认主电脑已开启共享。"}
              </p>
            </div>
            <button
              className="ghost-button"
              disabled={disabled || isBusy}
              type="button"
              onClick={() => void discover()}
            >
              重新搜索
            </button>
          </div>
          {hosts.length > 0 ? (
            <div className="discovery-list">
              {hosts.map((host) => (
                <button
                  className={
                    selectedHost?.hostAddress === host.hostAddress &&
                    selectedHost?.hostPort === host.hostPort
                      ? "discovery-item selected"
                      : "discovery-item"
                  }
                  key={`${host.hostAddress}:${host.hostPort}`}
                  type="button"
                  onClick={() => {
                    setSelectedHost(host);
                    setHostAddress(host.hostAddress);
                    setHostPort(host.hostPort);
                    setStep("pair");
                  }}
                >
                  <span className="discovery-host-main">
                    <strong>{host.hostAddress}</strong>
                    <em>{host.hostPort}</em>
                  </span>
                  <span className="discovery-host-meta">
                    {host.appName} {host.appVersion} · Schema{" "}
                    {host.schemaVersion}
                  </span>
                  <span className="discovery-host-status">{host.message}</span>
                </button>
              ))}
            </div>
          ) : null}
          <div className="wizard-actions">
            <button
              className="ghost-button"
              disabled={isBusy}
              type="button"
              onClick={() => setStep(clientOnly ? "manual" : "role")}
            >
              返回
            </button>
            <button
              className="ghost-button"
              disabled={isBusy}
              type="button"
              onClick={() => setStep("manual")}
            >
              手动输入地址
            </button>
          </div>
        </div>
      ) : null}

      {step === "manual" ? (
        <div className="wizard-panel">
          <Field label="主电脑地址">
            <input
              autoFocus
              disabled={isBusy}
              placeholder="例如 192.168.1.20"
              value={hostAddress}
              onChange={(event) => setHostAddress(event.target.value)}
            />
          </Field>
          <Field label="端口">
            <input
              disabled={isBusy}
              max="65535"
              min="1024"
              type="number"
              value={hostPort}
              onChange={(event) => setHostPort(Number(event.target.value))}
            />
          </Field>
          {testResult ? (
            <div
              className={
                testResult.ok
                  ? "settings-result success"
                  : "settings-result warning"
              }
            >
              <strong>{testResult.message}</strong>
              <span>
                {testResult.appName ?? "-"} {testResult.appVersion ?? ""}
              </span>
            </div>
          ) : null}
          <div className="wizard-actions">
            <button
              className="ghost-button"
              disabled={isBusy}
              type="button"
              onClick={() => setStep("discover")}
            >
              返回搜索
            </button>
            <button
              className="primary-button"
              disabled={disabled || isBusy || !hostAddress.trim()}
              type="button"
              onClick={() => void testManualHost()}
            >
              测试并继续
            </button>
          </div>
        </div>
      ) : null}

      {step === "pair" ? (
        <div className="wizard-panel">
          <dl className="wizard-summary">
            <div>
              <dt>主电脑</dt>
              <dd>
                {effectiveHostAddress}:{effectiveHostPort}
              </dd>
            </div>
          </dl>
          <Field label="配对码">
            <input
              autoFocus
              disabled={isBusy}
              inputMode="numeric"
              maxLength={12}
              placeholder="输入主电脑显示的 12 位数字"
              value={pairCode}
              onChange={(event) =>
                setPairCode(event.target.value.replace(/\D/g, "").slice(0, 6))
              }
            />
          </Field>
          <Field label="这台电脑名称">
            <input
              disabled={isBusy}
              value={clientName}
              onChange={(event) => setClientName(event.target.value)}
            />
          </Field>
          <Field label="设备标识">
            <input
              disabled={isBusy}
              value={clientDeviceId}
              onChange={(event) => setClientDeviceId(event.target.value)}
            />
          </Field>
          <div className="wizard-actions">
            <button
              className="ghost-button"
              disabled={isBusy}
              type="button"
              onClick={() => setStep(hosts.length > 0 ? "discover" : "manual")}
            >
              返回
            </button>
            <button
              className="primary-button"
              disabled={
                disabled || isBusy || pairCode.length !== 12 || !clientName.trim()
              }
              type="button"
              onClick={() => void pairHost()}
            >
              {isBusy ? "连接中..." : "连接"}
            </button>
          </div>
        </div>
      ) : null}

        {step === "clientReady" ? (
          <div className="wizard-panel">
            <div className="settings-result success">
              <strong>已连接到主电脑</strong>
              <span>
                {effectiveHostAddress}:{effectiveHostPort}
              </span>
              <span>以后打开应用会自动使用主电脑上的库存数据。</span>
            </div>
            <div className="wizard-actions">
              <button
                className="primary-button"
                disabled={disabled || isBusy}
                type="button"
                onClick={() => void onFinish("已连接到主电脑")}
              >
                完成
              </button>
            </div>
          </div>
        ) : null}
      </div>
      {footerActions ? (
        <div className="wizard-footer-actions">{footerActions}</div>
      ) : null}
    </div>
  );
}
