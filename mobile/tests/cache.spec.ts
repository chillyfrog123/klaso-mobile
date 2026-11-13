import { test, expect } from "@playwright/test";
import data from "../src/demo.json" with { type: "json" };
const timestamp = "2026-11-09T12:34:00.000Z";
async function mockNative(page: import("@playwright/test").Page, mode: string) {
  await page.addInitScript(
    ({ fixtures, timestamp, mode }) => {
      const snapshots = {
        Grades: { payload: fixtures.Grades, updated: timestamp },
      };
      const empty = { snapshots: {}, details: {} };
      (window as any).__bridge = {
        bootstrap: async () => ({
          account: { owner: "fictional-a", session: true, remembered: true },
          archive: { snapshots, details: {} },
        }),
        language: async () => {},
        official: async () => {},
        logout: async () => {},
        login: async () => ({
          account: { owner: "fictional-b", session: true, remembered: false },
          archive: empty,
        }),
        load: async ({ section, owner }: any) => {
          if (owner === "fictional-b") throw Error("requestFailed");
          if (mode === "fail") throw Error("requestFailed");
          if (section === "Grades")
            return await new Promise((resolve) => {
              (window as any).__finish = () =>
                resolve({
                  payload: fixtures.Grades,
                  updated: "2026-11-10T15:00:00.000Z",
                });
            });
          throw Error("requestFailed");
        },
        detail: async () => {
          throw Error("requestFailed");
        },
      };
    },
    { fixtures: data, timestamp, mode },
  );
  await page.route("**/src/bridge.ts", (route) =>
    route.fulfill({
      contentType: "application/javascript",
      body: `export const native=true;export const bridge=window.__bridge;export const official=async()=>{};export const openGithub=async()=>{};export const errorCode=()=>"requestFailed";export const demoData=async()=>({});`,
    }),
  );
  await page.goto("/");
  await page.getByRole("button", { name: "Grades", exact: true }).click();
  await expect(page.getByText("Overall averages")).toBeVisible();
}
test("failed refresh retains grades and original successful timestamp", async ({
  page,
}) => {
  await mockNative(page, "fail");
  const previous = await page.locator(".freshness").textContent();
  await page.getByRole("button", { name: "Refresh", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText(
    "Your last successful data is unchanged",
  );
  await expect(page.getByText("Overall averages")).toBeVisible();
  await expect(page.locator(".freshness")).toHaveText(previous!);
});
test("a previous account response cannot enter the newly signed-in account", async ({
  page,
}) => {
  await mockNative(page, "delay");
  await page.getByRole("button", { name: "Refresh", exact: true }).click();
  await expect
    .poll(() => page.evaluate(() => typeof (window as any).__finish))
    .toBe("function");
  await page
    .getByRole("button", { name: "Profile", exact: true })
    .first()
    .click();
  await page.getByRole("button", { name: "Sign out", exact: true }).click();
  await page.getByLabel("Login", { exact: true }).fill("fictional-b");
  await page.getByLabel("Password", { exact: true }).fill("fictional-password");
  await page
    .getByRole("button", { name: "Sign in to Synergia", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Grades", exact: true }),
  ).toBeVisible();
  await page.evaluate(() => (window as any).__finish());
  await expect(page.getByText("Overall averages")).not.toBeVisible();
  await expect(page.getByText("Nothing here yet")).toBeVisible();
});

test("saved account switch replaces cached data without retaining previous owner", async ({
  page,
}) => {
  await mockNative(page, "fail");
  await page.evaluate(() => {
    (window as any).__bridge.bootstrap = async () => ({
      account: null,
      accounts: ["fictional-a", "fictional-b"],
      archive: { snapshots: {}, details: {} },
    });
    (window as any).__bridge.switchAccount = async ({ owner }: any) => ({
      account: { owner, session: true, remembered: true },
      accounts: ["fictional-a", "fictional-b"],
      archive: { snapshots: {}, details: {} },
    });
  });
  await page
    .getByRole("button", { name: "Profile", exact: true })
    .first()
    .click();
  await page.getByRole("button", { name: "Sign out", exact: true }).click();
  await page.getByRole("button", { name: "fictional-b", exact: true }).click();
  await expect(page.getByText("Overall averages")).not.toBeVisible();
  await page
    .getByRole("button", { name: "Profile", exact: true })
    .first()
    .click();
  await expect(
    page.getByRole("heading", { name: "fictional-b", exact: true }),
  ).toBeVisible();
});
