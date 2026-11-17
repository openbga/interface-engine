import type { Entry } from "./changelog";

/**
 * The newest changes this console has shown on this computer: the newest entry date, and the
 * entry files of that date (several changes can merge on one day).
 */
export interface SeenMarker {
  date: string;
  files: string[];
}

const STORAGE_KEY = "packie.whats-new.seen-v1";

/** Entries newer than the newest one seen; none without a marker (there are no entries). */
export function isUnread(entry: Entry, seen: SeenMarker | null): boolean {
  if (seen === null) return false;
  return entry.date > seen.date || (entry.date === seen.date && !seen.files.includes(entry.file));
}

export function unreadCount(entries: readonly Entry[], seen: SeenMarker | null): number {
  return entries.filter((entry) => isUnread(entry, seen)).length;
}

/** The marker after the list has been shown; `null` when there is nothing to remember. */
export function markerFor(entries: readonly Entry[]): SeenMarker | null {
  const newest = entries.reduce<string | null>(
    (date, entry) => (date === null || entry.date > date ? entry.date : date),
    null,
  );
  if (newest === null) return null;
  const files = entries.filter((entry) => entry.date === newest).map((entry) => entry.file);
  return { date: newest, files: files.sort() };
}

function isMarker(value: unknown): value is SeenMarker {
  return (
    typeof value === "object" &&
    value !== null &&
    "date" in value &&
    typeof value.date === "string" &&
    "files" in value &&
    Array.isArray(value.files) &&
    value.files.every((file: unknown) => typeof file === "string")
  );
}

export function readSeen(): SeenMarker | null {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null");
    return isMarker(value) ? value : null;
  } catch {
    // A damaged or unavailable store counts as a first visit: nothing shows as new.
    return null;
  }
}

/**
 * The marker for this console. On a first visit (no marker yet) the newest entry is recorded as
 * seen at once: nothing is new to someone who has not used the console before, so the
 * indicator only ever means "changed since you last looked".
 */
export function seenOnArrival(entries: readonly Entry[]): SeenMarker | null {
  const stored = readSeen();
  if (stored !== null) return stored;
  const marker = markerFor(entries);
  if (marker !== null) writeSeen(marker);
  return marker;
}

export function writeSeen(marker: SeenMarker): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(marker));
  } catch {
    // Not remembered; the indicator returns next time, which is harmless.
  }
}
