import { describe, expect, it } from "vitest";
import { endpointFromWire, endpointToWire } from "./endpointConfig";
import type { WireEndpoint } from "./wire";

const sparseSftp = (): WireEndpoint => ({
  endpoint_id: "partner",
  label: "Partner",
  kind: "sftp",
  settings: {
    host: "sftp.example",
    username: "lab",
    auth: { method: "password", password: { secret: "pw" } },
    host_key_fingerprint: "SHA256:abcdefghij",
    remote_directory: "/in",
  },
});

describe("SFTP file name pattern", () => {
  it("reads the default of * when the document leaves it out", () => {
    expect(endpointFromWire(sparseSftp())).toMatchObject({ kind: "sftp", filePattern: "*" });
  });

  it("leaves the original document alone when the pattern is unchanged", () => {
    const raw = sparseSftp();
    expect(endpointToWire(endpointFromWire(raw), raw)).toEqual(raw);
  });

  it("writes only a changed pattern and keeps settings the console does not model", () => {
    const raw = sparseSftp();
    (raw.settings as Record<string, unknown>).stable_seconds = 9;
    const ui = endpointFromWire(raw);
    if (ui.kind !== "sftp") throw new Error("expected an SFTP endpoint");
    const written = endpointToWire({ ...ui, filePattern: "*.csv" }, raw);
    expect(written.settings).toEqual({ ...raw.settings, file_pattern: "*.csv" });
  });
});

describe("what happens to a file once it is read", () => {
  it("reads the engine's defaults: move to processed, no outcome folders", () => {
    expect(endpointFromWire(sparseSftp())).toMatchObject({
      onConsumed: "move",
      processedDirectory: "processed",
      filteredDirectory: "",
      invalidDirectory: "",
    });
  });

  it("writes outcome folders, and an emptied one as null", () => {
    const raw = sparseSftp();
    Object.assign(raw.settings, { filtered_directory: "skipped", invalid_directory: "errors" });
    const ui = endpointFromWire(raw);
    if (ui.kind !== "sftp") throw new Error("expected an SFTP endpoint");
    const written = endpointToWire({ ...ui, filteredDirectory: "" }, raw);
    expect(written.settings).toMatchObject({
      filtered_directory: null,
      invalid_directory: "errors",
    });
  });

  it("leaves the defaults out of a new endpoint", () => {
    const ui = endpointFromWire(sparseSftp());
    const written = endpointToWire({ ...ui, id: "new" }, undefined);
    expect(written.settings).not.toHaveProperty("on_consumed");
    expect(written.settings).not.toHaveProperty("filtered_directory");
  });
});
