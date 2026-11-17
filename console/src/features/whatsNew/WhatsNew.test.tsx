import { screen, within } from "@testing-library/react";
import { matchPath, type RouteObject } from "react-router";
import { afterEach, describe, expect, it } from "vitest";
import { routes } from "@/app/routes";
import { SETTINGS_TAB_IDS } from "@/features/settings/SettingsPage";
import { expectNoA11yViolations } from "@/lib/test/a11y";
import { renderRoutes } from "@/lib/test/router";
import type { Entry } from "./changelog";
import { changelog as bundled, type Changelog, readChangelog } from "./entries";
import { markerFor, readSeen, writeSeen } from "./seen";
import { screenLabel, WhatsNew, whatsNewLabel } from "./WhatsNew";

function file(fields: string, summary = "Synthetic summary with `a_setting`."): string {
  return `---\n${fields}\n---\n${summary}\n\n## Technical notes\n\nNot shown in the console.\n`;
}

const FILES = {
  "2026-11-10-old.md": file(
    "date: 2026-11-10\ntype: fixed\nimpact: low\ntitle: An old fix\nrelease: 0.1.0",
  ),
  "2026-11-12-seen.md": file(
    "date: 2026-11-12\ntype: changed\nimpact: medium\ntitle: A change already seen",
  ),
  "2026-11-17-new.md": file(
    "date: 2026-11-17\ntype: added\nimpact: high\ntitle: A new capability\narea: Settings\nlink: /settings/backup",
  ),
};

const changelog = readChangelog(FILES);
const entries = changelog.entries as Entry[];

function renderButton(list: Changelog = changelog) {
  return renderRoutes(
    [{ path: "/dashboard", element: <WhatsNew changelog={list} /> }],
    "/dashboard",
  );
}

describe("What's new", () => {
  afterEach(() => localStorage.clear());

  it("names the unread count in the button's name", () => {
    expect(whatsNewLabel(0)).toBe("What's new");
    expect(whatsNewLabel(1)).toBe("What's new: 1 new change");
    expect(whatsNewLabel(3)).toBe("What's new: 3 new changes");
  });

  it("shows the changes not seen yet, and opening the list clears the count", async () => {
    writeSeen(
      markerFor(entries.filter((each) => each.date <= "2026-11-12")) ?? { date: "", files: [] },
    );
    const { user } = renderButton();
    const button = screen.getByRole("button", { name: "What's new: 1 new change" });
    expect(button.textContent).toBe("What's new1");
    await user.click(button);
    const dialog = screen.getByRole("dialog", { name: "What's new" });
    // Only the change that was unread carries New while the list is open.
    const newEntry = within(dialog).getByRole("heading", { name: /A new capability/ });
    expect(newEntry.textContent).toBe("Added A new capability New");
    const seenEntry = within(dialog).getByRole("heading", { name: /A change already seen/ });
    expect(seenEntry.textContent).not.toContain("New");
    await expectNoA11yViolations(dialog);
    await user.click(within(dialog).getByRole("button", { name: "Close" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByRole("button", { name: "What's new" }).textContent).toBe("What's new");
    // Remembered: a second console shows no count either.
    renderButton();
    expect(screen.getAllByRole("button", { name: "What's new" })).toHaveLength(2);
  });

  it("shows no count on a first visit and records the newest change as seen at once", () => {
    renderButton();
    expect(screen.getByRole("button", { name: "What's new" }).textContent).toBe("What's new");
    expect(readSeen()).toEqual(markerFor(entries));
  });

  it("lists every entry in the document's order, with no New marks on a first visit", async () => {
    const { user } = renderButton();
    await user.click(screen.getByRole("button", { name: "What's new" }));
    const dialog = screen.getByRole("dialog");
    const headings = within(dialog)
      .getAllByRole("heading")
      .map((heading) => heading.textContent);
    expect(headings).toEqual([
      "What's new",
      "Unreleased 2026-11-12 to 2026-11-17",
      "High impact",
      "Added A new capability",
      "Medium impact",
      "Changed A change already seen",
      "0.1.0 2026-11-10",
      "Low impact",
      "Fixed An old fix",
    ]);
    expect(within(dialog).getAllByText("a_setting", { selector: "code" })).toHaveLength(3);
    const dated = within(dialog).getByText("2026-11-17", { selector: "time" });
    expect(dated.parentElement?.textContent).toBe("2026-11-17 · Settings");
    expect(within(dialog).queryByText(/Not shown in the console/)).toBeNull();
  });

  it("opens an entry's page and closes the list", async () => {
    const { user, router } = renderButton();
    await user.click(screen.getByRole("button", { name: /What's new/ }));
    await user.click(screen.getByRole("link", { name: "Open Settings" }));
    expect(router.state.location.pathname).toBe("/settings/backup");
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("says so when there are no changes, and shows no count", async () => {
    const { user } = renderButton({ entries: [], error: null });
    await user.click(screen.getByRole("button", { name: "What's new" }));
    expect(screen.getByText("No changes recorded yet")).toBeTruthy();
  });

  it("explains an entry that cannot be read", async () => {
    const broken = readChangelog({ "2026-11-17-bad.md": file("date: 2026-11-17\ntype: new") });
    expect(broken.error).toContain("2026-11-17-bad.md");
    const { user } = renderButton(broken);
    await user.click(screen.getByRole("button", { name: "What's new" }));
    expect(screen.getByText("The list of changes could not be read")).toBeTruthy();
    expect(screen.getByText(/2026-11-17-bad\.md/)).toBeTruthy();
  });

  it("labels a link by the screen it opens", () => {
    expect(screenLabel("/settings/notifications")).toBe("Settings");
    expect(screenLabel("/messages")).toBe("Messages");
    expect(screenLabel("/nowhere")).toBe("the page");
  });
});

/** Every route pattern of the app. */
function patternsOf(list: readonly RouteObject[]): string[] {
  return list.flatMap((route) => [
    ...(route.path !== undefined && route.path !== "*" ? [route.path] : []),
    ...patternsOf(route.children ?? []),
  ]);
}

describe("the bundled changelog.d entries", () => {
  it("all parse", () => {
    expect(bundled.error).toBeNull();
    expect(bundled.entries.length).toBeGreaterThan(0);
  });

  it.each(
    bundled.entries.filter((each) => each.link !== undefined).map((each) => [each.file, each.link]),
  )("%s links to a console route", (_file, link) => {
    const patterns = patternsOf(routes);
    const tab = link?.match(/^\/settings\/([^/]+)$/)?.[1];
    if (tab !== undefined) expect(SETTINGS_TAB_IDS).toContain(tab);
    expect(
      patterns.some((pattern) => matchPath({ path: pattern, end: true }, link ?? "") !== null),
    ).toBe(true);
  });
});
