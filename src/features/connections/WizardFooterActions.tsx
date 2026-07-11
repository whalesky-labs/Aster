import type { Dispatch, SetStateAction } from "react";
import type { ConnectionWizardStep } from "./connectionWizardUtils";

export function WizardFooterActions(props: {
  clientOnly: boolean; disabled: boolean; enableHost: () => Promise<void>;
  hostAddress: string; hostsCount: number; isBusy: boolean;
  onFinish: (message: string) => Promise<void>; onRefreshHost: () => Promise<void>;
  pairCode: string; pairHost: () => Promise<void>; clientName: string;
  setStep: Dispatch<SetStateAction<ConnectionWizardStep>>; step: ConnectionWizardStep;
  testManualHost: () => Promise<void>;
}) {
  const { clientOnly, disabled, enableHost, hostAddress, hostsCount, isBusy, onFinish,
    onRefreshHost, pairCode, pairHost, clientName, setStep, step, testManualHost } = props;
  return (() => {
    if (step === "role" && !clientOnly) return null;
    if (step === "hostConfirm" && !clientOnly) {
      return (
        <>
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
        </>
      );
    }
    if (step === "hostReady" && !clientOnly) {
      return (
        <>
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
        </>
      );
    }
    if (step === "discover") {
      return (
        <>
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
        </>
      );
    }
    if (step === "manual") {
      return (
        <>
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
        </>
      );
    }
    if (step === "pair") {
      return (
        <>
          <button
            className="ghost-button"
            disabled={isBusy}
            type="button"
            onClick={() => setStep(hostsCount > 0 ? "discover" : "manual")}
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
        </>
      );
    }
    if (step === "clientReady") {
      return (
        <button
          className="primary-button"
          disabled={disabled || isBusy}
          type="button"
          onClick={() => void onFinish("已连接到主电脑")}
        >
          完成
        </button>
      );
    }
    return null;
  })();

}
