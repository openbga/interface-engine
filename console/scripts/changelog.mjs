// Generates CHANGELOG.md from changelog.d/ (CONTRIBUTING.md, Changelog entries).
//
//   node scripts/changelog.mjs                    write CHANGELOG.md
//   node scripts/changelog.mjs --check            exit 1 if an entry is invalid or CHANGELOG.md is stale
//   node scripts/changelog.mjs --release 0.3.0    stamp `release: 0.3.0` on unreleased entries, then write
//   --root <dir>                                  repository root (default: the parent of console/)
//
// Node runs the shared TypeScript module directly (type stripping, Node 22.18 or later).
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  ChangelogError,
  isVersion,
  parseEntries,
  renderChangelog,
  stampRelease,
} from "../src/features/whatsNew/changelog.ts";

function fail(message) {
  process.stderr.write(`changelog: ${message}\n`);
  process.exit(1);
}

function options(args) {
  const parsed = {
    check: false,
    release: null,
    root: resolve(dirname(fileURLToPath(import.meta.url)), "..", ".."),
  };
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === "--check") parsed.check = true;
    else if (arg === "--release" && args[i + 1]) parsed.release = args[++i];
    else if (arg === "--root" && args[i + 1]) parsed.root = resolve(args[++i]);
    else fail(`unknown argument "${arg}"; use --check, --release <version> or --root <dir>`);
  }
  if (parsed.check && parsed.release) fail("--check and --release cannot be combined");
  if (parsed.release && !isVersion(parsed.release)) fail(`"${parsed.release}" is not a version`);
  return parsed;
}

function readEntries(directory) {
  const files = {};
  for (const name of readdirSync(directory).sort()) {
    if (name.endsWith(".md")) files[name] = readFileSync(join(directory, name), "utf8");
  }
  return files;
}

/** Adds `release: <version>` to every unreleased entry, on disk and in `files`. */
function stampAll(directory, files, release) {
  for (const [name, text] of Object.entries(files)) {
    let stamped = text;
    try {
      stamped = stampRelease(text, release);
    } catch (error) {
      fail(`changelog.d/${name}: ${error.message}`);
    }
    if (stamped !== text) {
      writeFileSync(join(directory, name), stamped);
      files[name] = stamped;
    }
  }
}

function checkCurrent(output, rendered) {
  let current = "";
  try {
    // A Windows checkout may convert line endings; the content is what must match.
    current = readFileSync(output, "utf8").replace(/\r\n/g, "\n");
  } catch {
    // A missing file is stale too.
  }
  if (current !== rendered) {
    fail("CHANGELOG.md is out of date; run `npm run changelog` in console/ and commit it");
  }
}

function main() {
  const { check, release, root } = options(process.argv.slice(2));
  const directory = join(root, "changelog.d");
  const output = join(root, "CHANGELOG.md");
  try {
    const files = readEntries(directory);
    if (release) stampAll(directory, files, release);
    const rendered = renderChangelog(parseEntries(files));
    const count = Object.keys(files).length;
    if (check) {
      checkCurrent(output, rendered);
      process.stdout.write(`changelog: CHANGELOG.md is current (${count} entries)\n`);
    } else {
      writeFileSync(output, rendered);
      process.stdout.write(`changelog: wrote CHANGELOG.md from ${count} entries\n`);
    }
  } catch (error) {
    if (error instanceof ChangelogError) fail(`changelog.d/${error.message}`);
    if (error?.code === "ENOENT") fail(`${directory} does not exist`);
    throw error;
  }
}

main();
