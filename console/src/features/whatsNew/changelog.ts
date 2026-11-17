// The changelog entries of `changelog.d/`: parsing, validation, grouping and the generated
// CHANGELOG.md. One module for the console (What's new) and for `scripts/changelog.mjs`, which
// runs it with Node's type stripping, so it imports nothing and uses erasable syntax only.

const ENTRY_TYPES = ["added", "changed", "fixed", "removed", "deprecated", "security"] as const;
export type EntryType = (typeof ENTRY_TYPES)[number];

const IMPACTS = ["high", "medium", "low"] as const;
export type Impact = (typeof IMPACTS)[number];

export const TYPE_LABELS: Record<EntryType, string> = {
  added: "Added",
  changed: "Changed",
  fixed: "Fixed",
  removed: "Removed",
  deprecated: "Deprecated",
  security: "Security",
};

export const IMPACT_HEADINGS: Record<Impact, string> = {
  high: "High impact",
  medium: "Medium impact",
  low: "Low impact",
};

export interface Entry {
  /** The file name in `changelog.d/`, such as `2026-11-17-whats-new.md`. */
  file: string;
  /** `YYYY-MM-DD`: the day the change merged. */
  date: string;
  type: EntryType;
  impact: Impact;
  title: string;
  /** The navigation section users know the change by. */
  area?: string;
  /** A console route, such as `/settings/notifications`. */
  link?: string;
  pr?: number;
  /** The version the change shipped in; absent while it is unreleased. */
  release?: string;
  /** The text before `## Technical notes`. */
  summary: string;
  /** The text under `## Technical notes`, or "". */
  notes: string;
}

export interface Group {
  /** The version, or `null` for the unreleased entries. */
  release: string | null;
  /** Oldest and newest entry dates. */
  from: string;
  to: string;
  sections: { impact: Impact; entries: Entry[] }[];
}

export class ChangelogError extends Error {
  override name = "ChangelogError";
}

const FILE_NAME = /^\d{4}-\d{2}-\d{2}-[a-z0-9]+(?:-[a-z0-9]+)*\.md$/;
const DATE = /^(\d{4})-(\d{2})-(\d{2})$/;
const VERSION = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;
const NOTES_HEADING = /^## Technical notes[ \t]*$/m;
const REQUIRED = ["date", "type", "impact", "title"] as const;
const OPTIONAL = ["area", "link", "pr", "release"] as const;
const KEYS: readonly string[] = [...REQUIRED, ...OPTIONAL];

/** A release version such as `0.2.0` or `1.0.0-rc.1`. */
export function isVersion(value: string): boolean {
  return VERSION.test(value);
}

function isOneOf<T extends string>(values: readonly T[], value: string): value is T {
  return (values as readonly string[]).includes(value);
}

function isCalendarDate(value: string): boolean {
  const match = DATE.exec(value);
  if (!match) return false;
  const [year, month, day] = [Number(match[1]), Number(match[2]), Number(match[3])];
  const date = new Date(Date.UTC(year, month - 1, day));
  return date.getUTCMonth() === month - 1 && date.getUTCDate() === day;
}

/** Reads the flat `key: value` front matter between the two `---` lines. */
function frontMatter(file: string, text: string): { fields: Map<string, string>; body: string } {
  const lines = text.replace(/\r\n/g, "\n").split("\n");
  if (lines[0] !== "---") throw new ChangelogError(`${file}: must start with a "---" line`);
  const end = lines.indexOf("---", 1);
  if (end === -1) throw new ChangelogError(`${file}: front matter has no closing "---" line`);
  const fields = new Map<string, string>();
  for (const line of lines.slice(1, end)) {
    if (line.trim() === "") continue;
    const colon = line.indexOf(":");
    if (colon <= 0) throw new ChangelogError(`${file}: "${line}" is not a "key: value" line`);
    const key = line.slice(0, colon).trim();
    if (!KEYS.includes(key)) {
      throw new ChangelogError(`${file}: unknown key "${key}" (allowed: ${KEYS.join(", ")})`);
    }
    if (fields.has(key)) throw new ChangelogError(`${file}: key "${key}" appears twice`);
    fields.set(key, line.slice(colon + 1).trim());
  }
  return { fields, body: lines.slice(end + 1).join("\n") };
}

/** Per key: the test a value must pass, and what to say when it does not. */
const CHECKS: Record<string, [(value: string) => boolean, string]> = {
  date: [isCalendarDate, "must be a YYYY-MM-DD date"],
  type: [(value) => isOneOf(ENTRY_TYPES, value), `must be one of ${ENTRY_TYPES.join(", ")}`],
  impact: [(value) => isOneOf(IMPACTS, value), `must be one of ${IMPACTS.join(", ")}`],
  title: [(value) => !value.endsWith("."), "ends with a period"],
  link: [(value) => value.startsWith("/"), 'must be a console route starting with "/"'],
  pr: [(value) => /^[1-9]\d*$/.test(value), "must be a pull request number"],
  release: [isVersion, "must be a version such as 0.2.0"],
};

function validate(file: string, fields: Map<string, string>): void {
  for (const key of REQUIRED) {
    if (!fields.has(key)) throw new ChangelogError(`${file}: required key "${key}" is missing`);
  }
  for (const [key, value] of fields) {
    if (value === "") throw new ChangelogError(`${file}: key "${key}" is empty`);
    const [test, problem] = CHECKS[key] ?? [() => true, ""];
    if (!test(value)) throw new ChangelogError(`${file}: key "${key}" ${problem}, not "${value}"`);
  }
}

/** Parses one entry file, or throws a `ChangelogError` naming the file and the key. */
export function parseEntry(file: string, text: string): Entry {
  if (!FILE_NAME.test(file)) {
    throw new ChangelogError(`${file}: the file name must be YYYY-MM-DD-short-slug.md`);
  }
  const { fields, body } = frontMatter(file, text);
  validate(file, fields);
  const heading = NOTES_HEADING.exec(body);
  const summary = (heading ? body.slice(0, heading.index) : body).trim();
  const notes = heading ? body.slice(heading.index + heading[0].length).trim() : "";
  if (summary === "") throw new ChangelogError(`${file}: the summary is missing`);
  const pr = fields.get("pr");
  return {
    file,
    date: fields.get("date") ?? "",
    // Checked by `validate`.
    type: fields.get("type") as EntryType,
    impact: fields.get("impact") as Impact,
    title: fields.get("title") ?? "",
    area: fields.get("area"),
    link: fields.get("link"),
    pr: pr === undefined ? undefined : Number(pr),
    release: fields.get("release"),
    summary,
    notes,
  };
}

/** Parses every entry; the first invalid file fails the whole set. */
export function parseEntries(files: Record<string, string>): Entry[] {
  return Object.entries(files).map(([file, text]) => parseEntry(file, text));
}

function compareText(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

/** Newest version first: numeric parts, then a release before its pre-releases. */
function compareVersionsDescending(a: string, b: string): number {
  const [coreA = "", preA] = a.split(/-(.*)/s);
  const [coreB = "", preB] = b.split(/-(.*)/s);
  const partsA = coreA.split(".").map(Number);
  const partsB = coreB.split(".").map(Number);
  for (let i = 0; i < 3; i++) {
    const diff = (partsB[i] ?? 0) - (partsA[i] ?? 0);
    if (diff !== 0) return diff;
  }
  if (preA === preB) return 0;
  if (preA === undefined) return -1;
  if (preB === undefined) return 1;
  return compareText(preB, preA);
}

/** Newest first, then by title. */
function compareEntries(a: Entry, b: Entry): number {
  return (
    compareText(b.date, a.date) || compareText(a.title, b.title) || compareText(a.file, b.file)
  );
}

/**
 * The groups of the document and of What's new: unreleased first, then each release newest
 * first; inside a group, high to low impact (empty ones left out), newest first, then by title.
 */
export function groupEntries(entries: readonly Entry[]): Group[] {
  const byRelease = new Map<string | null, Entry[]>();
  for (const entry of entries) {
    const key = entry.release ?? null;
    byRelease.set(key, [...(byRelease.get(key) ?? []), entry]);
  }
  const releases = [...byRelease.keys()].sort((a, b) =>
    a === null ? -1 : b === null ? 1 : compareVersionsDescending(a, b),
  );
  return releases.map((release) => {
    const members = [...(byRelease.get(release) ?? [])].sort(compareEntries);
    const dates = members.map((entry) => entry.date).sort();
    return {
      release,
      from: dates[0] ?? "",
      to: dates[dates.length - 1] ?? "",
      sections: IMPACTS.map((impact) => ({
        impact,
        entries: members.filter((entry) => entry.impact === impact),
      })).filter((section) => section.entries.length > 0),
    };
  });
}

export function groupTitle(group: Group): string {
  return group.release ?? "Unreleased";
}

/** `2026-11-12` or `2026-11-12 to 2026-11-17`. */
export function dateRange(group: Group): string {
  return group.from === group.to ? group.from : `${group.from} to ${group.to}`;
}

const REPOSITORY_URL = "https://github.com/openbga/interface-engine";

function indent(text: string): string {
  return text
    .split("\n")
    .map((line) => (line.trim() === "" ? "" : `  ${line}`))
    .join("\n");
}

function renderEntry(entry: Entry): string {
  const pr = entry.pr === undefined ? "" : `, [#${entry.pr}](${REPOSITORY_URL}/pull/${entry.pr})`;
  const lines = [
    `- **${TYPE_LABELS[entry.type]}:** ${entry.title} (${entry.date}${pr})`,
    "",
    indent(entry.summary),
  ];
  if (entry.notes !== "") lines.push("", "  Technical notes:", "", indent(entry.notes));
  return lines.join("\n");
}

export const GENERATED_NOTICE =
  "<!-- Generated from changelog.d/ by `npm run changelog` in console/. Do not edit by hand:\n" +
  "     add or change a file in changelog.d/ instead (CONTRIBUTING.md, Changelog entries). -->";

/** The whole CHANGELOG.md. */
export function renderChangelog(entries: readonly Entry[]): string {
  const parts = [
    GENERATED_NOTICE,
    "",
    "# Changelog",
    "",
    "What changed in OpenBGA Interface Engine, grouped by release with unreleased changes on",
    "top, then by impact. Types follow [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).",
  ];
  for (const group of groupEntries(entries)) {
    parts.push("", `## ${groupTitle(group)} (${dateRange(group)})`);
    for (const section of group.sections) {
      parts.push("", `### ${IMPACT_HEADINGS[section.impact]}`);
      for (const entry of section.entries) parts.push("", renderEntry(entry));
    }
  }
  return `${parts.join("\n")}\n`;
}

/** Adds `release: <version>` to an entry file that has no release yet; otherwise unchanged. */
export function stampRelease(text: string, version: string): string {
  if (!isVersion(version)) throw new ChangelogError(`"${version}" is not a version`);
  const lines = text.split("\n");
  const end = lines.indexOf("---", 1);
  if (lines[0] !== "---" || end === -1) throw new ChangelogError("the entry has no front matter");
  if (lines.slice(1, end).some((line) => /^release\s*:/.test(line))) return text;
  lines.splice(end, 0, `release: ${version}`);
  return lines.join("\n");
}
