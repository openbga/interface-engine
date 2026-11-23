// "Test with a PDF…" under a PDF report source's template (docs/spec/pdf-reports.md §9): reads
// a chosen report with the template as the editor holds it, and maps it for one of the
// channel's destinations as they are edited, before anything is applied.
import { useState } from "react";
import { useFormContext } from "react-hook-form";
import type { DestinationConfig } from "@/api/types";
import { Button } from "@/components/Button";
import { PdfReportTest } from "@/features/pdfReports/PdfReportTest";
import { type ChannelFormValues, valuesToChannel } from "./channelForm";

export function PdfTestButton({ templateName }: { templateName: string }) {
  const { getValues } = useFormContext();
  const [test, setTest] = useState<{
    templateJson: string | null;
    destinations: DestinationConfig[];
  } | null>(null);
  function open() {
    const channel = valuesToChannel(getValues() as ChannelFormValues);
    const template = getValues(templateName) as string | undefined;
    setTest({
      templateJson: template?.trim() ? template : null,
      destinations: channel?.kind === "message" ? channel.destinations : [],
    });
  }
  return (
    <>
      <Button onPress={open}>Test with a PDF…</Button>
      {test !== null && (
        <PdfReportTest
          isOpen
          onOpenChange={(isOpen) => {
            if (!isOpen) setTest(null);
          }}
          templateJson={test.templateJson}
          destinations={test.destinations}
        />
      )}
    </>
  );
}
