import { useEffect } from "react";
import { emit, listen } from "@tauri-apps/api/event";
import type { AppearanceSettings, LiquidGlassStyle, ThemeMode } from "../../entities/runtime";
import type { LocaleCode } from "../../i18n";

const STORAGE_KEY = "aster.appearance";
const CHANGED_EVENT = "appearance:changed";

export const defaultAppearanceSettings: AppearanceSettings = {
  themeMode: "auto", liquidGlassStyle: "tinted", accentColor: "#2f6dff", locale: "zh-CN",
};

function resolveTheme(mode: ThemeMode): "light" | "dark" {
  if (mode === "dark" || mode === "light") return mode;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function resolveAccentContrast(color: string) {
  const normalized = color.replace("#", "");
  if (!/^[0-9a-fA-F]{6}$/.test(normalized)) return "#ffffff";
  const red = Number.parseInt(normalized.slice(0, 2), 16) / 255;
  const green = Number.parseInt(normalized.slice(2, 4), 16) / 255;
  const blue = Number.parseInt(normalized.slice(4, 6), 16) / 255;
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue > 0.66 ? "#101114" : "#ffffff";
}

export function normalizeAppearanceSettings(value: Partial<AppearanceSettings> | null): AppearanceSettings {
  const themeMode: ThemeMode = value?.themeMode === "light" || value?.themeMode === "dark" ? value.themeMode : "auto";
  const liquidGlassStyle: LiquidGlassStyle = value?.liquidGlassStyle === "transparent" ? "transparent" : "tinted";
  const accentColor = typeof value?.accentColor === "string" && /^#[0-9a-fA-F]{6}$/.test(value.accentColor)
    ? value.accentColor.toLowerCase() : defaultAppearanceSettings.accentColor;
  const locale: LocaleCode = value?.locale === "en-US" ? "en-US" : "zh-CN";
  return { themeMode, liquidGlassStyle, accentColor, locale };
}

export function loadAppearanceSettings() {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    return normalizeAppearanceSettings(raw ? JSON.parse(raw) as Partial<AppearanceSettings> : null);
  } catch {
    return defaultAppearanceSettings;
  }
}

function applyAppearanceSettings(settings: AppearanceSettings) {
  const root = document.documentElement;
  root.dataset.theme = resolveTheme(settings.themeMode);
  root.dataset.glassStyle = settings.liquidGlassStyle;
  root.style.setProperty("--accent", settings.accentColor);
  root.style.setProperty("--accent-contrast", resolveAccentContrast(settings.accentColor));
}

export function useSyncedAppearanceSettings(settings?: AppearanceSettings) {
  useEffect(() => {
    const appearance = settings ?? loadAppearanceSettings();
    applyAppearanceSettings(appearance);
    document.documentElement.lang = appearance.locale;
    if (settings) window.localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  }, [settings]);
  useEffect(() => {
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handleChange = () => applyAppearanceSettings(settings ?? loadAppearanceSettings());
    mediaQuery.addEventListener("change", handleChange);
    return () => mediaQuery.removeEventListener("change", handleChange);
  }, [settings]);
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<AppearanceSettings>(CHANGED_EVENT, (event) => {
      const appearance = normalizeAppearanceSettings(event.payload);
      applyAppearanceSettings(appearance);
      document.documentElement.lang = appearance.locale;
    }).then((nextUnlisten) => { unlisten = nextUnlisten; });
    return () => unlisten?.();
  }, []);
}

export function useBroadcastAppearanceSettings(settings: AppearanceSettings) {
  useEffect(() => { void emit(CHANGED_EVENT, settings); }, [settings]);
}
