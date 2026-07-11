import { createI18n, type I18n } from "../../i18n";

const defaultI18n = createI18n("zh-CN");

export function formatDateTime(value?: string | null) {
  if (!value) return "-";
  const normalized = value.trim().replace("T", " ");
  const [datePart, timePart] = normalized.split(" ");
  if (!timePart) return datePart;
  const cleanTime = timePart.split(/[.+-]/)[0];
  const [hour = "00", minute = "00", second = "00"] = cleanTime.split(":");
  return `${datePart} ${hour.padStart(2, "0")}:${minute.padStart(2, "0")}:${second.padStart(2, "0")}`;
}

export function formatFileSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export function backupTypeLabel(type: string, i18n: I18n = defaultI18n) {
  return i18n.backupTypeLabel(type);
}

export function auditActionLabel(action: string, i18n: I18n = defaultI18n) {
  return i18n.auditActionLabel(action);
}

export function auditEntityLabel(type: string, i18n: I18n = defaultI18n) {
  return i18n.auditEntityLabel(type);
}

export function normalizeSearchText(value?: string | number | boolean | null) {
  return String(value ?? "").trim().toLocaleLowerCase("zh-CN");
}

export function matchesSearchText(
  search: string,
  values: Array<string | number | boolean | null | undefined>,
) {
  const query = normalizeSearchText(search);
  return !query || values.some((value) => normalizeSearchText(value).includes(query));
}
