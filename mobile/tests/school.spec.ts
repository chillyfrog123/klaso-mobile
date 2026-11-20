import { test, expect, type Page } from "@playwright/test";
import data from "../src/demo.json" with { type: "json" };

async function mockSchool(
  page: Page,
  options: {
    delayed?: boolean;
    enabled?: boolean;
    signedOut?: boolean;
    room?: string;
  } = {},
) {
  await page.addInitScript(
    ({ fixtures, options }) => {
      const base = fixtures.Grades.Grades.items[0];
      const old = {
        ...base,
        id: "old",
        subject: "School subject",
        semester: "1",
        value: "3",
      };
      const recent = {
        ...base,
        id: "recent",
        subject: "School subject",
        semester: "2",
        value: "5",
      };
      const duplicate = {
        ...base,
        id: "",
        subject: "School subject",
        semester: "2",
        value: "4",
      };
      const fallback = JSON.stringify([
        duplicate.subject,
        duplicate.value,
        duplicate.category,
        duplicate.date,
        duplicate.teacher,
        duplicate.semester,
      ]);
      const payload = structuredClone(fixtures.Grades);
      payload.Grades.items = [old, recent, duplicate, { ...duplicate }];
      payload.Grades.summaries = [0, 1, 2].map(() => ({
        overall: [4, 4.25],
        subjects: { "School subject": [4, 4.25] },
      }));
      const stamp = "2026-11-17T09:00:00.000Z";
      const archive = {
        snapshots: { Grades: { payload, updated: stamp } },
        details: {},
        activity: {
          newGradeIds: ["id:recent", fallback + "#1"],
          baselineReady: true,
          since: "2026-11-16T10:00:00.000Z",
          updated: stamp,
        },
      };
      const account = { owner: "fictional-a", session: true, remembered: true };
      const control: any = ((window as any).__school = {
        account: options.signedOut ? null : account,
        archive,
        enabled: options.enabled ?? true,
        permission: "prompt",
        checks: 0,
        pauses: [],
        notification: {},
        monitoring: () => ({
          enabled: control.enabled,
          permission: control.permission,
          background: !!control.account?.remembered,
        }),
      });
      (window as any).__demo = fixtures;
      const state = () => ({
        account: control.account,
        accounts: ["fictional-a", "fictional-b"],
        archive: control.archive,
      });
      (window as any).__bridge = {
        bootstrap: async () => structuredClone(state()),
        language: async () => {},
        monitoring: async () => control.monitoring(),
        setMonitoring: async ({ enabled, requestPermission }: any) => {
          control.enabled = enabled;
          if (requestPermission) control.permission = "granted";
          return control.monitoring();
        },
        consumeNotification: async () => {
          const result = control.notification;
          control.notification = {};
          return result;
        },
        setSchoolPaused: async ({ paused }: any) => {
          control.pauses.push(paused);
        },
        syncSchool: async () => {
          control.checks++;
          if (options.delayed && control.checks === 1) {
            const captured = structuredClone(state());
            return new Promise((resolve) => {
              control.finish = () => resolve(captured);
            });
          }
          return structuredClone({ ...state(), syncErrors: [] });
        },
        load: async ({ section, week }: any) => {
          if (section === "Grades" && control.archive.snapshots.Grades)
            return structuredClone({
              ...control.archive.snapshots.Grades,
              activity: control.archive.activity,
            });
          if (section === "Timetable") {
            const lessons = structuredClone(fixtures.Timetable.Timetable);
            if (options.room !== undefined) lessons[0].room = options.room;
            const delta =
              new Date(week + "T12:00:00").getTime() -
              new Date("2026-11-09T12:00:00").getTime();
            for (const lesson of lessons)
              lesson.date = new Date(
                new Date(lesson.date + "T12:00:00").getTime() + delta,
              ).toLocaleDateString("en-CA");
            return { payload: { Timetable: lessons }, updated: stamp };
          }
          throw Error("requestFailed");
        },
        switchAccount: async ({ owner }: any) => {
          control.account = { ...account, owner };
          control.archive = {
            snapshots: {},
            details: {},
            activity: { newGradeIds: [], baselineReady: false },
          };
          return structuredClone(state());
        },
        logout: async () => {
          control.account = null;
          control.archive = { snapshots: {}, details: {} };
        },
        official: async () => {},
      };
    },
    { fixtures: data, options },
  );
  await page.route(/\/src\/bridge\.ts(?:\?.*)?$/, (route) =>
    route.fulfill({
      contentType: "application/javascript",
      body: `export const native=true;export const bridge=window.__bridge;export const official=async()=>{};export const openGithub=async()=>{};export const errorCode=(e)=>e?.message==="verification"?"verification":"requestFailed";export const demoData=async()=>structuredClone(window.__demo);`,
    }),
  );
  await page.goto("/");
}

async function profile(page: Page) {
  await page
    .getByRole("button", { name: "Profile", exact: true })
    .first()
    .click();
}

test("new grades keep duplicate identities, combine with semester, and retain full averages", async ({
  page,
}) => {
  await mockSchool(page);
  await page.getByRole("button", { name: "Grades", exact: true }).click();
  await expect(page.locator(".grade-row .grade")).toHaveCount(4);
  await expect(page.locator(".grade-row .grade.is-new")).toHaveCount(2);
  const averages = await page.locator(".summary-card h2").textContent();
  await page
    .getByRole("button", { name: "New since last login", exact: false })
    .click();
  await expect(page.locator(".grade-row .grade")).toHaveCount(2);
  await expect(page.locator(".summary-card h2")).toHaveText(averages!);
  await page.setViewportSize({ width: 320, height: 700 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: "artifacts/new-grades-phone.png",
    fullPage: true,
  });
  await page.getByRole("button", { name: "Semester", exact: true }).click();
  await page.getByRole("radio", { name: "Semester 1", exact: true }).click();
  await expect(page.locator(".grade-row .grade")).toHaveCount(0);
  await page.getByRole("button", { name: "All grades", exact: true }).click();
  await expect(page.locator(".grade-row .grade")).toHaveCount(1);
  await page.getByRole("button", { name: "Refresh", exact: true }).click();
  await expect(page.locator(".grade-row .grade.is-new")).toHaveCount(0);
  await page.getByRole("button", { name: "Semester", exact: true }).click();
  await page.getByRole("radio", { name: "All semesters", exact: true }).click();
  await expect(page.locator(".grade-row .grade.is-new")).toHaveCount(2);
});

test("automatic check updates saved grades without a manual refresh", async ({
  page,
}) => {
  await page.clock.install();
  await mockSchool(page, { delayed: true });
  await page.getByRole("button", { name: "Grades", exact: true }).click();
  await expect
    .poll(() => page.evaluate(() => typeof (window as any).__school.finish))
    .toBe("function");
  await page.evaluate(() => {
    // Resolve the running check with the original result; the next poll reads the new snapshot.
    (window as any).__school.finish();
  });
  await page.evaluate(() => {
    const state = (window as any).__school;
    const snapshot = state.archive.snapshots.Grades;
    snapshot.payload.Grades.items.push({
      ...snapshot.payload.Grades.items[0],
      id: "auto",
      value: "6",
    });
    snapshot.updated = "2026-11-17T09:05:00.000Z";
    state.archive.activity.newGradeIds.push("id:auto");
    state.archive.activity.updated = snapshot.updated;
  });
  await page.clock.fastForward(5 * 60 * 1000 + 1000);
  await expect(page.locator(".grade-row .grade")).toHaveCount(5);
  await expect(page.locator(".grade-row .grade.is-new")).toHaveCount(3);
});

test("notification permission is explicit and disabling checks stops polling", async ({
  page,
}) => {
  await page.clock.install();
  await mockSchool(page);
  await expect
    .poll(() => page.evaluate(() => (window as any).__school.checks))
    .toBe(1);
  await profile(page);
  await expect(
    page.getByRole("heading", { name: "School notifications", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Allow notifications", exact: true })
    .click();
  await expect
    .poll(() => page.evaluate(() => (window as any).__school.permission))
    .toBe("granted");
  await page
    .getByRole("switch", {
      name: "Automatic checks and notifications",
      exact: false,
    })
    .uncheck();
  const checks = await page.evaluate(() => (window as any).__school.checks);
  await page.clock.fastForward(16 * 60 * 1000);
  expect(await page.evaluate(() => (window as any).__school.checks)).toBe(
    checks,
  );
  await page
    .getByRole("switch", {
      name: "Automatic checks and notifications",
      exact: false,
    })
    .check();
  await expect
    .poll(() => page.evaluate(() => (window as any).__school.checks))
    .toBeGreaterThan(checks);
});

test("late automatic response cannot replace a newly selected account", async ({
  page,
}) => {
  await mockSchool(page, { delayed: true });
  await expect
    .poll(() => page.evaluate(() => typeof (window as any).__school.finish))
    .toBe("function");
  await profile(page);
  await page.getByRole("button", { name: "fictional-b", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Grades", exact: true }),
  ).toBeVisible();
  await page.evaluate(() => (window as any).__school.finish());
  await expect(page.locator(".grade-row .grade")).toHaveCount(0);
  await profile(page);
  await expect(
    page.getByRole("heading", { name: "fictional-b", exact: true }),
  ).toBeVisible();
});

test("signed-out app never starts school checks", async ({ page }) => {
  await page.clock.install();
  await mockSchool(page, { signedOut: true });
  await page.clock.fastForward(16 * 60 * 1000);
  expect(await page.evaluate(() => (window as any).__school.checks)).toBe(0);
});

test("hidden app skips foreground polling and checks on resume", async ({
  page,
}) => {
  await page.clock.install();
  await mockSchool(page);
  await expect
    .poll(() => page.evaluate(() => (window as any).__school.checks))
    .toBe(1);
  await page.evaluate(() => {
    Object.defineProperty(document, "hidden", {
      configurable: true,
      get: () => true,
    });
    document.dispatchEvent(new Event("visibilitychange"));
  });
  await page.clock.fastForward(6 * 60 * 1000);
  expect(await page.evaluate(() => (window as any).__school.checks)).toBe(1);
  await page.evaluate(() => {
    Object.defineProperty(document, "hidden", {
      configurable: true,
      get: () => false,
    });
    document.dispatchEvent(new Event("visibilitychange"));
  });
  await expect
    .poll(() => page.evaluate(() => (window as any).__school.checks))
    .toBe(2);
});

test("offline demo pauses live school checks", async ({ page }) => {
  await page.clock.install();
  await mockSchool(page);
  await expect
    .poll(() => page.evaluate(() => (window as any).__school.checks))
    .toBe(1);
  await profile(page);
  await page
    .getByRole("button", { name: "Explore offline demo", exact: true })
    .click();
  await expect(page.getByText("Overall averages")).toBeVisible();
  await expect
    .poll(() => page.evaluate(() => (window as any).__school.pauses.at(-1)))
    .toBe(true);
  const checks = await page.evaluate(() => (window as any).__school.checks);
  await page.clock.fastForward(16 * 60 * 1000);
  expect(await page.evaluate(() => (window as any).__school.checks)).toBe(
    checks,
  );
});

test("room number appears in the timetable card and lesson details", async ({
  page,
}) => {
  await mockSchool(page, { enabled: false });
  await page.locator(".day-picker button").first().click();
  const lesson = page.locator(".lesson-card:visible").first();
  await expect(lesson.locator(".lesson-meta span").last()).toHaveText("101");
  await lesson.click();
  await expect(page.getByRole("dialog")).toContainText("101");
});

test("server-provided classroom names appear unchanged in cards and details", async ({
  page,
}) => {
  await page.setViewportSize({ width: 320, height: 700 });
  const room = "Pracownia informatyczna — budynek A, II piętro";
  await mockSchool(page, { enabled: false, room });
  await page.locator(".day-picker button").first().click();
  const lesson = page.locator(".lesson-card:visible").first();
  await expect(lesson.locator(".lesson-meta span").last()).toHaveText(room);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await lesson.click();
  await expect(
    page.getByRole("dialog").getByText(room, { exact: true }),
  ).toBeVisible();
});
