import { test, expect } from "@playwright/test";
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
  await demo(page);
  await page.getByRole("button", { name: "Timetable", exact: true }).click();
  await expect(page.locator(".lesson-card:visible").first()).toBeVisible();
  await page.locator(".day-picker button").nth(1).click();
  await expect(page.locator(".lesson-card.substitution")).toBeVisible();
  await page.locator(".day-picker button").nth(2).click();
  await expect(page.locator(".lesson-card.canceled")).toBeVisible();
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

test("0.1.2 custom choices, dark theme, behavior order and blue sixes", async ({
  page,
}) => {
  await demo(page);
  await expect(page.locator("select")).toHaveCount(0);
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
  await page.getByRole("button", { name: "Theme", exact: true }).click();
  await page.getByRole("radio", { name: "Dark", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("data-mode", "dark");
  await page
    .getByRole("button", { name: "About the app", exact: true })
    .click();
  await expect(page.getByText("Version 0.1.2")).toBeVisible();
  await expect(
    page.getByText(
      "No analytics or telemetry. Your school data stays private.",
    ),
  ).toBeVisible();
  await page.screenshot({ path: "artifacts/klaso-about-dark.png" });
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-mode", "dark");
});
