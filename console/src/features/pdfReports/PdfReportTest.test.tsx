import { screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MockEngineApi } from "@/api/mock";
import type { DestinationConfig } from "@/api/types";
import { detail, summary } from "@/features/messages/fixtures";
import { MessageNotices } from "@/features/messages/MessageNotices";
import { stubApi } from "@/features/shared-operate/testFixtures";
import { expectNoA11yViolations } from "@/lib/test/a11y";
import { renderWithProviders } from "@/lib/test/render";
import { PdfReportTest } from "./PdfReportTest";

const TEMPLATE = JSON.stringify({
  fields: [
    { name: "sample_id", label: "Lab Number:" },
    { name: "dob", label: "DOB:" },
  ],
});

const HIE: DestinationConfig = {
  id: "hie",
  label: "HIE",
  endpointId: "ep-hie",
  protocol: "hl7_mllp",
  settings: {},
  enabled: true,
  filters: [],
  mappings: [],
  codeTables: {},
  retry: { initialMs: 1000, maxMs: 60_000, maxAttempts: 5 },
};

function report(text = "%PDF-1.7 synthetic report") {
  return new File([text], "report.pdf", { type: "application/pdf" });
}

function open(destinations: DestinationConfig[] = []) {
  const api = new MockEngineApi({ latency: "none" });
  const view = renderWithProviders(
    <PdfReportTest
      isOpen
      onOpenChange={() => {}}
      templateJson={TEMPLATE}
      destinations={destinations}
    />,
    { api },
  );
  return { ...view, api };
}

async function choose(user: ReturnType<typeof open>["user"], file: File) {
  const input = document.querySelector<HTMLInputElement>('input[type="file"]');
  if (input === null) throw new Error("no file input");
  await user.upload(input, file);
  await screen.findByText(`${file.name} (1 KB)`);
}

describe("Test with a PDF", () => {
  it("reads a chosen report and shows its fields and extracted lines", async () => {
    const { user, api, container } = open();
    await choose(user, report());
    await user.click(screen.getByRole("button", { name: "Read the report" }));
    expect(await screen.findByText("The report reads with the template.")).toBeTruthy();
    const fields = screen.getByRole("table", { name: "Fields read" });
    expect(within(fields).getByText("field.sample_id")).toBeTruthy();
    expect(within(fields).getByText("SYN-0001")).toBeTruthy();
    expect(screen.getByText(/\[40\] Lab Number: SYN-0001/)).toBeTruthy();
    const audit = await api.getAuditEvents({
      filter: { action: "pdf_report_test" },
      beforeSeq: null,
      limit: 10,
    });
    expect(audit.events.some((e) => e.outcome === "succeeded")).toBe(true);
    await expectNoA11yViolations(container);
  });

  it("gives the engine's reason for a file that is not a readable PDF", async () => {
    const { user } = open();
    await choose(user, report("not a pdf"));
    await user.click(screen.getByRole("button", { name: "Read the report" }));
    expect(await screen.findByText("The report does not read with the template")).toBeTruthy();
    expect(screen.getByText(/the file is not a readable PDF/)).toBeTruthy();
    expect(screen.queryByRole("table", { name: "Fields read" })).toBeNull();
  });

  it("shows the message for a chosen destination with the embedded report shortened", async () => {
    const { user } = open([HIE]);
    await choose(user, report());
    await user.click(screen.getByRole("button", { name: /Map it for/ }));
    await user.click(await screen.findByRole("option", { name: "HIE" }));
    await user.click(screen.getByRole("button", { name: "Read the report" }));
    const output = await screen.findByRole("region", { name: "Message for the destination" });
    expect(output.textContent).toContain("OBX|1|ED|");
    expect(output.textContent).toContain("characters)");
  });
});

describe("an invalid PDF report message", () => {
  it("says what failed and offers a test against the current template", async () => {
    const invalid = detail({
      summary: summary(1, { state: "invalid", protocol: "pdf_report", deliveries: [] }),
      deliveries: [],
      reasonCode: "template_mismatch",
      reasonDetail: 'label "Lab Number:" not found on page 1',
    });
    const api = stubApi({
      getConfig: async () => new MockEngineApi({ latency: "none" }).getConfig(),
    });
    renderWithProviders(<MessageNotices detail={invalid} />, { api });
    expect(screen.getByText('What failed: label "Lab Number:" not found on page 1.')).toBeTruthy();
    expect(screen.getByRole("button", { name: "Test against the current template" })).toBeTruthy();
  });
});
