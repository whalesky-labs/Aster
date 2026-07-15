import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    let callbackId = 0;
    Object.defineProperty(window, "__TAURI_OS_PLUGIN_INTERNALS__", {
      value: { platform: "macos" },
    });
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      value: {
        metadata: {
          currentWebview: { label: "main", windowLabel: "main" },
          currentWindow: { label: "main" },
        },
        invoke: async (command: string) => {
          if (command === "get_current_user") return null;
          if (command.startsWith("list_")) return [];
          if (command === "get_app_status") {
            return {
              appName: "Aster", appVersion: "0.1.0", schemaVersion: 1,
              runtime: {
                mode: "standalone", hostAddress: null, hostPort: 17871,
                clientPaired: false, clientDeviceId: "test-device",
                dataDir: "/tmp", databasePath: "/tmp/aster.sqlite",
                backupDir: "/tmp/backups", exportDir: "/tmp/exports",
                importReportDir: "/tmp/imports",
              },
              latestMovementMonth: null,
              metrics: {}, recentOperations: [],
              health: { databaseOk: true, message: "ok" },
            };
          }
          if (command.startsWith("plugin:event|")) return 1;
          return null;
        },
        transformCallback: () => ++callbackId,
        unregisterCallback: () => undefined,
      },
    });
  });
});

test("login screen has no bottom gap or horizontal overflow", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "物资有序，运营从容" })).toBeVisible();
  await expect(page.locator(".login-expression-item img").first()).toBeVisible();
  const dimensions = await page.locator(".login-shell").evaluate((element) => ({
    bodyWidth: document.body.scrollWidth,
    shellBottom: Math.round(element.getBoundingClientRect().bottom),
    viewportHeight: window.innerHeight,
    viewportWidth: window.innerWidth,
  }));
  expect(dimensions.bodyWidth).toBeLessThanOrEqual(dimensions.viewportWidth);
  expect(dimensions.shellBottom).toBe(dimensions.viewportHeight);
});

test("editor window fills the viewport below the overlay titlebar", async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("aster.appearance", JSON.stringify({
      accentColor: "#2f6dff",
      liquidGlassStyle: "tinted",
      locale: "zh-CN",
      themeMode: "dark",
    }));
  });
  await page.setViewportSize({ width: 520, height: 380 });
  await page.goto("/?editor=department&mode=create&titlebar=overlay");
  await expect(page.getByRole("button", { name: "保存部门" })).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  const dimensions = await page.locator(".editor-shell").evaluate((element) => {
    const shell = element.getBoundingClientRect();
    const actions = element.querySelector(".editor-actions")?.getBoundingClientRect();
    return {
      actionsBottom: actions ? Math.round(actions.bottom) : null,
      bodyWidth: document.body.scrollWidth,
      paddingTop: Number.parseFloat(getComputedStyle(element).paddingTop),
      shellBottom: Math.round(shell.bottom),
      shellTop: Math.round(shell.top),
      viewportHeight: window.innerHeight,
      viewportWidth: window.innerWidth,
    };
  });
  expect(dimensions.shellTop).toBe(0);
  expect(dimensions.paddingTop).toBe(38);
  expect(dimensions.actionsBottom).toBe(dimensions.viewportHeight);
  expect(dimensions.shellBottom).toBe(dimensions.viewportHeight);
  expect(dimensions.bodyWidth).toBeLessThanOrEqual(dimensions.viewportWidth);
});
