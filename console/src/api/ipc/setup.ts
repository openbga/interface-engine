// Setup checks (`list_serial_ports`, `test_endpoint`, `test_notification`, `test_pdf_report`):
// the engine's reports as the console's, and a console endpoint as the engine's `EndpointTest`.
// Pure.

import type {
  PdfReportTestResult as PdfReportTestResultIpc,
  NotificationResult as RawNotification,
  SerialPort as RawPort,
  ProbeReport as RawProbe,
  Request,
} from "@/ipc/generated";
import type {
  EngineError,
  NotificationResult,
  NotificationSettings,
  PdfReportTestRequest,
  PdfReportTestResult,
  ProbeReport,
  SerialPortInfo,
} from "../types";
import { destinationAlone } from "./channelConfig";
import { issueList } from "./connectorConfig";
import { engineError } from "./errors";
import { toUiPath } from "./paths";
import { isRecord } from "./util";
import type { WireConfig } from "./wire";

export function serialPortFromIpc(raw: RawPort): SerialPortInfo {
  return {
    name: raw.name,
    kind: raw.kind,
    vid: raw.vid,
    pid: raw.pid,
    serialNumber: raw.serial_number,
    manufacturer: raw.manufacturer,
    product: raw.product,
    usedBy: raw.used_by.map((use) => ({
      endpointId: use.endpoint_id,
      endpointLabel: use.endpoint_label,
      channelIds: use.channels,
      heldBy: use.held_by,
    })),
  };
}

export function probeFromIpc(raw: RawProbe): ProbeReport {
  return {
    ok: raw.ok,
    steps: raw.steps.map(({ name, ok, detail, millis }) => ({ name, ok, detail, millis })),
    summary: raw.summary,
  };
}

export function notificationResultFromIpc(raw: RawNotification): NotificationResult {
  return { target: raw.target, ok: raw.ok, detail: raw.detail };
}

/** The `notifications` section of the bootstrap document (configuration.md §3), as far as the console shows it. */
export interface BootstrapNotifications {
  rate_limit_minutes?: number;
  email?: {
    host: string;
    port: number;
    security?: "starttls" | "tls" | "none";
    username?: string | null;
    password?: { secret: string } | null;
    from: string;
    to: string[];
    severities?: string[];
  } | null;
  webhook?: {
    url: string;
    bearer?: { secret: string } | null;
    severities?: string[];
    timeout_seconds?: number;
  } | null;
}

const DEFAULT_SEVERITIES = ["warning", "critical"];

/** The bootstrap's notification targets with their defaults filled in; null when the section is absent. */
export function notificationSettingsFrom(
  raw: BootstrapNotifications | null | undefined,
): NotificationSettings | null {
  if (raw === null || raw === undefined) return null;
  const { email, webhook } = raw;
  return {
    rateLimitMinutes: raw.rate_limit_minutes ?? 15,
    email: email
      ? {
          host: email.host,
          port: email.port,
          security: email.security ?? "starttls",
          username: email.username ?? null,
          passwordSecret: email.password?.secret ?? null,
          from: email.from,
          to: email.to,
          severities: email.severities ?? DEFAULT_SEVERITIES,
        }
      : null,
    webhook: webhook
      ? {
          url: webhook.url,
          bearerSecret: webhook.bearer?.secret ?? null,
          severities: webhook.severities ?? DEFAULT_SEVERITIES,
          timeoutSeconds: webhook.timeout_seconds ?? 10,
        }
      : null,
  };
}

/**
 * The engine names the problems of a probe's definition relative to the endpoint
 * (`settings.port_name`); this gives them the console's field paths at `endpoints[0]`, the
 * endpoint of `document`, which holds only that definition. Anything else passes through.
 */
export function atEndpointZero(error: unknown, document: WireConfig): unknown {
  if (typeof error !== "object" || error === null || !("fieldErrors" in error)) return error;
  const { fieldErrors } = error as EngineError;
  return {
    ...error,
    fieldErrors: fieldErrors.map(({ field, message }) => ({
      field: field.startsWith("endpoints[") ? field : toUiPath(`endpoints[0].${field}`, document),
      message,
    })),
  };
}

/**
 * The engine's request for a PDF report test: the template parsed from the editor's JSON, the
 * destination in the document's form. Problems converting them are raised as `validation` at
 * `templateJson` and `destination.<field>`.
 */
export function pdfTestToIpc(
  request: PdfReportTestRequest,
): Extract<Request, { type: "test_pdf_report" }> {
  const issues = issueList();
  let settings: unknown = null;
  try {
    settings = JSON.parse(request.templateJson);
  } catch {
    issues.add("templateJson", "The template is not valid JSON.");
  }
  if (issues.list.length === 0 && !isRecord(settings)) {
    issues.add("templateJson", "The template must be a JSON object.");
  }
  const destination =
    request.destination === undefined ? null : destinationAlone(request.destination, issues);
  if (issues.list.length > 0 || !isRecord(settings)) {
    throw engineError(
      "validation",
      "The template or the destination cannot be tested as they are.",
      issues.list.map(({ path, message }) => ({ field: path, message })),
    );
  }
  return {
    type: "test_pdf_report",
    settings,
    file: request.file ?? null,
    message_id: request.messageId ?? null,
    destination:
      destination === null
        ? null
        : {
            protocol: destination.protocol,
            settings: (destination.settings ?? {}) as Record<string, unknown>,
            mapping: (destination.mapping ?? null) as Record<string, unknown> | null,
          },
  };
}

export function pdfResultFromIpc(result: PdfReportTestResultIpc): PdfReportTestResult {
  return {
    ok: result.ok,
    reasonCode: result.reason_code,
    reasonDetail: result.reason_detail,
    pages: result.pages,
    fields: result.fields,
    output: result.output,
    outputError: result.output_error,
  };
}
