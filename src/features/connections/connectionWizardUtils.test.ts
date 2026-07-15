import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  defaultClientDeviceId,
  defaultClientName,
  detectDesktopPlatform,
  formatError,
  wizardStepTitle,
} from "./connectionWizardUtils";

describe("connection wizard utilities", () => {
  beforeEach(() => {
    localStorage.clear();
    delete window.__TAURI_OS_PLUGIN_INTERNALS__;
  });

  it("prefers the native desktop platform", () => {
    window.__TAURI_OS_PLUGIN_INTERNALS__ = { platform: "Windows" };
    expect(detectDesktopPlatform()).toBe("windows");
    expect(defaultClientName("windows")).toBe("Windows 电脑");
    expect(defaultClientName("macos")).toBe("macOS 电脑");
    expect(defaultClientName("linux")).toBe("Aster 电脑");
  });

  it("falls back to the browser user agent when native platform data is absent", () => {
    Object.defineProperty(navigator, "userAgent", { configurable: true, value: "Windows NT 10.0" });
    expect(detectDesktopPlatform()).toBe("windows");
    Object.defineProperty(navigator, "userAgent", { configurable: true, value: "Macintosh" });
    expect(detectDesktopPlatform()).toBe("macos");
    Object.defineProperty(navigator, "userAgent", { configurable: true, value: "Linux" });
    expect(detectDesktopPlatform()).toBe("unknown");
  });

  it("persists a stable generated device id", () => {
    vi.spyOn(Date, "now").mockReturnValue(1_700_000_000_000);
    vi.spyOn(Math, "random").mockReturnValue(0.25);
    const first = defaultClientDeviceId();
    expect(defaultClientDeviceId()).toBe(first);
    expect(first).toMatch(/^device-/);
    vi.restoreAllMocks();
  });

  it("provides a title for each wizard state", () => {
    expect(wizardStepTitle("role")).toContain("怎么使用");
    expect(wizardStepTitle("hostReady")).toContain("已开启");
    expect(wizardStepTitle("clientReady")).toContain("完成");
    expect(wizardStepTitle("hostConfirm")).toContain("主电脑");
    expect(wizardStepTitle("discover")).toContain("搜索");
    expect(wizardStepTitle("manual")).toContain("手动");
    expect(wizardStepTitle("pair")).toContain("配对码");
    expect(formatError(new Error("failed"))).toBe("failed");
    expect(formatError("failed")).toBe("failed");
  });
});
