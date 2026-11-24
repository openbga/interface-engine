// Rules about protocols that both the editor and the demo engine apply
// (docs/spec/configuration.md, connectors).
import type { Protocol } from "@/api/types";

/**
 * Whether a destination of protocol `destination` delivers `source` messages unchanged, without
 * a mapping: the same protocol, or HL7 files for an HL7 MLLP source.
 */
export function passesThrough(source: Protocol, destination: Protocol): boolean {
  return destination === source || (source === "hl7_mllp" && destination === "hl7_file");
}
