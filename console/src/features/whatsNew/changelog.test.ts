import { describe, expect, it } from "vitest";
import {
  dateRange,
  type Entry,
  GENERATED_NOTICE,
  groupEntries,
  parseEntries,
  parseEntry,
  renderChangelog,
  stampRelease,
} from "./changelog";

const VALID = `---
date: 2026-11-17
type: added
impact: high
title: Reject a sample with a recorded reason
area: Messages
link: /messages
pr: 45
---
One plain sentence for operators.

A second paragraph.

## Technical notes

- A setting \`example_key\`.
`;

function entry(fields: Partial<Entry> & Pick<Entry, "file">): Entry {
  return {
    date: "2026-11-12",
    type: "added",
    impact: "medium",
    title: "Synthetic change",
    summary: "Summary.",
    notes: "",
    ...fields,
  };
}

function withField(key: string, value: string | null): string {
  const lines = VALID.split("\n").filter((line) => !line.startsWith(`${key}:`));
  if (value !== null) lines.splice(1, 0, `${key}: ${value}`);
  return lines.join("\n");
}

describe("parseEntry", () => {
  it("reads the front matter, the summary and the technical notes", () => {
    expect(parseEntry("2026-11-17-reject-sample.md", VALID)).toEqual({
      file: "2026-11-17-reject-sample.md",
      date: "2026-11-17",
      type: "added",
      impact: "high",
      title: "Reject a sample with a recorded reason",
      area: "Messages",
      link: "/messages",
      pr: 45,
      release: undefined,
      summary: "One plain sentence for operators.\n\nA second paragraph.",
      notes: "- A setting `example_key`.",
    });
  });

  it("accepts an entry without optional keys or notes, and Windows line endings", () => {
    const text =
      "---\r\ndate: 2026-01-31\r\ntype: fixed\r\nimpact: low\r\ntitle: A fix\r\n---\r\nText.\r\n";
    const parsed = parseEntry("2026-01-31-fix.md", text);
    expect(parsed).toMatchObject({ type: "fixed", impact: "low", summary: "Text.", notes: "" });
    expect(parsed.area).toBeUndefined();
  });

  it.each([
    ["type", "improved", 'key "type" must be one of added, changed, fixed'],
    ["impact", "critical", 'key "impact" must be one of high, medium, low'],
    ["date", "17/11/2026", 'key "date" must be a YYYY-MM-DD date'],
    ["date", "2026-02-30", 'key "date" must be a YYYY-MM-DD date'],
    ["title", "Ends with a period.", 'key "title" ends with a period'],
    ["link", "messages", 'key "link" must be a console route'],
    ["pr", "#45", 'key "pr" must be a pull request number'],
    ["release", "next", 'key "release" must be a version'],
    ["area", "", 'key "area" is empty'],
    ["impcat", "high", 'unknown key "impcat"'],
  ])("rejects %s: %s", (key, value, message) => {
    expect(() => parseEntry("2026-11-17-bad.md", withField(key, value))).toThrow(
      `2026-11-17-bad.md: ${message}`,
    );
  });

  it.each(["date", "type", "impact", "title"])("rejects a missing %s", (key) => {
    expect(() => parseEntry("2026-11-17-bad.md", withField(key, null))).toThrow(
      `2026-11-17-bad.md: required key "${key}" is missing`,
    );
  });

  it("rejects a repeated key, a missing summary and malformed front matter", () => {
    expect(() => parseEntry("2026-11-17-x.md", VALID.replace("pr: 45", "type: fixed"))).toThrow(
      'key "type" appears twice',
    );
    expect(() =>
      parseEntry(
        "2026-11-17-x.md",
        VALID.replace(/---\nOne[\s\S]*$/, "---\n\n## Technical notes\n\nx"),
      ),
    ).toThrow("the summary is missing");
    expect(() => parseEntry("2026-11-17-x.md", VALID.slice(4))).toThrow('must start with a "---"');
    expect(() => parseEntry("2026-11-17-x.md", "---\ndate: 2026-11-17\n")).toThrow("no closing");
    expect(() => parseEntry("2026-11-17-x.md", VALID.replace("pr: 45", "just text"))).toThrow(
      '"just text" is not a "key: value" line',
    );
  });

  it("rejects a file name that is not YYYY-MM-DD-slug.md", () => {
    for (const name of [
      "whats-new.md",
      "2026-11-17.md",
      "2026-11-17-Whats_New.md",
      "2026-11-17-x.txt",
    ]) {
      expect(() => parseEntry(name, VALID)).toThrow(`${name}: the file name must be`);
    }
  });

  it("fails the whole set on the first invalid file", () => {
    expect(() =>
      parseEntries({ "2026-11-17-ok.md": VALID, "2026-11-17-bad.md": withField("type", "x") }),
    ).toThrow("2026-11-17-bad.md");
  });
});

describe("groupEntries", () => {
  const entries = [
    entry({ file: "2026-01-05-a.md", date: "2026-01-05", release: "0.9.0", impact: "low" }),
    entry({ file: "2026-03-01-b.md", date: "2026-03-01", release: "0.10.0", impact: "high" }),
    entry({ file: "2026-02-20-c.md", date: "2026-02-20", release: "0.10.0", impact: "low" }),
    entry({ file: "2026-04-02-d.md", date: "2026-04-02", impact: "medium", title: "Beta" }),
    entry({ file: "2026-04-02-e.md", date: "2026-04-02", impact: "medium", title: "Alpha" }),
    entry({ file: "2026-04-09-f.md", date: "2026-04-09", impact: "medium", title: "Zulu" }),
    entry({ file: "2026-03-30-g.md", date: "2026-03-30", impact: "high" }),
    entry({ file: "2026-02-01-h.md", date: "2026-02-01", release: "0.10.0-rc.1" }),
  ];
  const groups = groupEntries(entries);

  it("puts unreleased first, then releases newest first by version number", () => {
    expect(groups.map((group) => group.release)).toEqual([null, "0.10.0", "0.10.0-rc.1", "0.9.0"]);
  });

  it("gives each group the date range of its entries", () => {
    expect(groups.map(dateRange)).toEqual([
      "2026-03-30 to 2026-04-09",
      "2026-02-20 to 2026-03-01",
      "2026-02-01",
      "2026-01-05",
    ]);
  });

  it("orders impact high to low, leaves out empty ones, then newest first and by title", () => {
    const [unreleased] = groups;
    expect(unreleased?.sections.map((section) => section.impact)).toEqual(["high", "medium"]);
    expect(unreleased?.sections[1]?.entries.map((each) => each.title)).toEqual([
      "Zulu",
      "Alpha",
      "Beta",
    ]);
    expect(groups[1]?.sections.map((section) => section.impact)).toEqual(["high", "low"]);
  });

  it("returns no groups for no entries", () => {
    expect(groupEntries([])).toEqual([]);
  });
});

describe("renderChangelog", () => {
  const text = renderChangelog([
    parseEntry("2026-11-17-reject-sample.md", VALID),
    entry({ file: "2026-11-12-old.md", release: "0.1.0", type: "fixed", impact: "low" }),
  ]);

  it("starts with the do-not-edit notice and groups by release with date ranges", () => {
    expect(text.startsWith(GENERATED_NOTICE)).toBe(true);
    expect(text).toContain("## Unreleased (2026-11-17)");
    expect(text).toContain("## 0.1.0 (2026-11-12)");
    expect(text.indexOf("## Unreleased")).toBeLessThan(text.indexOf("## 0.1.0"));
    expect(text).toContain("### High impact");
    expect(text).toContain("### Low impact");
    expect(text).not.toContain("### Medium impact");
  });

  it("shows type, title, date and pull request, then the summary and indented notes", () => {
    expect(text).toContain(
      "- **Added:** Reject a sample with a recorded reason (2026-11-17, [#45](https://github.com/openbga/interface-engine/pull/45))\n\n" +
        "  One plain sentence for operators.\n\n  A second paragraph.\n\n" +
        "  Technical notes:\n\n  - A setting `example_key`.\n",
    );
    expect(text).toContain("- **Fixed:** Synthetic change (2026-11-12)\n\n  Summary.\n");
    expect(text.endsWith("\n")).toBe(true);
    expect(text.endsWith("\n\n")).toBe(false);
  });
});

describe("stampRelease", () => {
  it("adds the release to an unreleased entry, inside the front matter", () => {
    const stamped = stampRelease(VALID, "0.3.0");
    expect(parseEntry("2026-11-17-reject-sample.md", stamped).release).toBe("0.3.0");
    expect(stamped.replace("release: 0.3.0\n", "")).toBe(VALID);
  });

  it("leaves an entry that already has a release unchanged", () => {
    const released = withField("release", "0.2.0");
    expect(stampRelease(released, "0.3.0")).toBe(released);
  });

  it("refuses a version that is not one", () => {
    expect(() => stampRelease(VALID, "v1")).toThrow('"v1" is not a version');
  });
});
