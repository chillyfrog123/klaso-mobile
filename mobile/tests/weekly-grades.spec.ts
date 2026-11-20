import { test, expect, type Page } from "@playwright/test";
import data from "../src/demo.json" with { type: "json" };

async function weeklyGrades(page: Page, now = "2026-11-20T12:00:00Z") {
  await page.clock.install({ time: new Date(now) });
  await page.addInitScript((fixtures) => {
    localStorage.setItem("language", "pl");
    const payload = structuredClone(fixtures.Grades);
    const base = payload.Grades.items[0];
    payload.Grades.items = [
      ["previous-week", "2026-11-15", "1", "1"],
      ["monday", "2026-11-16", "1", "2"],
      ["friday", "2026-11-20", "2", "4"],
      ["sunday", "2026-11-22", "2", "6"],
      ["next-week", "2026-11-23", "1", "3"],
      ["undated", "", "1", "5"],
    ].map(([id, date, semester, value]) => ({
      ...base,
      id,
      date,
      semester,
      value,
      subject: "Matematyka",
    }));
    payload.Grades.summaries = [
      [3.5, 3.75],
      [2.75, 3],
      [5, 5.25],
    ].map((pair) => ({ overall: pair, subjects: { Matematyka: pair } }));
    const updated = "2026-11-20T12:00:00Z";
    const snapshot = { payload, updated };
    const activity = {
      newGradeIds: ["id:previous-week", "id:friday"],
      baselineReady: true,
      since: "2026-11-19T12:00:00Z",
      updated,
    };
    (window as any).__bridge = {
      bootstrap: async () => ({
        account: { owner: "fictional-a", session: true, remembered: true },
        archive: { snapshots: { Grades: snapshot }, details: {}, activity },
      }),
      language: async () => {},
      monitoring: async () => ({
        enabled: false,
        permission: "granted",
        background: false,
      }),
      consumeNotification: async () => ({}),
      setSchoolPaused: async () => {},
      load: async ({ section }: any) => {
        if (section === "Grades") return { ...snapshot, activity };
        throw Error("requestFailed");
      },
    };
  }, data);
  await page.route(/\/src\/bridge\.ts(?:\?.*)?$/, (route) =>
    route.fulfill({
      contentType: "application/javascript",
      body: `export const native=true;export const bridge=window.__bridge;export const official=async()=>{};export const openGithub=async()=>{};export const errorCode=()=>"requestFailed";export const demoData=async()=>({});`,
    }),
  );
  await page.goto("/");
  await page.getByRole("button", { name: "Oceny", exact: true }).click();
}

test("weekly grades are a separate view, combine with semester, and retain full averages and session badges", async ({
  page,
}) => {
  await weeklyGrades(page);
  const all = page.getByRole("button", {
    name: "Wszystkie oceny",
    exact: true,
  });
  const sinceLogin = page.getByRole("button", {
    name: "Nowe od ostatniego logowania",
  });
  const thisWeek = page.getByRole("button", { name: "Nowe w tym tygodniu" });
  const grades = page.locator(".grade-row .grade");
  const averages = await page.locator(".summary-card h2").textContent();
  await expect(grades).toHaveCount(6);
  await expect(page.locator(".grade-row .grade.is-new")).toHaveCount(2);
  await thisWeek.click();
  await expect(thisWeek).toHaveAttribute("aria-pressed", "true");
  await expect(all).toHaveAttribute("aria-pressed", "false");
  await expect(sinceLogin).toHaveAttribute("aria-pressed", "false");
  await expect(grades).toHaveText(["2", "4", "6"]);
  await expect(page.locator(".grade-row .grade.is-new")).toHaveCount(1);
  await expect(page.locator(".summary-card h2")).toHaveText(averages!);
  await expect(page.locator(".summary-footer")).toContainText("6 oceny");
  await expect(page.locator(".grade-filters")).toContainText(
    "według daty oceny",
  );
  await sinceLogin.click();
  await expect(thisWeek).toHaveAttribute("aria-pressed", "false");
  await expect(grades).toHaveText(["1", "4"]);
  await thisWeek.click();
  await page.getByRole("button", { name: "Semestr", exact: true }).click();
  await page.getByRole("radio", { name: "Semestr 1", exact: true }).click();
  await expect(grades).toHaveText(["2"]);
  await expect(page.locator(".grade-row .grade.is-new")).toHaveCount(0);
  const semesterAverages = await page.locator(".summary-card h2").textContent();
  await all.click();
  await expect(grades).toHaveText(["1", "2", "3", "5"]);
  await expect(page.locator(".summary-card h2")).toHaveText(semesterAverages!);
  await expect(page.locator(".grade-row .grade.is-new")).toHaveCount(1);
  await thisWeek.click();
  await page.setViewportSize({ width: 320, height: 700 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: "artifacts/weekly-grades-phone.png",
    fullPage: true,
  });
});

test("weekly grades show their own empty state", async ({ page }) => {
  await weeklyGrades(page, "2026-12-01T12:00:00Z");
  await page.getByRole("button", { name: "Nowe w tym tygodniu" }).click();
  await expect(page.locator(".grade-row .grade")).toHaveCount(0);
  await expect(
    page.getByRole("heading", {
      name: "Brak ocen z tego tygodnia",
      exact: true,
    }),
  ).toBeVisible();
});
