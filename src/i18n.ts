import { enUS } from "./i18n/en-US";
import { zhCN } from "./i18n/zh-CN";

export type LocaleCode = "zh-CN" | "en-US";

type TranslationParams = Record<string, string | number>;
export type TranslationDictionary = Record<string, string>;





const dictionaries: Record<LocaleCode, TranslationDictionary> = {
  "zh-CN": zhCN,
  "en-US": enUS,
};

export type I18n = ReturnType<typeof createI18n>;

function interpolate(template: string, params?: TranslationParams) {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (match, key) =>
    Object.prototype.hasOwnProperty.call(params, key)
      ? String(params[key])
      : match,
  );
}

function prefixedLabel(
  t: (key: string, params?: TranslationParams) => string,
  prefix: string,
  value: string,
) {
  const key = `${prefix}.${value}`;
  const translated = t(key);
  return translated === key ? value : translated;
}

export function createI18n(locale: LocaleCode) {
  const dictionary = dictionaries[locale] ?? dictionaries["zh-CN"];

  function t(key: string, params?: TranslationParams) {
    return interpolate(dictionary[key] ?? key, params);
  }

  return {
    locale,
    t,
    formatMoney(value: number) {
      return new Intl.NumberFormat(locale, {
        minimumFractionDigits: 2,
        maximumFractionDigits: 2,
      }).format(value);
    },
    modeLabel(value: string) {
      return prefixedLabel(t, "mode", value);
    },
    backupTypeLabel(value: string) {
      return prefixedLabel(t, "backupType", value);
    },
    auditActionLabel(value: string) {
      return prefixedLabel(t, "auditAction", value);
    },
    auditEntityLabel(value: string) {
      return prefixedLabel(t, "auditEntity", value);
    },
    approvalTypeLabel(value: string) {
      return prefixedLabel(t, "approvalType", value);
    },
    approvalStatusLabel(value: string) {
      return prefixedLabel(t, "approvalStatus", value);
    },
    stocktakeStatusLabel(value: string) {
      return prefixedLabel(t, "stocktakeStatus", value);
    },
    stocktakeScopeLabel(value: string) {
      return prefixedLabel(t, "stocktakeScope", value);
    },
    movementTypeLabel(value: string) {
      return prefixedLabel(t, "movementType", value);
    },
  };
}
