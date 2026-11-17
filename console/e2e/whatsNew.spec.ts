import { expect, test } from "@playwright/test";
import { loaded, open } from "./support";

test("What's new: the top bar counts unseen changes, the list shows them, and opening it clears the count", async ({
  page,
}) => {
  await open(page, "/dashboard");
  await loaded(page, "Dashboard");
  const banner = page.getByRole("banner");
  // A first visit counts nothing: the newest change is recorded as seen at once.
  await expect(banner.getByRole("button", { name: "What's new", exact: true })).toBeVisible();
  expect(
    await page.evaluate(() => localStorage.getItem("packie.whats-new.seen-v1")),
  ).not.toBeNull();

  // A returning operator who last looked before the newest changes sees them counted.
  await page.evaluate(() =>
    localStorage.setItem(
      "packie.whats-new.seen-v1",
      JSON.stringify({ date: "2026-11-12", files: [] }),
    ),
  );
  await page.reload();
  await loaded(page, "Dashboard");
  const unread = banner.getByRole("button", { name: /^What's new: \d+ new changes?$/ });
  await expect(unread).toBeVisible();
  // The tooltip says the same. React Aria opens it on keyboard focus (and on a real pointer's
  // hover, which a synthetic hover does not trigger).
  await page.keyboard.press("Shift");
  await unread.focus();
  await expect(page.getByRole("tooltip")).toHaveText(/^What's new: \d+ new changes?$/);

  await unread.click();
  const dialog = page.getByRole("dialog", { name: "What's new" });
  await expect(dialog.getByRole("heading", { level: 3, name: /^Unreleased / })).toBeVisible();
  await expect(
    dialog.getByRole("heading", { level: 4, name: "High impact" }).first(),
  ).toBeVisible();
  await expect(
    dialog.getByRole("heading", { level: 5, name: "Added See what changed in What's new" }),
  ).toBeVisible();

  // An entry's link opens its page and closes the list.
  await dialog.getByRole("link", { name: "Open Settings" }).first().click();
  await expect(dialog).toBeHidden();
  await expect(page.getByRole("heading", { level: 1, name: "Settings" })).toBeVisible();
  await expect(banner.getByRole("button", { name: "What's new", exact: true })).toBeVisible();

  // Remembered on this computer: a reload shows no count, and Escape closes the list.
  await page.reload();
  await loaded(page, "Settings");
  const read = banner.getByRole("button", { name: "What's new", exact: true });
  await read.click();
  await expect(dialog).toBeVisible();
  await expect(dialog.getByText("New", { exact: true })).toHaveCount(0);
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
  await expect(read).toBeFocused();
});
