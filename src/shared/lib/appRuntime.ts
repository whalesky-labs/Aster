import { invoke } from "@tauri-apps/api/core";
import { emitTo } from "@tauri-apps/api/event";
import { check, type CheckOptions } from "@tauri-apps/plugin-updater";
import type { ProxyCandidate } from "../../entities/runtime";
import type { EditorKind } from "./editorWindows";

export type EditorSavedPayload = {
  editor: EditorKind;
  message?: string;
  documentType?: "inbound" | "outbound";
  stocktakeId?: string;
};

export function formatError(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  if (message.includes("Cannot read properties of undefined") && message.includes("invoke")) {
    return "当前页面需要在 Aster 桌面客户端中运行；浏览器预览只能查看界面壳，不能访问本地 SQLite 和 Tauri 命令。";
  }
  return message;
}

export async function checkAppUpdateWithFallback() {
  let candidates: ProxyCandidate[] = [];
  try {
    candidates = (await invoke<ProxyCandidate[]>("get_system_proxy_candidates"))
      .filter((candidate) => candidate.url.trim());
  } catch {
    candidates = [];
  }
  const attempts: Array<{ label: string; options?: CheckOptions; proxy?: string }> = [{ label: "直连" }];
  candidates.forEach((candidate) => attempts.push({
    label: candidate.label, options: { proxy: candidate.url }, proxy: candidate.url,
  }));
  const errors: string[] = [];
  for (const attempt of attempts) {
    try {
      const update = await check(attempt.options);
      return { attemptLabel: attempt.label, proxy: attempt.proxy ?? null, update };
    } catch (error) {
      errors.push(`${attempt.label}：${formatError(error)}`);
    }
  }
  throw new Error([
    "无法连接更新源，已尝试直连和本机代理。",
    "如果当前电脑已开启 VPN，请确认 VPN 代理允许桌面应用访问 GitHub Releases。",
    errors[errors.length - 1] ?? "",
  ].filter(Boolean).join("\n"));
}

export async function notifyEditorSaved(payload: EditorSavedPayload) {
  await emitTo("main", "editor:saved", payload);
}
