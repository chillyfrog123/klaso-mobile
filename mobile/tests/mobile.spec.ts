import { test, expect } from "@playwright/test";
import packageInfo from "../package.json" with { type: "json" };
async function demo(page: any) {
  await page.goto("/");
  await page
    .getByRole("button", { name: "Profile", exact: true })
    .first()
    .click();
  await page.getByRole("button", { name: "Explore offline demo" }).click();
  await expect(page.getByText("Overall averages")).toBeVisible();
}
test("phone grades, translated preferences and accessible details", async ({
  page,
}) => {
  await demo(page);
  await expect(page.locator(".bottom-nav button")).toHaveCount(5);
  await page.locator(".grade.high").first().click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await expect(page.getByText("Contributes to average")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).not.toBeVisible();
  await page
    .getByRole("button", { name: "Profile", exact: true })
    .first()
    .click();
  await page.getByRole("button", { name: "Language", exact: true }).click();
  await page.getByRole("radio", { name: "Polski", exact: true }).click();
  await page.getByRole("button", { name: "Fiolet", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("lang", "pl");
  await page.reload();
  await page
    .getByRole("button", { name: "Profil", exact: true })
    .first()
    .click();
  await expect(
    page.getByRole("button", { name: "Język", exact: true }),
  ).toContainText("Polski");
  await expect(page.locator("html")).toHaveAttribute("data-accent", "Violet");
});
test("timetable, full-day attendance filtering, inbox and notices", async ({
  page,
}) => {
  await page.setViewportSize({ width: 320, height: 700 });
  await demo(page);
  await page.getByRole("button", { name: "Timetable", exact: true }).click();
  await page.locator(".day-picker button").first().click();
  await expect(page.locator(".lesson-card:visible").first()).toBeVisible();
  await page.locator(".day-picker button").nth(1).click();
  const substitution = page.locator(".lesson-change:visible");
  await expect(substitution.locator("button.substitution")).toHaveCount(2);
  await expect(substitution.locator(":scope > svg")).toBeVisible();
  const previous = substitution.getByRole("button", {
    name: /Previous lesson/,
  });
  const current = substitution.getByRole("button", { name: /New lesson/ });
  await expect(previous).toContainText("Język niemiecki");
  await expect(previous).toContainText("204");
  await expect(current).toContainText("Biologia");
  await expect(current).toContainText("103");
  expect((await previous.boundingBox())?.width).toBe(
    (await current.boundingBox())?.width,
  );
  await previous.click();
  await expect(page.getByRole("dialog")).toContainText("204");
  await page.keyboard.press("Escape");
  await current.click();
  await expect(page.getByRole("dialog")).toContainText("103");
  await page.keyboard.press("Escape");
  await page.locator(".day-picker button").nth(2).click();
  const replacement = page.locator(".lesson-change:visible");
  await expect(replacement.locator(".lesson-card.canceled")).toContainText(
    "Matematyka",
  );
  await expect(
    replacement.locator(".lesson-card:not(.canceled):not(.substitution)"),
  ).toContainText("Fizyka");
  await page.locator(".day-picker button").nth(6).click();
  await expect(
    page.locator(".day-column:not(.inactive-day) .day-off strong"),
  ).toBeVisible();
  await page.getByRole("button", { name: "Attendance", exact: true }).click();
  await page.getByRole("button", { name: "Clear all", exact: true }).click();
  await expect(page.getByText("Nothing here yet")).toBeVisible();
  await page
    .getByRole("button", { name: "EA Excused absence", exact: true })
    .click();
  await expect(page.locator(".attendance-row .type-0").first()).toBeVisible();
  await expect(page.locator(".attendance-day h2").first()).toContainText(
    "Monday",
  );
  expect(
    await page
      .locator(".attendance-day > .card")
      .first()
      .evaluate((card) => {
        const tiles = card.querySelectorAll<HTMLElement>(".attendance-row");
        return (
          card.scrollWidth === card.clientWidth &&
          tiles[0].offsetTop < tiles[tiles.length - 1].offsetTop
        );
      }),
  ).toBe(true);
  await page.getByRole("button", { name: "Messages", exact: true }).click();
  await page.locator(".mail-card").first().click();
  await expect(page.locator(".reader")).toBeVisible();
  await expect(page.locator(".mail-list")).not.toBeVisible();
  await expect(page.locator(".message-body")).toContainText("Dzień dobry");
  await page.getByRole("button", { name: "Back", exact: true }).click();
  await page.getByRole("button", { name: "Notices", exact: true }).click();
  await page.locator("summary").first().click();
  await expect(page.locator("details[open] .message-body")).toContainText(
    "Zapraszamy",
  );
});
test("touch swipes move one timetable day across the week boundary", async ({
  page,
}) => {
  await demo(page);
  await page.getByRole("button", { name: "Timetable", exact: true }).click();
  const days = page.locator(".day-picker button");
  await days.nth(5).click();
  const week = page.locator(".week-toolbar strong");
  const initialWeek = await week.textContent();
  const touch = await page.context().newCDPSession(page);
  await touch.send("Emulation.setTouchEmulationEnabled", { enabled: true });
  const swipe = async (from: [number, number], to: [number, number]) => {
    await touch.send("Input.dispatchTouchEvent", {
      type: "touchStart",
      touchPoints: [{ x: from[0], y: from[1] }],
    });
    for (const fraction of [0.25, 0.5, 0.75, 1]) {
      await touch.send("Input.dispatchTouchEvent", {
        type: "touchMove",
        touchPoints: [
          {
            x: from[0] + (to[0] - from[0]) * fraction,
            y: from[1] + (to[1] - from[1]) * fraction,
          },
        ],
      });
    }
    await touch.send("Input.dispatchTouchEvent", {
      type: "touchEnd",
      touchPoints: [],
    });
  };
  await swipe([300, 290], [90, 290]);
  await expect(days.nth(6)).toHaveAttribute("aria-pressed", "true");
  await expect(week).toHaveText(initialWeek!);
  await swipe([300, 290], [90, 290]);
  await expect(days.first()).toHaveAttribute("aria-pressed", "true");
  await expect(week).not.toHaveText(initialWeek!);
  await expect(page.locator(".lesson-card:visible").first()).toBeVisible();
  await swipe([90, 290], [300, 290]);
  await expect(days.nth(6)).toHaveAttribute("aria-pressed", "true");
  await expect(week).toHaveText(initialWeek!);
  await swipe([180, 290], [185, 185]);
  await expect(days.nth(6)).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByRole("dialog")).not.toBeVisible();
});
test("320px layout, reduced motion and tablet split view", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 700 });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await demo(page);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBeTruthy();
  expect(
    await page
      .locator(".grade.high")
      .first()
      .evaluate((el) => getComputedStyle(el, "::after").animationName),
  ).toBe("none");
  await page.screenshot({ path: "artifacts/grades-phone.png", fullPage: true });
  await page.setViewportSize({ width: 1024, height: 900 });
  await page.getByRole("button", { name: "Messages", exact: true }).click();
  await page.locator(".mail-card").first().click();
  await expect(page.locator(".reader")).toBeVisible();
  await expect(page.locator(".mail-list")).toBeVisible();
  await page.screenshot({
    path: "artifacts/messages-tablet.png",
    fullPage: true,
  });
});

test("Android uses Material You and the iPhone preview keeps Liquid Glass", async ({
  page,
}) => {
  await demo(page);
  await expect(page.locator("html")).toHaveAttribute("data-design", "material");
  expect(
    await page
      .locator(".bottom-nav")
      .evaluate((el) => getComputedStyle(el).backdropFilter),
  ).toBe("none");
  await page.screenshot({ path: "artifacts/android-material-grades.png" });
  await page.goto("/?platform=ios");
  await page
    .getByRole("button", { name: "Profile", exact: true })
    .first()
    .click();
  await page.getByRole("button", { name: "Explore offline demo" }).click();
  await expect(page.getByText("Overall averages")).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute("data-design", "glass");
  expect(
    await page
      .locator(".bottom-nav")
      .evaluate((el) => getComputedStyle(el).backdropFilter),
  ).toContain("blur");
  await page.screenshot({ path: "artifacts/iphone-glass-grades.png" });
});

test("custom choices, dark theme, behavior order and blue sixes", async ({
  page,
}) => {
  await demo(page);
  await expect(page.locator("select")).toHaveCount(0);
  await expect(page.locator(".topbar")).not.toContainText("Klaso");
  expect(
    await page
      .locator(".behaviour")
      .evaluate(
        (el) =>
          !!(
            el.compareDocumentPosition(
              document.querySelector(".summary-card")!,
            ) & Node.DOCUMENT_POSITION_FOLLOWING
          ),
      ),
  ).toBe(true);
  await expect(page.locator(".grade.excellent").first()).toHaveText("6");
  await page
    .getByRole("button", { name: "Profile", exact: true })
    .first()
    .click();
  await expect(
    page.getByText("Private by design", { exact: true }),
  ).toHaveCount(0);
  await expect(page.getByText("Klaso Mobile", { exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: "Theme", exact: true }).click();
  await page.getByRole("radio", { name: "Dark", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("data-mode", "dark");
  await page
    .getByRole("button", { name: "About the app", exact: true })
    .click();
  await expect(page.getByText(`Version ${packageInfo.version}`)).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Klaso Mobile", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("Private by design", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Open GitHub", exact: true }),
  ).toHaveCount(1);
  await expect(page.locator(".about a")).toHaveCount(0);
  await expect(
    page.getByText(
      "No analytics or telemetry. Your school data stays private.",
    ),
  ).toBeVisible();
  await page.screenshot({ path: "artifacts/klaso-about-dark.png" });
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-mode", "dark");
});
