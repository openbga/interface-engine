// Setup checks (docs/spec/diagnostics.md §10) and notifications (§6.1): read-only, nothing is
// sent to an instrument or a destination system.
import type { SerialPortKind } from "@/ipc/generated";
import type { DestinationConfig, EndpointConfig } from "./config";

export type { SerialPortKind };

/** A configured serial endpoint that names a port. */
export interface SerialPortUse {
  endpointId: string;
  endpointLabel: string;
  /** The channels that use the endpoint. */
  channelIds: string[];
  /** The channel holding the port open right now, if any. */
  heldBy: string | null;
}

/** A serial port the operating system reports, sorted by name in the list. */
export interface SerialPortInfo {
  /** `COM3`, `/dev/ttyUSB0`. */
  name: string;
  kind: SerialPortKind;
  /** USB vendor id and product id, when known. */
  vid: number | null;
  pid: number | null;
  serialNumber: string | null;
  manufacturer: string | null;
  product: string | null;
  /** The configured endpoints naming this port. */
  usedBy: SerialPortUse[];
}

/** An endpoint definition to probe: the editor's current, possibly unsaved, settings. */
export interface EndpointTestRequest {
  endpoint: EndpointConfig;
  /** The overall limit, 1 to 60 seconds; the engine's default (10) when absent. */
  timeoutSeconds?: number;
}

/** One step of a probe: what was found or why it failed (counts and causes, never file names or secrets). */
export interface ProbeStep {
  /** `resolve`, `connect`, `open`, `bind`, `read directory`, `write test`, `authenticate`, `list`… */
  name: string;
  ok: boolean;
  detail: string;
  millis: number;
}

export interface ProbeReport {
  /** Every step that ran succeeded. */
  ok: boolean;
  /** The steps that ran, in order; a step after a failed one is omitted. */
  steps: ProbeStep[];
  /** One sentence for the operator. */
  summary: string;
}

/**
 * A PDF report read with a template, and mapped for a destination when one is given
 * (docs/spec/pdf-reports.md §9): the editor's current, possibly unsaved, settings. Nothing is
 * stored or sent; the engine audits the test.
 */
export interface PdfReportTestRequest {
  /** The source's template as the editor holds it (a JSON object). */
  templateJson: string;
  /** The report: its bytes in Base64, at most 8 MiB. */
  file?: string;
  /** Or a stored PDF report message, whose original is read. */
  messageId?: string;
  /** A destination of the channel to map the report for. */
  destination?: DestinationConfig;
}

/** One phrase of an extracted line, in points from the top left of the page. */
interface PdfPhraseLayout {
  text: string;
  left: number;
  right: number;
  size: number;
}

interface PdfLineLayout {
  /** Vertical center, points from the top of the page. */
  center: number;
  phrases: PdfPhraseLayout[];
}

interface PdfPageLayout {
  number: number;
  lines: PdfLineLayout[];
}

/** What a PDF report test found. Report content: shown only in the test's own view. */
export interface PdfReportTestResult {
  /** The report reads with the template. */
  ok: boolean;
  /** Why not: `pdf_unreadable`, `pdf_no_text`, `pdf_limits`, `template_mismatch`, `field_invalid`. */
  reasonCode: string | null;
  /** What failed, in template terms. */
  reasonDetail: string | null;
  /** The extracted lines, to write or correct a template. */
  pages: PdfPageLayout[];
  /** `field.<name>` and `report.pages` with their values; empty when the report does not read. */
  fields: { path: string; value: string }[];
  /** The message mapped for the destination, the embedded report shortened. */
  output: string | null;
  /** The mapping's transform error instead. */
  outputError: string | null;
}

/** The outcome of a test message to one notification target. */
export interface NotificationResult {
  /** `email` or `webhook`. */
  target: string;
  ok: boolean;
  /** `sent`, or why not. */
  detail: string;
}

/** An SMTP relay as the bootstrap configures it (the password is only ever a secret's name). */
export interface EmailTarget {
  host: string;
  port: number;
  security: "starttls" | "tls" | "none";
  username: string | null;
  /** The secret holding the password, by name. */
  passwordSecret: string | null;
  from: string;
  to: string[];
  severities: string[];
}

/** A webhook as the bootstrap configures it. */
export interface WebhookTarget {
  url: string;
  /** The secret holding the bearer token, by name. */
  bearerSecret: string | null;
  severities: string[];
  timeoutSeconds: number;
}

/** The bootstrap's `notifications` section; targets are null when not configured. */
export interface NotificationSettings {
  rateLimitMinutes: number;
  email: EmailTarget | null;
  webhook: WebhookTarget | null;
}
