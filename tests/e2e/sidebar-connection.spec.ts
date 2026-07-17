import { expect, test, type Page } from "@playwright/test";

async function installAuthenticatedHostMocks(page: Page) {
  await page.addInitScript(() => {
    window.localStorage.setItem("aster.appearance", JSON.stringify({
      accentColor: "#2f6dff",
      liquidGlassStyle: "tinted",
      locale: "zh-CN",
      themeMode: "dark",
    }));

    let callbackId = 0;
    const currentUser = {
      id: "admin",
      username: "admin",
      displayName: "管理员",
      roles: [{ id: "role-admin", code: "admin", name: "管理员" }],
      permissions: ["view_reports", "write_stock"],
    };
    const appStatus = {
      appName: "Aster",
      appVersion: "",
      schemaVersion: 1,
      runtime: {
        mode: "host",
        hostAddress: null,
        hostPort: 17871,
        clientPaired: false,
        clientDeviceId: "test-device",
        dataDir: "/tmp",
        databasePath: "/tmp/aster.sqlite",
        backupDir: "/tmp/backups",
        exportDir: "/tmp/exports",
        importReportDir: "/tmp/imports",
      },
      latestMovementMonth: null,
      metrics: {
        itemCount: 0,
        departmentCount: 0,
        supplierCount: 0,
        currentStockAmount: 0,
        lowStockCount: 0,
        negativeStockCount: 0,
        thisMonthInboundAmount: 0,
        thisMonthOutboundAmount: 0,
      },
      recentOperations: [],
      health: {
        databaseOk: true,
        stockBalanceConsistencyOk: true,
        stockBalanceIssueCount: 0,
        autoBackupEnabled: false,
        intervalBackupEnabled: false,
        intervalBackupHours: 24,
        secondBackupOk: true,
        message: "就绪",
      },
    };

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
          if (command === "get_current_user") return currentUser;
          if (command === "get_app_status") return appStatus;
          if (command === "get_password_change_required") return false;
          if (command === "get_host_service_status") {
            return {
              running: true,
              bindAddress: "0.0.0.0",
              port: 17871,
              pairCode: "123456789012",
              clientCount: 0,
              message: "已启动",
            };
          }
          if (command.endsWith("_page")) {
            return { items: [], nextCursor: null };
          }
          if (command.startsWith("list_")) return [];
          if (command.startsWith("plugin:event|")) return 1;
          return null;
        },
        transformCallback: () => ++callbackId,
        unregisterCallback: () => undefined,
      },
    });
  });
}

test("overflowing sidebar connection hint scrolls continuously", async ({ page }) => {
  await installAuthenticatedHostMocks(page);
  await page.setViewportSize({ width: 1280, height: 820 });
  await page.goto("/");

  const marquee = page.locator(".sidebar-connection-marquee");
  await expect(marquee).toBeVisible();
  await expect(marquee.locator(".sidebar-connection-marquee-item").first()).toHaveText(
    "正式数据保存在这台电脑，其他电脑可通过配对码连接。",
  );
  await expect(marquee).toHaveAttribute("data-overflowing", "true");
  await expect(marquee.locator(".sidebar-connection-marquee-item")).toHaveCount(2);

  const track = marquee.locator(".sidebar-connection-marquee-track");
  await expect.poll(() => track.evaluate((element) => getComputedStyle(element).transform))
    .not.toBe("none");
  const firstTransform = await track.evaluate((element) => getComputedStyle(element).transform);
  await expect.poll(() => track.evaluate((element) => getComputedStyle(element).transform))
    .not.toBe(firstTransform);
});

test("reduced motion shows the full hint without animation", async ({ browser }) => {
  const context = await browser.newContext({
    reducedMotion: "reduce",
    viewport: { width: 1280, height: 820 },
  });
  const page = await context.newPage();
  await installAuthenticatedHostMocks(page);
  await page.goto("/");

  const marquee = page.locator(".sidebar-connection-marquee");
  await expect(marquee.locator(".sidebar-connection-marquee-item")).toHaveCount(1);
  await expect(marquee).toContainText(
    "正式数据保存在这台电脑，其他电脑可通过配对码连接。",
  );
  const styles = await marquee.evaluate((element) => {
    const track = element.querySelector<HTMLElement>(".sidebar-connection-marquee-track");
    return {
      animationName: track ? getComputedStyle(track).animationName : null,
      overflow: getComputedStyle(element).overflow,
      whiteSpace: getComputedStyle(element).whiteSpace,
    };
  });
  expect(styles).toEqual({
    animationName: "none",
    overflow: "visible",
    whiteSpace: "normal",
  });

  await context.close();
});
