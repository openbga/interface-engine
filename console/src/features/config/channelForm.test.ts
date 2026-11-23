import { describe, expect, it } from "vitest";
import type { MessageChannelConfig } from "@/api/types";
import { channelToValues, passthroughProblem, valuesToChannel } from "./channelForm";

const REPORT_TEMPLATE = JSON.stringify({ fields: [{ name: "sample_id", label: "Lab Number:" }] });
const HL7_FILE = JSON.stringify({ file_name: "ORU_{sample_id}.hl7", sending_facility: "LAB" });

const channel: MessageChannelConfig = {
  kind: "message",
  id: "reports",
  label: "Reports",
  enabled: true,
  source: {
    id: "source",
    endpointId: "reports-in",
    protocol: "pdf_report",
    settings: { templateJson: REPORT_TEMPLATE },
    duplicateDetection: true,
  },
  filters: [],
  destinations: [
    {
      id: "hie",
      label: "HIE",
      endpointId: "hl7-out",
      protocol: "hl7_file",
      settings: { templateJson: HL7_FILE },
      enabled: true,
      filters: [],
      mappings: [],
      codeTables: {},
      retry: { initialMs: 1000, maxMs: 60_000, maxAttempts: 5 },
    },
  ],
  retention: { days: 90, maxMessages: null },
};

describe("the channel form", () => {
  it("keeps the JSON settings of a PDF report source and an HL7 file destination", () => {
    const saved = valuesToChannel(channelToValues(channel));
    if (saved?.kind !== "message") throw new Error("a message channel");
    expect(saved.source.settings).toEqual({ templateJson: REPORT_TEMPLATE });
    expect(saved.destinations[0]?.settings).toEqual({ templateJson: HL7_FILE });
  });
});

describe("a destination without a mapping", () => {
  const unmapped = { protocol: "hl7_file" as const, mappings: [], codeTables: [] };

  it("writes an HL7 source's messages to HL7 files unchanged", () => {
    expect(passthroughProblem("hl7_mllp", unmapped)).toBeNull();
  });

  it("needs a mapping to write HL7 files from any other source", () => {
    expect(passthroughProblem("pdf_report", unmapped)).toMatch(/must use the source's protocol/);
  });
});
