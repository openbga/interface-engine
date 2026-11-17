import { type Entry, parseEntries } from "./changelog";

// The entry files of `changelog.d/` at the repository root, bundled at build time: the list
// ships with the console and nothing is fetched.
const modules = import.meta.glob<string>("../../../../changelog.d/*.md", {
  query: "?raw",
  import: "default",
  eager: true,
});

export type Changelog = { entries: Entry[]; error: null } | { entries: []; error: string };

/** Entry files keyed by file name, such as `2026-11-17-whats-new.md`. */
function entryFiles(): Record<string, string> {
  return Object.fromEntries(
    Object.entries(modules).map(([path, text]) => [path.slice(path.lastIndexOf("/") + 1), text]),
  );
}

/** Every entry, or the first file's problem (CI rejects such a file before it ships). */
export function readChangelog(files: Record<string, string> = entryFiles()): Changelog {
  try {
    return { entries: parseEntries(files), error: null };
  } catch (error) {
    return { entries: [], error: error instanceof Error ? error.message : String(error) };
  }
}

export const changelog: Changelog = readChangelog();
