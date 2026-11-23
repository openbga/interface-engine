// The PDF report test of the mock engine (docs/spec/pdf-reports.md §9). The mock cannot read PDF
// files: it takes a file that starts with `%PDF` (or a stored message) as a report printing each
// label of the template once on its own line, followed by a synthetic value, so the editor's test
// view can be built and checked. Any other file is unreadable. A destination gets a short HL7
// message with the report embedded, shortened as the engine shortens it.

import { isRecord } from "../ipc/util";
import type { PdfReportTestRequest, PdfReportTestResult } from "../types";
import type { MockWorld } from "./world";

interface TemplateField {
  name: string;
  label: string;
}

/** The fields of a template, each with its first label; null when it is not a template. */
function templateFields(json: string): TemplateField[] | null {
  let template: unknown;
  try {
    template = JSON.parse(json);
  } catch {
    return null;
  }
  if (!isRecord(template) || !Array.isArray(template.fields)) return null;
  const fields: TemplateField[] = [];
  for (const field of template.fields) {
    if (!isRecord(field) || typeof field.name !== "string") return null;
    const labels = Array.isArray(field.label) ? field.label : [field.label];
    if (typeof labels[0] !== "string") return null;
    fields.push({ name: field.name, label: labels[0] });
  }
  return fields;
}

function isPdf(file: string | undefined): boolean {
  try {
    return file !== undefined && atob(file).startsWith("%PDF");
  } catch {
    return false;
  }
}

const EMPTY: Omit<PdfReportTestResult, "ok"> = {
  reasonCode: null,
  reasonDetail: null,
  pages: [],
  fields: [],
  output: null,
  outputError: null,
};

export function testPdfReport(
  world: MockWorld,
  request: PdfReportTestRequest,
): PdfReportTestResult {
  const details = {
    source: request.messageId === undefined ? "file" : "message",
    destination: request.destination?.protocol ?? null,
  };
  const object =
    request.messageId === undefined
      ? {}
      : { objectType: "message" as const, objectId: request.messageId };
  world.audit.record({ action: "pdf_report_test", outcome: "requested", details, ...object });
  const fields = templateFields(request.templateJson);
  if (fields === null || fields.length === 0) {
    world.audit.record({ action: "pdf_report_test", outcome: "failed", details, ...object });
    return world.fail("validation", "The template has 1 error; nothing was read.", [
      {
        field: "templateJson",
        message: "The template must be a JSON object with 1 to 200 fields.",
      },
    ]);
  }
  const result =
    request.messageId !== undefined || isPdf(request.file)
      ? read(fields, request)
      : {
          ...EMPTY,
          ok: false,
          reasonCode: "pdf_unreadable",
          reasonDetail: "the file is not a readable PDF",
        };
  world.audit.record({
    action: "pdf_report_test",
    outcome: "succeeded",
    details: { ...details, ok: result.ok },
    ...object,
  });
  return result;
}

function read(fields: TemplateField[], request: PdfReportTestRequest): PdfReportTestResult {
  const values = fields.map((field, i) => ({
    path: `field.${field.name}`,
    value: `SYN-${String(i + 1).padStart(4, "0")}`,
  }));
  const lines = fields.map((field, i) => {
    const text = `${field.label} ${values[i]?.value ?? ""}`;
    return {
      center: 100 + 14 * i,
      phrases: [{ text, left: 40, right: 40 + 5 * text.length, size: 9 }],
    };
  });
  const output =
    request.destination === undefined
      ? null
      : "MSH|^~\\&|PACKIE|LAB|||20261012120500||ORU^R01|MOCK-0001|P|2.5.1\r" +
        `PID|||${values[0]?.value ?? ""}\r` +
        "OBX|1|ED|REPORT^Report^L||^AP^PDF^Base64^JVBERi0xLjcKJcfsj6IKNSAwIG9iago8PC9MZW5ndGggNiAwIFIvRmlsdGVyIC9GbGF0ZURlY29kZT4+CnN0cmVhbQp4nO1dW4…(1204 characters)||||||F\r";
  return {
    ...EMPTY,
    ok: true,
    pages: [{ number: 1, lines }],
    fields: [...values, { path: "report.pages", value: "1" }],
    output,
  };
}
