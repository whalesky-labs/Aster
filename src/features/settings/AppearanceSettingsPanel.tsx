import type { Dispatch, SetStateAction } from "react";
import type { I18n, LocaleCode } from "../../i18n";
import type { AppearanceSettings, LiquidGlassStyle, ThemeMode } from "../../entities/runtime";

const accentColors = ["#8f96a3", "#2f6dff", "#a65dd9", "#f062a8", "#ff6a57", "#ffb020", "#f5dd00", "#33c96f"] as const;
const labels: Record<string, string> = {
  "#8f96a3": "color.graphite", "#2f6dff": "color.blue", "#a65dd9": "color.purple",
  "#f062a8": "color.pink", "#ff6a57": "color.coral", "#ffb020": "color.amber",
  "#f5dd00": "color.lemon", "#33c96f": "color.green",
};

export function AppearanceSettingsPanel({ appearanceSettings, i18n, onAppearanceChange }: {
  appearanceSettings: AppearanceSettings;
  i18n: I18n;
  onAppearanceChange: Dispatch<SetStateAction<AppearanceSettings>>;
}) {
  const effectiveTheme = appearanceSettings.themeMode === "auto"
    ? (window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light")
    : appearanceSettings.themeMode;
  const glassPreviewThemeClass = effectiveTheme === "light" ? "preview-glass-light" : "preview-glass-dark";
  const accentColorLabel = (color: string) => i18n.t(labels[color] ?? color);
  return (
    <>
      <div className="settings-group">
        <h3 className="settings-group-title">{i18n.t("settings.appearance")}</h3>
        <article className="surface settings-block appearance-settings-block">
          <div className="setting-row">
            <div className="settings-meta">
              <span className="settings-label">{i18n.t("settings.themeMode")}</span>
              <p className="settings-hint">
                {i18n.t("settings.effectiveTheme", {
                  theme:
                    effectiveTheme === "dark"
                      ? i18n.t("settings.dark")
                      : i18n.t("settings.light"),
                })}
              </p>
            </div>
            <div className="setting-control">
              <div className="preview-grid preview-grid-3">
                {(["auto", "light", "dark"] as ThemeMode[]).map((mode) => (
                  <button
                    key={mode}
                    className={`preview-card ${appearanceSettings.themeMode === mode ? "active" : ""}`}
                    type="button"
                    onClick={() =>
                      onAppearanceChange((current) => ({
                        ...current,
                        themeMode: mode,
                      }))
                    }
                  >
                    <span
                      className={`preview-art preview-theme preview-theme-${mode}`}
                    />
                    <span className="preview-label">
                      {mode === "auto"
                        ? i18n.t("settings.followSystem")
                        : mode === "light"
                          ? i18n.t("settings.light")
                          : i18n.t("settings.dark")}
                    </span>
                  </button>
                ))}
              </div>
            </div>
          </div>

          <div className="setting-row">
            <div className="settings-meta">
              <span className="settings-label">Liquid Glass</span>
              <p className="settings-hint">
                {i18n.t("settings.liquidGlassHint")}
              </p>
            </div>
            <div className="setting-control">
              <div className="preview-grid preview-grid-2">
                {(["transparent", "tinted"] as LiquidGlassStyle[]).map(
                  (style) => (
                    <button
                      key={style}
                      className={`preview-card ${appearanceSettings.liquidGlassStyle === style ? "active" : ""}`}
                      type="button"
                      onClick={() =>
                        onAppearanceChange((current) => ({
                          ...current,
                          liquidGlassStyle: style,
                        }))
                      }
                    >
                      <span
                        className={`preview-art preview-glass preview-glass-${style} ${glassPreviewThemeClass}`}
                      />
                      <span className="preview-label">
                        {style === "transparent"
                          ? i18n.t("settings.transparent")
                          : i18n.t("settings.tinted")}
                      </span>
                    </button>
                  ),
                )}
              </div>
            </div>
          </div>

          <div className="setting-row">
            <div className="settings-meta">
              <span className="settings-label">
                {i18n.t("settings.interfaceLanguage")}
              </span>
            </div>
            <div className="setting-control setting-control-inline">
              <select
                value={appearanceSettings.locale}
                onChange={(event) =>
                  onAppearanceChange((current) => ({
                    ...current,
                    locale: event.target.value as LocaleCode,
                  }))
                }
              >
                <option value="zh-CN">简体中文</option>
                <option value="en-US">English</option>
              </select>
            </div>
          </div>
        </article>
      </div>

      <div className="settings-group settings-group-accent">
        <h3 className="settings-group-title">{i18n.t("settings.theme")}</h3>
        <article className="surface settings-block appearance-settings-block">
          <div className="setting-row setting-row-color">
            <div className="settings-meta">
              <span className="settings-label">{i18n.t("settings.color")}</span>
            </div>
            <div className="setting-control">
              <div className="color-row">
                {accentColors.map((color) => (
                  <div key={color} className="color-option">
                    <button
                      className={`color-dot ${appearanceSettings.accentColor.toLowerCase() === color ? "active" : ""}`}
                      style={{ background: color }}
                      type="button"
                      title={accentColorLabel(color)}
                      onClick={() =>
                        onAppearanceChange((current) => ({
                          ...current,
                          accentColor: color,
                        }))
                      }
                    />
                    {appearanceSettings.accentColor.toLowerCase() === color ? (
                      <span className="color-option-label">
                        {accentColorLabel(color)}
                      </span>
                    ) : null}
                  </div>
                ))}
              </div>
            </div>
          </div>
        </article>
      </div>

    </>
  );
}
