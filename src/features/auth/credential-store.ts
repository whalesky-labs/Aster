import { invoke } from "@tauri-apps/api/core";

const LEGACY_LOGIN_KEY = "aster.rememberedLogin";
const REMEMBERED_USERNAME_KEY = "aster.rememberedUsername";

export type SavedCredential = {
  username: string;
  password: string;
};

export function migrateLegacyLoginStorage() {
  window.localStorage.removeItem(LEGACY_LOGIN_KEY);
}

export function loadRememberedUsername() {
  return window.localStorage.getItem(REMEMBERED_USERNAME_KEY)?.trim() ?? "";
}

export async function loadSystemCredential(username: string) {
  if (!username.trim()) return null;
  return invoke<SavedCredential | null>("load_saved_credential", { username });
}

export async function persistLoginCredential(
  username: string,
  password: string,
  remember: boolean,
) {
  const normalizedUsername = username.trim();
  if (!remember) {
    window.localStorage.removeItem(REMEMBERED_USERNAME_KEY);
    await invoke("delete_login_credential", { username: normalizedUsername });
    return;
  }
  await invoke("save_login_credential", {
    request: { password, username: normalizedUsername },
  });
  window.localStorage.setItem(REMEMBERED_USERNAME_KEY, normalizedUsername);
}
