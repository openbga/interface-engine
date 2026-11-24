// The PDF report test (docs/spec/pdf-reports.md §9): reads a report with a template that need
// not be saved and, for a chosen destination, shows the message it would be sent as. Nothing is
// stored or sent; the engine audits each test. What it shows is report content: it is drawn only
// in this dialog and dropped when the dialog closes or the console goes idle.
import { useState } from "react";
import { FileTrigger } from "react-aria-components";
import { useEngineMutation, useEngineQuery } from "@/api/hooks";
import type { DestinationConfig, EngineError, PdfReportTestResult } from "@/api/types";
import { Alert } from "@/components/Alert";
import { Button } from "@/components/Button";
import { Dialog } from "@/components/Dialog";
import { MiniTable } from "@/components/MiniTable";
import { Select } from "@/components/Select";
import { useOnIdle } from "@/lib/idle";
import styles from "./PdfReportTest.module.css";

/** The largest report the engine tests (it decodes at most 8 MiB). */
const MAX_FILE_BYTES = 8 * 1024 * 1024;
const NO_DESTINATION = "none";

interface Chosen {
  name: string;
  bytes: number;
  base64: string;
}

async function base64Of(file: File): Promise<string> {
  const bytes = new Uint8Array(await file.arrayBuffer());
  let text = "";
  for (let at = 0; at < bytes.length; at += 0x8000) {
    text += String.fromCharCode(...bytes.subarray(at, at + 0x8000));
  }
  return btoa(text);
}

interface PdfReportTestProps {
  isOpen: boolean;
  onOpenChange: (isOpen: boolean) => void;
  /** The template to read with (the editor's current text); null when there is none. */
  templateJson: string | null;
  /** The destinations the report can be mapped for. */
  destinations: readonly DestinationConfig[];
  /** A stored PDF report to read instead of a chosen file. */
  messageId?: string;
}

export function PdfReportTest({
  isOpen,
  onOpenChange,
  templateJson,
  destinations,
  messageId,
}: PdfReportTestProps) {
  const [chosen, setChosen] = useState<Chosen | null>(null);
  const [tooLarge, setTooLarge] = useState(false);
  const [destinationId, setDestinationId] = useState<string>(NO_DESTINATION);
  const test = useEngineMutation(
    (api, request: Parameters<typeof api.testPdfReport>[0]) => api.testPdfReport(request),
    { invalidates: ["getAuditEvents"] },
  );
  function forget() {
    setChosen(null);
    setTooLarge(false);
    test.reset();
  }
  useOnIdle(() => {
    forget();
    onOpenChange(false);
  });
  async function choose(files: FileList | null) {
    const file = files?.[0];
    test.reset();
    if (file === undefined) return;
    setTooLarge(file.size > MAX_FILE_BYTES);
    setChosen(
      file.size > MAX_FILE_BYTES
        ? null
        : { name: file.name, bytes: file.size, base64: await base64Of(file) },
    );
  }
  function run() {
    if (templateJson === null) return;
    const destination = destinations.find((d) => d.id === destinationId);
    test.mutate({
      templateJson,
      ...(messageId === undefined ? { file: chosen?.base64 } : { messageId }),
      ...(destination === undefined ? {} : { destination }),
    });
  }
  const ready = templateJson !== null && (messageId !== undefined || chosen !== null);
  return (
    <Dialog
      title={messageId === undefined ? "Test with a PDF" : "Test against the current template"}
      description="Reads the report with the template and shows what it would send. Nothing is stored or sent; the test is recorded in the audit log."
      isOpen={isOpen}
      onOpenChange={(open) => {
        if (!open) forget();
        onOpenChange(open);
      }}
      size="wide"
      actions={
        <>
          <Button onPress={() => onOpenChange(false)}>Close</Button>
          <Button variant="primary" isDisabled={!ready} isPending={test.isPending} onPress={run}>
            Read the report
          </Button>
        </>
      }
    >
      {templateJson === null && (
        <Alert tone="warning">The channel has no PDF report template to read with.</Alert>
      )}
      {messageId === undefined && (
        <div className={styles.choose}>
          <FileTrigger acceptedFileTypes={["application/pdf", ".pdf"]} onSelect={choose}>
            <Button>Choose a PDF…</Button>
          </FileTrigger>
          <span className={styles.file} aria-live="polite">
            {chosen === null
              ? "No file chosen."
              : `${chosen.name} (${Math.ceil(chosen.bytes / 1024)} KB)`}
          </span>
        </div>
      )}
      {tooLarge && (
        <Alert tone="warning">The file is larger than 8 MiB; choose a smaller one.</Alert>
      )}
      {destinations.length > 0 && (
        <Select
          name="pdf-test-destination"
          label="Map it for"
          options={[
            { id: NO_DESTINATION, label: "No destination: show the fields only" },
            ...destinations.map((d) => ({ id: d.id, label: d.label || d.id })),
          ]}
          value={destinationId}
          onChange={(value) => setDestinationId(value ?? NO_DESTINATION)}
        />
      )}
      {test.error !== null && <TestError error={test.error} />}
      {test.data !== undefined && <TestResult result={test.data} />}
    </Dialog>
  );
}

function TestError({ error }: { error: EngineError }) {
  return (
    <Alert tone="danger" title="The test did not run">
      <p>{error.message}</p>
      {error.fieldErrors.length > 0 && (
        <ul>
          {error.fieldErrors.map((f) => (
            <li key={`${f.field}|${f.message}`}>
              <code>{f.field}</code>: {f.message}
            </li>
          ))}
        </ul>
      )}
    </Alert>
  );
}

/** What the engine found: the reason, or the fields and the mapped message; and the lines. */
function TestResult({ result }: { result: PdfReportTestResult }) {
  return (
    <div className={styles.result}>
      {result.ok ? (
        <Alert tone="success">The report reads with the template.</Alert>
      ) : (
        <Alert tone="warning" title="The report does not read with the template">
          <p>
            {result.reasonDetail ?? "No detail."} (<code>{result.reasonCode}</code>)
          </p>
        </Alert>
      )}
      {result.fields.length > 0 && (
        <MiniTable
          label="Fields read"
          columns={[
            { id: "path", header: "Path", cell: (row) => <code>{row.path}</code> },
            { id: "value", header: "Value", cell: (row) => row.value },
          ]}
          rows={result.fields}
          getRowId={(row) => row.path}
        />
      )}
      {result.output !== null && (
        <section aria-label="Message for the destination">
          <h3 className={styles.heading}>Message for the destination</h3>
          <pre className={styles.text}>{result.output.replaceAll("\r", "\n")}</pre>
        </section>
      )}
      {result.outputError !== null && (
        <Alert tone="danger" title="The mapping failed">
          {result.outputError}
        </Alert>
      )}
      {result.pages.length > 0 && (
        <details className={styles.lines}>
          <summary>Extracted text, with each phrase's left edge in points</summary>
          {result.pages.map((page) => (
            <pre key={page.number} className={styles.text}>
              {`Page ${page.number}\n`}
              {page.lines
                .map((line) =>
                  line.phrases.map((p) => `[${Math.round(p.left)}] ${p.text}`).join("   "),
                )
                .join("\n")}
            </pre>
          ))}
        </details>
      )}
    </div>
  );
}

/**
 * "Test against the current template" on an invalid PDF report: the channel's active template
 * reads the stored original, so a corrected template can be checked before the report is resent
 * as a new message.
 */
export function TestStoredReport({
  messageId,
  channelId,
}: {
  messageId: string;
  channelId: string;
}) {
  const [isOpen, setOpen] = useState(false);
  const config = useEngineQuery(["getConfig"], (api) => api.getConfig());
  const channel = config.data?.channels.find((c) => c.id === channelId);
  const message = channel?.kind === "message" ? channel : undefined;
  const templateJson =
    message?.source.protocol === "pdf_report"
      ? (message.source.settings.templateJson ?? null)
      : null;
  return (
    <>
      <Button onPress={() => setOpen(true)} isDisabled={config.data === undefined}>
        Test against the current template
      </Button>
      {isOpen && (
        <PdfReportTest
          isOpen
          onOpenChange={setOpen}
          templateJson={templateJson}
          destinations={message?.destinations ?? []}
          messageId={messageId}
        />
      )}
    </>
  );
}
