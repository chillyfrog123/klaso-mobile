import { test, expect, type Locator, type Page } from "@playwright/test";
import data from "../src/demo.json" with { type: "json" };

const firstMessage = data.Messages.Messages.items[0];
const selected = "Spotykamy się przed szkołą o godzinie 8:00.";

async function openMessage(page: Page, polish = false) {
  await page.goto("/");
  await page
    .getByRole("button", { name: polish ? "Profil" : "Profile", exact: true })
    .first()
    .click();
  await page
    .getByRole("button", {
      name: polish ? "Otwórz demo offline" : "Explore offline demo",
      exact: true,
    })
    .click();
  await page
    .getByRole("button", {
      name: polish ? "Wiadomości" : "Messages",
      exact: true,
    })
    .click();
  await page.locator(".mail-card").first().click();
  const body = page.locator(".reader .message-body");
  await expect(body).toHaveText(firstMessage.content);
  return body;
}

async function selectText(body: Locator, text: string) {
  const actual = await body.evaluate((element, wanted) => {
    const start = element.textContent!.indexOf(wanted);
    if (start < 0) throw new Error("Selection text is absent from the message");
    const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT);
    const range = document.createRange();
    let offset = 0;
    let started = false;
    while (walker.nextNode()) {
      const node = walker.currentNode;
      const end = offset + node.textContent!.length;
      if (!started && start < end) {
        range.setStart(node, start - offset);
        started = true;
      }
      if (started && start + wanted.length <= end) {
        range.setEnd(node, start + wanted.length - offset);
        break;
      }
      offset = end;
    }
    const selection = window.getSelection()!;
    selection.removeAllRanges();
    selection.addRange(range);
    return selection.toString();
  }, text);
  expect(actual).toBe(text);
}

async function contextMenu(body: Locator) {
  return body.evaluate((element) => {
    const event = new MouseEvent("contextmenu", {
      bubbles: true,
      cancelable: true,
      button: 2,
    });
    element.dispatchEvent(event);
    return event.defaultPrevented;
  });
}

test("selected message text opens themed actions and copies the captured text", async ({
  page,
  context,
}) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  const body = await openMessage(page);
  await selectText(body, selected);
  expect(await contextMenu(body)).toBe(true);
  const dialog = page.getByRole("dialog", {
    name: "Selected text",
    exact: true,
  });
  await expect(dialog).toBeVisible();
  await expect(dialog).toHaveClass(/\bchoice\b/);
  await expect(dialog.getByText(selected, { exact: true })).toBeVisible();
  await dialog.getByRole("button", { name: "Copy", exact: true }).click();
  await expect(dialog).not.toBeVisible();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(
    selected,
  );
});

test("select all is restricted to the current message body", async ({
  page,
  context,
}) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  const body = await openMessage(page);
  await selectText(body, selected);
  await contextMenu(body);
  const dialog = page.getByRole("dialog", {
    name: "Selected text",
    exact: true,
  });
  await dialog.getByRole("button", { name: "Select all", exact: true }).click();
  await expect(
    dialog.getByText(firstMessage.content, { exact: true }),
  ).toBeVisible();
  await dialog.getByRole("button", { name: "Copy", exact: true }).click();
  // Windows' system clipboard exposes line endings as CRLF.
  const copied = await page.evaluate(() => navigator.clipboard.readText());
  expect(copied.replace(/\r\n/g, "\n")).toBe(firstMessage.content);
});

test("selection actions reopen after Close, Escape, and backdrop dismissal", async ({
  page,
}) => {
  const body = await openMessage(page);
  const dialog = page.getByRole("dialog", {
    name: "Selected text",
    exact: true,
  });
  for (const dismissal of ["Close", "Escape", "backdrop"] as const) {
    await selectText(body, selected);
    await contextMenu(body);
    await expect(dialog).toBeVisible();
    if (dismissal === "Close")
      await dialog.getByRole("button", { name: "Close", exact: true }).click();
    else if (dismissal === "Escape") await page.keyboard.press("Escape");
    else await page.mouse.click(2, 2);
    await expect(dialog).not.toBeVisible();
  }
});

test("native selection keeps adjustable handles until the themed actions are requested", async ({
  page,
  context,
}) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  const body = await openMessage(page);
  const dialog = page.getByRole("dialog", {
    name: "Selected text",
    exact: true,
  });
  const trigger = page.getByRole("button", {
    name: "Selected text",
    exact: true,
  });
  await selectText(body, selected);
  const accepted = await page.evaluate(() => {
    const event = new Event("klaso-text-selection", { cancelable: true });
    document.dispatchEvent(event);
    return event.defaultPrevented;
  });
  expect(accepted).toBe(true);
  await expect(dialog).not.toBeVisible();
  await expect(trigger).toBeVisible();
  expect(await page.evaluate(() => window.getSelection()?.toString())).toBe(
    selected,
  );

  const expanded =
    "przypominam o naszej wycieczce klasowej. Spotykamy się przed szkołą o godzinie 8:00.";
  await selectText(body, expanded);
  await page.evaluate(() =>
    document.dispatchEvent(new Event("selectionchange")),
  );
  await trigger.click();
  await expect(dialog).toBeVisible();
  await expect(dialog.getByText(expanded, { exact: true })).toBeVisible();
  await dialog.getByRole("button", { name: "Copy", exact: true }).click();
  await expect(dialog).not.toBeVisible();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(
    expanded,
  );

  await selectText(body, selected);
  await page.evaluate(() =>
    document.dispatchEvent(
      new Event("klaso-text-selection", { cancelable: true }),
    ),
  );
  await expect(trigger).toBeVisible();
  await page.evaluate(() => {
    window.getSelection()?.removeAllRanges();
    document.dispatchEvent(new Event("selectionchange"));
  });
  await expect(trigger).not.toBeVisible();

  await selectText(body, selected);
  await page.evaluate(() =>
    document.dispatchEvent(
      new Event("klaso-text-selection", { cancelable: true }),
    ),
  );
  await expect(trigger).toBeVisible();
  await page.evaluate(() =>
    document.dispatchEvent(new Event("klaso-text-selection-ended")),
  );
  await expect(trigger).not.toBeVisible();
  await expect(dialog).not.toBeVisible();
});

test("Escape dismisses native text selection before leaving the message", async ({
  page,
}) => {
  const body = await openMessage(page);
  await selectText(body, selected);
  await page.evaluate(() =>
    document.dispatchEvent(
      new Event("klaso-text-selection", { cancelable: true }),
    ),
  );
  const trigger = page.getByRole("button", {
    name: "Selected text",
    exact: true,
  });
  await expect(trigger).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(trigger).not.toBeVisible();
  await expect(body).toBeVisible();
  await expect(body).toHaveText(firstMessage.content);
  expect(await page.evaluate(() => window.getSelection()?.isCollapsed)).toBe(
    true,
  );
});

test("Share forwards only the selected text to the platform share sheet", async ({
  page,
}) => {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, "share", {
      configurable: true,
      value: async (payload: ShareData) => {
        (window as typeof window & { sharedText?: string }).sharedText =
          payload.text;
      },
    });
  });
  const body = await openMessage(page);
  await selectText(body, selected);
  await contextMenu(body);
  const dialog = page.getByRole("dialog", {
    name: "Selected text",
    exact: true,
  });
  await dialog.getByRole("button", { name: "Share", exact: true }).click();
  await expect
    .poll(() =>
      page.evaluate(
        () => (window as typeof window & { sharedText?: string }).sharedText,
      ),
    )
    .toBe(selected);
  await expect(dialog).not.toBeVisible();
});

test("editable inputs and empty message selections retain their standard menus", async ({
  page,
}) => {
  await page.goto("/");
  await page
    .getByRole("button", { name: "Profile", exact: true })
    .first()
    .click();
  const input = page.getByRole("textbox", { name: "Login", exact: true });
  await input.fill("fictional-user");
  await input.evaluate((element: HTMLInputElement) => {
    element.focus();
    element.setSelectionRange(0, 9);
  });
  expect(await contextMenu(input)).toBe(false);
  await expect(
    page.getByRole("dialog", { name: "Selected text" }),
  ).not.toBeVisible();
  const body = await openMessage(page);
  await page.evaluate(() => window.getSelection()?.removeAllRanges());
  expect(await contextMenu(body)).toBe(false);
  await expect(
    page.getByRole("dialog", { name: "Selected text" }),
  ).not.toBeVisible();
});

test("Polish dark selection actions fit a narrow phone with long selected content", async ({
  page,
}) => {
  await page.setViewportSize({ width: 320, height: 700 });
  await page.addInitScript(() => {
    localStorage.setItem("language", "pl");
    localStorage.setItem("theme", "dark");
  });
  const body = await openMessage(page, true);
  // Stress the displayed document without changing the demo or production data.
  const longText = "Zażółć gęślą jaźń. ".repeat(500);
  await body.evaluate((element, text) => {
    element.textContent = text;
  }, longText);
  await selectText(body, longText);
  await contextMenu(body);
  const dialog = page.getByRole("dialog", {
    name: "Zaznaczony tekst",
    exact: true,
  });
  await expect(dialog).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute("data-mode", "dark");
  const bounds = await dialog.boundingBox();
  expect(bounds).not.toBeNull();
  expect(bounds!.x).toBeGreaterThanOrEqual(0);
  expect(bounds!.y).toBeGreaterThanOrEqual(0);
  expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(320);
  expect(bounds!.y + bounds!.height).toBeLessThanOrEqual(700);
  expect(
    await dialog.evaluate(
      (element) => element.scrollWidth <= element.clientWidth,
    ),
  ).toBe(true);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await dialog
    .getByRole("button", { name: "Kopiuj", exact: true })
    .scrollIntoViewIfNeeded();
  await expect(
    dialog.getByRole("button", { name: "Kopiuj", exact: true }),
  ).toBeVisible();
  await dialog
    .getByRole("button", { name: "Zaznacz wszystko", exact: true })
    .scrollIntoViewIfNeeded();
  await expect(
    dialog.getByRole("button", { name: "Zaznacz wszystko", exact: true }),
  ).toBeVisible();
  await dialog.getByRole("button", { name: "Zamknij", exact: true }).click();
  await expect(dialog).not.toBeVisible();
});
