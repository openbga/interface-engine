---
date: 2026-11-22
type: fixed
impact: low
title: Help and guides corrected where they disagreed with the engine
area: Documentation
---
The Connection help article now gives the console's actual reconnection timing: a first retry
after about half a second, doubling to about 5 seconds. The operator guides were corrected in
the same way wherever they described something the engine does differently.

## Technical notes

Operator guide corrections: the macOS daemon's error log is
`/Library/Logs/Positive ACK/launchd.stderr.log`; the Messages text filter searches identifiers
and message type, never bodies; a truncated transmission CSV ends with a `# truncated:` line and
its totals cover the whole period; an export with `--bodies` stops with an error when an
original cannot be read; `max_frame_chars` accepts 64–64,000; an AU class A destination and an
HL7 destination with `ack_mode: none` reach `handed_off` at most. No behavior changed.
