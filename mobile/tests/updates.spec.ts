import { test, expect, type Page } from "@playwright/test";

async function mockUpdater(
  page: Page,
  {
    current = false,
    saved,
    error,
  }: { current?: boolean; saved?: string; error?: string } = {},
) {
  await page.addInitScript(
    ({ current, saved, error }) => {
      if (saved && !localStorage.getItem("app-updates-v1"))
        localStorage.setItem("app-updates-v1", saved);
      (window as any).__updates = {
        checks: 0,
        installs: 0,
        current,
        error,
        check: async () => {
          const self = (window as any).__updates;
          self.checks++;
          return {
            currentVersion: "0.1.2",
            release: self.current
              ? null
              : {
                  version: "0.1.3",
                  notes: "Improved updates\nFictional test release",
                },
          };
        },
        install: async () => {
          const self = (window as any).__updates;
          self.installs++;
          if (self.error) throw Error(self.error);
        },
      };
    },
    { current, saved, error },
  );
  await page.route("**/src/updateBridge.ts", (route) =>
    route.fulfill({
      contentType: "application/javascript",
      body: "export const updatesSupported=true; export const updateSource=window.__updates;",
    }),
  );
  // The actual web Capacitor App implementation has no native app-state listener.
  await page.goto("/");
}

async function profile(page: Page) {
  await page
    .getByRole("button", { name: "Profile", exact: true })
    .first()
    .click();
}

test("startup prompts without downloading, Later dismisses, manual check prompts again", async ({
  page,
}) => {
  await mockUpdater(page);
  const prompt = page.getByRole("dialog", { name: "Update available" });
  await expect(prompt).toBeVisible();
  await expect(prompt).toContainText("0.1.3");
  expect(await page.evaluate(() => (window as any).__updates.installs)).toBe(0);
  await prompt.getByRole("button", { name: "Later", exact: true }).click();
  await profile(page);
  await expect(
    page.getByRole("switch", { name: "Automatic updates" }),
  ).not.toBeChecked();
  await expect(
    page.getByRole("switch", { name: "Check automatically" }),
  ).toBeChecked();
  await page
    .getByRole("button", { name: "Check for updates", exact: true })
    .click();
  await expect(prompt).toBeVisible();
  await prompt
    .getByRole("button", { name: "Download and update", exact: true })
    .click();
  await expect(prompt).not.toBeVisible();
  await expect(page.getByRole("status")).toContainText("Installer opened");
  expect(await page.evaluate(() => (window as any).__updates.installs)).toBe(1);
});

test("opting in persists, starts installation, and disabling checks disables automatic updates", async ({
  page,
}) => {
  await mockUpdater(page);
  await page.getByRole("button", { name: "Later", exact: true }).click();
  await profile(page);
  await page.getByRole("switch", { name: "Automatic updates" }).check();
  await expect(page.getByRole("status")).toContainText("Installer opened");
  await page.reload();
  await expect(
    page.getByRole("dialog", { name: "Update available" }),
  ).not.toBeVisible();
  await profile(page);
  await expect(
    page.getByRole("switch", { name: "Automatic updates" }),
  ).toBeChecked();
  await expect
    .poll(() => page.evaluate(() => (window as any).__updates.installs))
    .toBe(1);
  await page.getByRole("switch", { name: "Check automatically" }).uncheck();
  await expect(
    page.getByRole("switch", { name: "Automatic updates" }),
  ).not.toBeChecked();
  await page.reload();
  await profile(page);
  expect(await page.evaluate(() => (window as any).__updates.checks)).toBe(0);
  await page
    .getByRole("button", { name: "Check for updates", exact: true })
    .click();
  await expect(
    page.getByRole("dialog", { name: "Update available" }),
  ).toBeVisible();
});

test("permission denial is actionable and manual retry succeeds", async ({
  page,
}) => {
  await mockUpdater(page, { error: "updatePermissionRequired" });
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Download and update" })
    .click();
  await expect(page.getByRole("alert")).toContainText("Allow Klaso Mobile");
  await page.getByRole("button", { name: "App updates", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "App updates" }),
  ).toBeVisible();
  await page.evaluate(() => {
    (window as any).__updates.error = undefined;
  });
  await page
    .getByRole("button", { name: "Download and update", exact: true })
    .click();
  await expect(page.getByRole("status")).toContainText("Installer opened");
});

test("current version, localized preferences, and narrow-screen layout", async ({
  page,
}) => {
  await page.setViewportSize({ width: 320, height: 720 });
  await mockUpdater(page, { current: true });
  await profile(page);
  await expect(page.getByRole("status")).toHaveText(
    "You're using the latest version.",
  );
  await page.getByRole("button", { name: "Language", exact: true }).click();
  await page.getByRole("radio", { name: "Polski", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Aktualizacje aplikacji" }),
  ).toBeVisible();
  await expect(
    page.getByRole("switch", { name: "Automatyczne aktualizacje" }),
  ).not.toBeChecked();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.locator(".app-updates").screenshot({
    path: "artifacts/updates-polish-320.png",
  });
});

test("web preview explains native-only updates and makes no release request", async ({
  page,
}) => {
  let requests = 0;
  await page.route("https://api.github.com/**", (route) => {
    requests++;
    return route.abort();
  });
  await page.goto("/");
  await profile(page);
  await expect(
    page.getByText("App updates are available in the installed Android app."),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Check for updates" }),
  ).toHaveCount(0);
  expect(requests).toBe(0);
});
