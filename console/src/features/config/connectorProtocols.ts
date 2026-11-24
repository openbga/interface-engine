// Which protocols a connector can speak over which kind of endpoint (docs/spec/configuration.md
// §6.4). `raw` is what a bridge carries; message channels do not offer it. A PDF report is only
// ever read, so destinations do not offer it; HL7 files are only written, so sources do not.
import type { EndpointKind, Protocol } from "@/api/types";

export type MessageProtocol = Exclude<Protocol, "raw">;

const ENDPOINT_KINDS: Record<MessageProtocol, readonly EndpointKind[]> = {
  astm: ["serial", "tcp_client", "tcp_listener"],
  au: ["serial", "tcp_client", "tcp_listener"],
  hl7_mllp: ["tcp_client", "tcp_listener"],
  hl7_file: ["folder", "sftp"],
  delimited_file: ["folder", "sftp"],
  pdf_report: ["folder", "sftp"],
};

const SOURCE_ONLY: readonly MessageProtocol[] = ["pdf_report"];
const DESTINATION_ONLY: readonly MessageProtocol[] = ["hl7_file"];

const ALL = Object.keys(ENDPOINT_KINDS) as MessageProtocol[];

/** The protocols that suit an endpoint kind and side; every one while no endpoint is chosen. */
export function protocolsFor(
  kind: EndpointKind | null,
  side: "source" | "destination" = "source",
): MessageProtocol[] {
  return ALL.filter(
    (protocol) =>
      (kind === null || ENDPOINT_KINDS[protocol].includes(kind)) &&
      !(side === "source" ? DESTINATION_ONLY : SOURCE_ONLY).includes(protocol),
  );
}
