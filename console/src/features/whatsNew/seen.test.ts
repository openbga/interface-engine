import { afterEach, describe, expect, it } from "vitest";
import type { Entry } from "./changelog";
import { isUnread, markerFor, readSeen, seenOnArrival, unreadCount, writeSeen } from "./seen";

function entry(file: string, date: string): Entry {
  return { file, date, type: "added", impact: "low", title: "T", summary: "S", notes: "" };
}

const entries = [
  entry("2026-11-10-a.md", "2026-11-10"),
  entry("2026-11-12-b.md", "2026-11-12"),
  entry("2026-11-12-c.md", "2026-11-12"),
];

describe("the unread indicator", () => {
  afterEach(() => localStorage.clear());

  it("records the newest entry as seen on a first visit, so nothing counts as new", () => {
    expect(readSeen()).toBeNull();
    const marker = seenOnArrival(entries);
    expect(marker).toEqual(markerFor(entries));
    expect(readSeen()).toEqual(marker);
    expect(unreadCount(entries, marker)).toBe(0);
  });

  it("keeps an existing marker on a later visit", () => {
    const older = markerFor(entries.slice(0, 1));
    if (older) writeSeen(older);
    expect(seenOnArrival(entries)).toEqual(older);
    expect(unreadCount(entries, older)).toBe(2);
  });

  it("counts nothing without a marker (an empty list)", () => {
    expect(seenOnArrival([])).toBeNull();
    expect(unreadCount(entries, null)).toBe(0);
  });

  it("remembers the newest date and that day's files, and then counts none", () => {
    const marker = markerFor(entries);
    expect(marker).toEqual({ date: "2026-11-12", files: ["2026-11-12-b.md", "2026-11-12-c.md"] });
    expect(unreadCount(entries, marker)).toBe(0);
  });

  it("counts a later entry, and a new entry on the newest seen day", () => {
    const marker = markerFor(entries);
    const later = entry("2026-11-13-d.md", "2026-11-13");
    const sameDay = entry("2026-11-12-e.md", "2026-11-12");
    expect(unreadCount([...entries, later, sameDay], marker)).toBe(2);
    expect(isUnread(sameDay, marker)).toBe(true);
  });

  it("does not count an entry older than the newest one seen", () => {
    expect(isUnread(entry("2026-11-01-z.md", "2026-11-01"), markerFor(entries))).toBe(false);
  });

  it("has nothing to remember for an empty list", () => {
    expect(markerFor([])).toBeNull();
  });

  it("keeps the marker in browser storage and ignores a damaged value", () => {
    expect(readSeen()).toBeNull();
    writeSeen({ date: "2026-11-12", files: ["2026-11-12-b.md"] });
    expect(readSeen()).toEqual({ date: "2026-11-12", files: ["2026-11-12-b.md"] });
    localStorage.setItem("packie.whats-new.seen-v1", "{not json");
    expect(readSeen()).toBeNull();
    localStorage.setItem("packie.whats-new.seen-v1", JSON.stringify({ date: 3, files: [] }));
    expect(readSeen()).toBeNull();
  });
});
