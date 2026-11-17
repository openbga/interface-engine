import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

// Vitest runs in console/.
const SCRIPT = join(process.cwd(), "scripts", "changelog.mjs");

const ENTRY = `---
date: 2026-11-17
type: added
impact: high
title: A synthetic change
---
What operators can do now.
`;

let root = "";

function run(...args) {
  const result = spawnSync(process.execPath, [SCRIPT, "--root", root, ...args], {
    encoding: "utf8",
  });
  return { status: result.status, out: result.stdout + result.stderr };
}

function entry(name, text = ENTRY) {
  writeFileSync(join(root, "changelog.d", name), text);
}

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "changelog-"));
  mkdirSync(join(root, "changelog.d"));
});

afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

describe("scripts/changelog.mjs", () => {
  it("writes CHANGELOG.md, and check mode passes on it", () => {
    entry("2026-11-17-synthetic.md");
    expect(run()).toMatchObject({ status: 0 });
    const written = readFileSync(join(root, "CHANGELOG.md"), "utf8");
    expect(written).toContain("- **Added:** A synthetic change (2026-11-17)");
    expect(run("--check")).toMatchObject({ status: 0 });
  });

  it("check mode fails when CHANGELOG.md is stale or missing, and leaves it alone", () => {
    entry("2026-11-17-synthetic.md");
    const missing = run("--check");
    expect(missing.status).toBe(1);
    expect(missing.out).toContain("CHANGELOG.md is out of date");
    run();
    entry("2026-11-18-another.md", ENTRY.replace("2026-11-17", "2026-11-18"));
    expect(run("--check").status).toBe(1);
    writeFileSync(join(root, "CHANGELOG.md"), "edited by hand\n");
    expect(run("--check").status).toBe(1);
    expect(readFileSync(join(root, "CHANGELOG.md"), "utf8")).toBe("edited by hand\n");
  });

  it("names the file and the key of an invalid entry", () => {
    entry("2026-11-17-bad.md", ENTRY.replace("impact: high", "impact: huge"));
    const result = run("--check");
    expect(result.status).toBe(1);
    expect(result.out).toContain('changelog.d/2026-11-17-bad.md: key "impact" must be one of');
  });

  it("stamps the release on unreleased entries only", () => {
    entry("2026-11-17-synthetic.md");
    entry("2026-11-10-older.md", ENTRY.replace("---\nWhat", "release: 0.1.0\n---\nWhat"));
    expect(run("--release", "0.2.0")).toMatchObject({ status: 0 });
    const stamped = readFileSync(join(root, "changelog.d", "2026-11-17-synthetic.md"), "utf8");
    const older = readFileSync(join(root, "changelog.d", "2026-11-10-older.md"), "utf8");
    expect(stamped).toContain("release: 0.2.0\n---");
    expect(older).toContain("release: 0.1.0\n---");
    expect(older).not.toContain("0.2.0");
    const written = readFileSync(join(root, "CHANGELOG.md"), "utf8");
    expect(written).toContain("## 0.2.0 (2026-11-17)");
    expect(written).not.toContain("## Unreleased");
    expect(run("--check").status).toBe(0);
  });

  it("refuses unknown arguments and a version that is not one", () => {
    expect(run("--fix").status).toBe(1);
    expect(run("--release", "next").out).toContain('"next" is not a version');
  });
});
