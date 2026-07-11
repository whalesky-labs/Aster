export type ConnectionWizardStep =
  | "role" | "hostConfirm" | "hostReady" | "discover" | "manual" | "pair" | "clientReady";

declare global { interface Window { __TAURI_OS_PLUGIN_INTERNALS__?: { platform?: string }; } }

export function detectDesktopPlatform() {
  const platform = window.__TAURI_OS_PLUGIN_INTERNALS__?.platform;
  if (platform) return platform.toLowerCase();
  if (navigator.userAgent.includes("Windows")) return "windows";
  if (navigator.userAgent.includes("Mac")) return "macos";
  return "unknown";
}
export function formatError(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
export function wizardStepTitle(step: ConnectionWizardStep) {
  if (step === "hostConfirm") return "把这台电脑设为主电脑";
  if (step === "hostReady") return "主电脑已开启";
  if (step === "discover") return "搜索主电脑";
  if (step === "manual") return "手动连接主电脑";
  if (step === "pair") return "输入配对码";
  if (step === "clientReady") return "连接完成";
  return "这台电脑要怎么使用？";
}

export function defaultClientName(platform: string) {
  if (platform === "windows") return "Windows 电脑";
  if (platform === "macos") return "macOS 电脑";
  return "Aster 电脑";
}

export function defaultClientDeviceId() {
  const stored = window.localStorage.getItem("aster.clientDeviceId");
  if (stored) return stored;
  const generated = `device-${Date.now().toString(36)}-${Math.random()
    .toString(36)
    .slice(2, 8)}`;
  window.localStorage.setItem("aster.clientDeviceId", generated);
  return generated;
}
